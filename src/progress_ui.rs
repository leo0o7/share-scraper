use std::io::{self, Write};
use std::time::{Duration, Instant};

use indicatif::{HumanCount, InMemoryTerm, ProgressBar, ProgressDrawTarget, ProgressStyle};
use scraper_utils::progress::{ProgressEvent, ProgressPhase};
use tabled::{
    builder::Builder,
    settings::{object::Columns, style::HorizontalLine, Alignment as TableAlignment, Style},
};
use tokio::sync::mpsc;
use tokio::time::{self, MissedTickBehavior};

use crate::operation::{OperationMetadata, ScraperOperation};
use crate::progress_state::{PhaseSnapshot, PhaseStatus, ProgressSnapshot, ProgressState};

const FRAME_INTERVAL: Duration = Duration::from_millis(100);
const TARGET_WIDTH: usize = 100;
const MAX_BAR_WIDTH: usize = 30;
const MIN_BAR_WIDTH: usize = 12;
const SPINNER_FRAMES: [&str; 11] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", " "];

pub async fn render(operation: ScraperOperation, receiver: mpsc::UnboundedReceiver<ProgressEvent>) {
    let mut renderer = TerminalRenderer::new(operation, io::stdout());
    let _ = renderer.run(receiver).await;
}

struct TerminalRenderer<W> {
    state: ProgressState,
    writer: W,
    rendered_lines: usize,
}

impl<W: Write> TerminalRenderer<W> {
    fn new(operation: ScraperOperation, writer: W) -> Self {
        Self {
            state: ProgressState::new(operation, Instant::now()),
            writer,
            rendered_lines: 0,
        }
    }

    async fn run(
        &mut self,
        mut receiver: mpsc::UnboundedReceiver<ProgressEvent>,
    ) -> io::Result<()> {
        let mut ticker = time::interval(FRAME_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        self.redraw_live(Instant::now())?;
        loop {
            tokio::select! {
                event = receiver.recv() => {
                    let Some(event) = event else {
                        break;
                    };
                    self.state.apply(&event, Instant::now());
                    self.redraw_live(Instant::now())?;
                }
                _ = ticker.tick() => {
                    self.redraw_live(Instant::now())?;
                }
            }
        }

        let now = Instant::now();
        self.state.complete(now);
        self.redraw(&render_final(
            &self.state.snapshot(now),
            self.state.operation.metadata(),
        ))
    }

    fn redraw_live(&mut self, now: Instant) -> io::Result<()> {
        self.redraw(&render_live(
            &self.state.snapshot(now),
            self.state.operation.metadata(),
        ))
    }

    fn redraw(&mut self, output: &str) -> io::Result<()> {
        if self.rendered_lines > 0 {
            write!(self.writer, "\x1b[{}F\x1b[J", self.rendered_lines)?;
        }

        writeln!(self.writer, "{output}")?;
        self.writer.flush()?;
        self.rendered_lines = output.lines().count();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alignment {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
struct Column {
    title: &'static str,
    alignment: Alignment,
}

fn render_live(snapshot: &ProgressSnapshot, metadata: OperationMetadata) -> String {
    [
        metadata.title.to_string(),
        String::new(),
        render_summary_table(snapshot, metadata),
        String::new(),
        render_phase_table(snapshot, metadata),
        String::new(),
        render_error_table(snapshot, metadata),
        String::new(),
        "Current phase".to_string(),
        render_loader(snapshot, metadata),
    ]
    .join("\n")
}

fn render_final(snapshot: &ProgressSnapshot, metadata: OperationMetadata) -> String {
    let mut sections = vec![format!(
        "{} {}",
        metadata.title,
        if snapshot.failed {
            "failed"
        } else {
            "completed"
        }
    )];
    sections.push(format!("Duration: {}", format_duration(snapshot.elapsed)));
    if let Some(reason) = failure_reason(snapshot, metadata) {
        sections.push(format!("Reason: {reason}"));
    }
    sections.push(String::new());
    sections.push(render_phase_table(snapshot, metadata));
    sections.push(String::new());
    sections.push(render_error_table(snapshot, metadata));
    sections.join("\n")
}

fn render_summary_table(snapshot: &ProgressSnapshot, metadata: OperationMetadata) -> String {
    let current_phase = snapshot
        .current_phase
        .map(|phase| metadata.phase_label(phase))
        .unwrap_or("Complete");

    render_table(
        "Summary",
        &[
            Column {
                title: "Field",
                alignment: Alignment::Left,
            },
            Column {
                title: "Value",
                alignment: Alignment::Left,
            },
        ],
        &[
            vec!["Status".to_string(), "Running".to_string()],
            vec!["Elapsed".to_string(), format_duration(snapshot.elapsed)],
            vec!["Current phase".to_string(), current_phase.to_string()],
        ],
    )
}

fn render_phase_table(snapshot: &ProgressSnapshot, metadata: OperationMetadata) -> String {
    match metadata.operation {
        ScraperOperation::ScrapeAndInsertShares => render_share_phase_table(
            snapshot,
            metadata,
            "Shares scraped",
            ProgressPhase::LoadShareIsins,
        ),
        ScraperOperation::RefreshShares => render_share_phase_table(
            snapshot,
            metadata,
            "Shares refreshed",
            ProgressPhase::LoadStaleShares,
        ),
        ScraperOperation::ScrapeAndInsertIsins => render_isin_phase_table(snapshot, metadata),
    }
}

fn render_share_phase_table(
    snapshot: &ProgressSnapshot,
    metadata: OperationMetadata,
    scrape_column: &'static str,
    load_phase: ProgressPhase,
) -> String {
    let scrape_phase = ProgressPhase::ScrapeShares;
    let save_phase = ProgressPhase::InsertShares;

    render_table(
        "Phases",
        &[
            left_column("Phase"),
            left_column("Status"),
            right_column("Shares total"),
            right_column(scrape_column),
            right_column("Shares saved"),
            right_column("Errors"),
            right_column("Elapsed"),
        ],
        &[
            vec![
                metadata.phase_label(load_phase).to_string(),
                snapshot.status(load_phase).label().to_string(),
                phase_total_or_dash(snapshot, load_phase),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                phase_elapsed_or_dash(snapshot, load_phase),
            ],
            vec![
                metadata.phase_label(scrape_phase).to_string(),
                snapshot.status(scrape_phase).label().to_string(),
                phase_total_or_dash(snapshot, scrape_phase),
                phase_success_or_dash(snapshot, scrape_phase),
                "-".to_string(),
                phase_errors_or_dash(snapshot, scrape_phase),
                phase_elapsed_or_dash(snapshot, scrape_phase),
            ],
            vec![
                metadata.phase_label(save_phase).to_string(),
                snapshot.status(save_phase).label().to_string(),
                phase_total_or_dash(snapshot, save_phase),
                "-".to_string(),
                phase_success_or_dash(snapshot, save_phase),
                phase_errors_or_dash(snapshot, save_phase),
                phase_elapsed_or_dash(snapshot, save_phase),
            ],
        ],
    )
}

fn render_isin_phase_table(snapshot: &ProgressSnapshot, metadata: OperationMetadata) -> String {
    let discover_phase = ProgressPhase::ScrapeIsins;
    let save_phase = ProgressPhase::InsertIsins;

    render_table(
        "Phases",
        &[
            left_column("Phase"),
            left_column("Status"),
            right_column("Pages scraped"),
            right_column("ISINs found"),
            right_column("ISINs saved"),
            right_column("Errors"),
            right_column("Elapsed"),
        ],
        &[
            vec![
                metadata.phase_label(discover_phase).to_string(),
                snapshot.status(discover_phase).label().to_string(),
                phase_completed_or_dash(snapshot, discover_phase),
                phase_isins_found_or_dash(snapshot, discover_phase),
                "-".to_string(),
                phase_errors_or_dash(snapshot, discover_phase),
                phase_elapsed_or_dash(snapshot, discover_phase),
            ],
            vec![
                metadata.phase_label(save_phase).to_string(),
                snapshot.status(save_phase).label().to_string(),
                "-".to_string(),
                phase_total_or_dash(snapshot, save_phase),
                phase_success_or_dash(snapshot, save_phase),
                phase_errors_or_dash(snapshot, save_phase),
                phase_elapsed_or_dash(snapshot, save_phase),
            ],
        ],
    )
}

fn render_error_table(snapshot: &ProgressSnapshot, metadata: OperationMetadata) -> String {
    let phase = metadata.scrape_phase;
    let progress = snapshot.phase(phase);

    render_table(
        "Scrape error breakdown",
        &[
            left_column("Phase"),
            right_column("Network"),
            right_column("Invalid"),
            right_column("Timeouts"),
            right_column("Max retries"),
            right_column("Parsing"),
        ],
        &[vec![
            metadata.phase_label(phase).to_string(),
            format_number(progress.map_or(0, |progress| progress.network_errors)),
            format_number(progress.map_or(0, |progress| progress.invalid_pages)),
            format_number(progress.map_or(0, |progress| progress.timeouts)),
            format_number(progress.map_or(0, |progress| progress.max_retries)),
            format_number(progress.map_or(0, |progress| progress.parsing_errors)),
        ]],
    )
}

fn render_loader(snapshot: &ProgressSnapshot, metadata: OperationMetadata) -> String {
    let Some(phase) = snapshot.active_phase else {
        return "Waiting for next phase".to_string();
    };
    let Some(progress) = snapshot.phase(phase) else {
        return "Waiting for progress events".to_string();
    };

    let label = metadata.loader_label(phase);
    let elapsed = phase_elapsed_or_dash(snapshot, phase);
    match progress.total {
        Some(total) if total > 0 => render_progress_loader(label, progress, total, &elapsed),
        _ => render_spinner_loader(label, progress, &elapsed, snapshot.elapsed),
    }
}

fn render_progress_loader(
    label: &str,
    progress: &PhaseSnapshot,
    total: u64,
    elapsed: &str,
) -> String {
    for width in (MIN_BAR_WIDTH..=MAX_BAR_WIDTH).rev() {
        let completed = progress.completed.min(total);
        let line = loader_line(
            label,
            &progress_bar(completed, total, width),
            &format!(
                "{}/{} {}% {}",
                format_number(completed),
                format_number(total),
                percent(completed, total),
                elapsed
            ),
            progress.last.as_deref(),
        );
        if char_count(&line) <= TARGET_WIDTH || width == MIN_BAR_WIDTH {
            return line;
        }
    }

    unreachable!("loader widths include a minimum width")
}

fn render_spinner_loader(
    label: &str,
    progress: &PhaseSnapshot,
    elapsed: &str,
    operation_elapsed: Duration,
) -> String {
    let spinner = spinner_frame(operation_elapsed);
    loader_line(label, &spinner, elapsed, progress.last.as_deref())
}

fn loader_line(label: &str, indicator: &str, metrics: &str, last: Option<&str>) -> String {
    let base = format!("{label}  {indicator}  {metrics}");
    let Some(last) = last else {
        return base;
    };

    let with_last = format!("{base} last: {}", truncate_chars(last, 12));
    if char_count(&with_last) <= TARGET_WIDTH {
        with_last
    } else {
        base
    }
}

fn render_table(title: &str, columns: &[Column], rows: &[Vec<String>]) -> String {
    let mut builder = Builder::default();
    builder.push_record(columns.iter().map(|column| column.title));
    for row in rows {
        builder.push_record(row.iter().cloned());
    }

    let mut table = builder.build();
    table.with(
        Style::modern()
            .remove_horizontal()
            .horizontals([(1, HorizontalLine::inherit(Style::modern()))]),
    );

    for (index, column) in columns.iter().enumerate() {
        if column.alignment == Alignment::Right {
            table.modify(Columns::one(index), TableAlignment::right());
        }
    }

    format!("{title}\n{table}")
}

fn left_column(title: &'static str) -> Column {
    Column {
        title,
        alignment: Alignment::Left,
    }
}

fn right_column(title: &'static str) -> Column {
    Column {
        title,
        alignment: Alignment::Right,
    }
}

fn phase_total_or_dash(snapshot: &ProgressSnapshot, phase: ProgressPhase) -> String {
    if snapshot.status(phase) == PhaseStatus::Pending {
        return "-".to_string();
    }

    snapshot
        .phase(phase)
        .and_then(|progress| progress.total)
        .map_or_else(|| "-".to_string(), format_number)
}

fn phase_completed_or_dash(snapshot: &ProgressSnapshot, phase: ProgressPhase) -> String {
    phase_number_or_dash(snapshot, phase, |progress| progress.completed)
}

fn phase_success_or_dash(snapshot: &ProgressSnapshot, phase: ProgressPhase) -> String {
    phase_number_or_dash(snapshot, phase, |progress| progress.successful)
}

fn phase_errors_or_dash(snapshot: &ProgressSnapshot, phase: ProgressPhase) -> String {
    phase_number_or_dash(snapshot, phase, |progress| progress.errors)
}

fn phase_isins_found_or_dash(snapshot: &ProgressSnapshot, phase: ProgressPhase) -> String {
    phase_number_or_dash(snapshot, phase, |progress| progress.isins_found)
}

fn phase_number_or_dash(
    snapshot: &ProgressSnapshot,
    phase: ProgressPhase,
    value: impl FnOnce(&PhaseSnapshot) -> u64,
) -> String {
    if snapshot.status(phase) == PhaseStatus::Pending {
        return "-".to_string();
    }

    snapshot.phase(phase).map_or_else(
        || "-".to_string(),
        |progress| format_number(value(progress)),
    )
}

fn phase_elapsed_or_dash(snapshot: &ProgressSnapshot, phase: ProgressPhase) -> String {
    snapshot
        .phase(phase)
        .and_then(|progress| progress.elapsed)
        .map_or_else(|| "-".to_string(), format_duration)
}

fn failure_reason(snapshot: &ProgressSnapshot, metadata: OperationMetadata) -> Option<String> {
    let scrape_errors = snapshot.scrape_errors;
    let save_errors = snapshot.save_errors;
    match (scrape_errors, save_errors) {
        (0, 0) => None,
        (scrape_errors, 0) => Some(format!(
            "{} {}",
            format_number(scrape_errors),
            metadata.scrape_error_label(scrape_errors)
        )),
        (0, save_errors) => Some(format!(
            "{} {}",
            format_number(save_errors),
            metadata.save_error_label(save_errors)
        )),
        (scrape_errors, save_errors) => Some(format!(
            "{} {}, {} {}",
            format_number(scrape_errors),
            metadata.scrape_error_label(scrape_errors),
            format_number(save_errors),
            metadata.save_error_label(save_errors)
        )),
    }
}

fn progress_bar(completed: u64, total: u64, width: usize) -> String {
    let length = total.max(1);
    let position = completed.min(total);
    let term = InMemoryTerm::new(1, width as u16);
    let draw_target = ProgressDrawTarget::term_like(Box::new(term.clone()));
    let template = format!("{{bar:{width}}}");
    let bar = ProgressBar::with_draw_target(Some(length), draw_target).with_style(
        ProgressStyle::with_template(&template)
            .expect("progress bar template should be valid")
            .progress_chars("█░"),
    );
    bar.set_position(position);
    bar.force_draw();

    let mut rendered = term.contents();
    if rendered.is_empty() {
        rendered = "░".repeat(width);
    }
    if position > 0 && position < total && !rendered.contains('█') {
        rendered.replace_range(.."░".len(), "▌");
    }
    rendered
}

fn spinner_frame(elapsed: Duration) -> String {
    let style = ProgressStyle::default_spinner().tick_strings(&SPINNER_FRAMES);
    let index = (elapsed.as_millis() / FRAME_INTERVAL.as_millis()) as u64;
    style.get_tick_str(index).to_string()
}

fn percent(completed: u64, total: u64) -> u64 {
    if total == 0 {
        0
    } else {
        ((completed.min(total) as u128) * 100 / (total as u128)) as u64
    }
}

fn format_number(value: u64) -> String {
    HumanCount(value).to_string()
}

fn format_duration(duration: Duration) -> String {
    if duration.as_millis() < 1_000 {
        return format!("{}ms", duration.as_millis());
    }

    let total_seconds = duration.as_secs();
    if total_seconds < 60 {
        return format!("{total_seconds:02}s");
    }

    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    if total_minutes < 60 {
        return format!("{total_minutes:02}m {seconds:02}s");
    }

    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;
    format!("{hours:02}h {minutes:02}m {seconds:02}s")
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if char_count(value) <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let keep = max_chars - 3;
    format!("{}...", value.chars().take(keep).collect::<String>())
}

fn char_count(value: &str) -> usize {
    value.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper_utils::progress::ScrapeErrorCategory;

    #[test]
    fn live_refresh_dashboard_uses_tables_and_one_line_unicode_loader() {
        let start = Instant::now();
        let active = start + Duration::from_secs(128);
        let mut state = ProgressState::new(ScraperOperation::RefreshShares, start);

        state.apply(
            &ProgressEvent::PhaseStarted {
                phase: ProgressPhase::LoadStaleShares,
                total: None,
            },
            start,
        );
        state.apply(
            &ProgressEvent::PhaseFinished {
                phase: ProgressPhase::LoadStaleShares,
            },
            start + Duration::from_millis(20),
        );
        state.apply(
            &ProgressEvent::PhaseStarted {
                phase: ProgressPhase::ScrapeShares,
                total: Some(1_318),
            },
            start,
        );
        state.apply(
            &ProgressEvent::ShareScraped {
                isin: "IT0005089476".to_string(),
                result: Ok(()),
            },
            active,
        );

        let snapshot = state.snapshot(active);
        let dashboard = render_live(&snapshot, state.operation.metadata());
        assert!(dashboard.contains("Share refresh"));
        assert!(dashboard.contains("┌"));
        assert!(dashboard.contains("Shares refreshed"));
        assert!(dashboard.contains("Scrape error breakdown"));
        assert!(dashboard.contains("│ Refresh shares"));

        let loader = dashboard
            .lines()
            .find(|line| line.starts_with("Refreshing shares"))
            .unwrap();
        assert!(loader.contains('█') || loader.contains('▌'));
        assert!(loader.contains("last: IT0005089476"));
        assert!(!loader.contains('['));
        assert!(!loader.contains(']'));
        assert!(char_count(loader) <= TARGET_WIDTH);
    }

    #[test]
    fn final_report_uses_human_duration_and_failure_reason() {
        let start = Instant::now();
        let finished = start + Duration::from_secs(128);
        let mut state = ProgressState::new(ScraperOperation::RefreshShares, start);

        state.apply(
            &ProgressEvent::PhaseStarted {
                phase: ProgressPhase::ScrapeShares,
                total: Some(1),
            },
            start,
        );
        state.apply(
            &ProgressEvent::ShareScraped {
                isin: "IT0005089476".to_string(),
                result: Err(ScrapeErrorCategory::Timeout),
            },
            finished,
        );
        state.apply(
            &ProgressEvent::PhaseFinished {
                phase: ProgressPhase::ScrapeShares,
            },
            finished,
        );
        state.complete(finished);

        let snapshot = state.snapshot(finished);
        let report = render_final(&snapshot, state.operation.metadata());
        assert!(report.contains("Share refresh failed"));
        assert!(report.contains("Duration: 02m 08s"));
        assert!(report.contains("Reason: 1 refresh error"));
        assert!(report.contains("Timeouts"));
    }

    #[test]
    fn formats_numbers_and_durations_for_terminal_output() {
        assert_eq!(format_number(1_631), "1,631");
        assert_eq!(format_duration(Duration::from_millis(420)), "420ms");
        assert_eq!(format_duration(Duration::from_secs(8)), "08s");
        assert_eq!(format_duration(Duration::from_secs(515)), "08m 35s");
        assert_eq!(format_duration(Duration::from_secs(3_729)), "01h 02m 09s");
    }

    #[test]
    fn progress_bar_uses_unicode_without_boundaries() {
        assert_eq!(progress_bar(5, 10, 10), "█████░░░░░");
        assert!(progress_bar(1, 1_318, 10).contains('▌'));
        assert!(!progress_bar(5, 10, 10).contains('['));
    }
}
