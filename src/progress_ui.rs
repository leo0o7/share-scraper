use std::collections::HashMap;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use indicatif::{HumanCount, InMemoryTerm, ProgressBar, ProgressDrawTarget, ProgressStyle};
use scraper_utils::progress::{ProgressEvent, ProgressPhase, ScrapeErrorCategory};
use tabled::{
    builder::Builder,
    settings::{object::Columns, style::HorizontalLine, Alignment as TableAlignment, Style},
};
use tokio::sync::mpsc;
use tokio::time::{self, MissedTickBehavior};

use crate::ScraperOperation;

const FRAME_INTERVAL: Duration = Duration::from_millis(100);
const TARGET_WIDTH: usize = 100;
const MAX_BAR_WIDTH: usize = 30;
const MIN_BAR_WIDTH: usize = 12;
const SPINNER_FRAMES: [&str; 11] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", " "];

const SHARE_SCRAPE_PHASES: [ProgressPhase; 3] = [
    ProgressPhase::LoadShareIsins,
    ProgressPhase::ScrapeShares,
    ProgressPhase::InsertShares,
];
const SHARE_REFRESH_PHASES: [ProgressPhase; 3] = [
    ProgressPhase::LoadStaleShares,
    ProgressPhase::ScrapeShares,
    ProgressPhase::InsertShares,
];
const ISIN_PHASES: [ProgressPhase; 2] = [ProgressPhase::ScrapeIsins, ProgressPhase::InsertIsins];

pub async fn render(operation: ScraperOperation, receiver: mpsc::Receiver<ProgressEvent>) {
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

    async fn run(&mut self, mut receiver: mpsc::Receiver<ProgressEvent>) -> io::Result<()> {
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
        self.redraw(&render_final(&self.state, now))
    }

    fn redraw_live(&mut self, now: Instant) -> io::Result<()> {
        self.redraw(&render_live(&self.state, now))
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

#[derive(Debug)]
struct ProgressState {
    operation: ScraperOperation,
    started_at: Instant,
    completed_at: Option<Instant>,
    phases: HashMap<ProgressPhase, PhaseProgress>,
}

impl ProgressState {
    fn new(operation: ScraperOperation, now: Instant) -> Self {
        Self {
            operation,
            started_at: now,
            completed_at: None,
            phases: HashMap::new(),
        }
    }

    fn apply(&mut self, event: &ProgressEvent, now: Instant) {
        match event {
            ProgressEvent::PhaseStarted { phase, total } => {
                self.phases.insert(
                    *phase,
                    PhaseProgress {
                        total: *total,
                        started_at: Some(now),
                        ..PhaseProgress::default()
                    },
                );

                if *phase == ProgressPhase::ScrapeShares {
                    if let Some(load_phase) = load_phase_for_operation(self.operation) {
                        if let Some(load_progress) = self.phases.get_mut(&load_phase) {
                            load_progress.total = *total;
                        }
                    }
                }
            }
            ProgressEvent::PhaseFinished { phase } => {
                let progress = self.phases.entry(*phase).or_default();
                progress.started_at.get_or_insert(now);
                progress.finished_at = Some(now);
            }
            ProgressEvent::ShareScraped { isin, result } => {
                let progress = self.active_or_default(ProgressPhase::ScrapeShares, now);
                progress.completed += 1;
                progress.last = Some(isin.clone());
                match result {
                    Ok(()) => progress.successful += 1,
                    Err(category) => progress.record_scrape_error(*category),
                }
            }
            ProgressEvent::ShareInserted { isin, successful } => {
                let progress = self.active_or_default(ProgressPhase::InsertShares, now);
                progress.completed += 1;
                progress.last = Some(isin.clone());
                if *successful {
                    progress.successful += 1;
                } else {
                    progress.errors += 1;
                }
            }
            ProgressEvent::IsinPageScraped {
                letter,
                page,
                isins_found,
                result,
                parsing_errors,
            } => {
                let progress = self.active_or_default(ProgressPhase::ScrapeIsins, now);
                progress.completed += 1;
                progress.successful += isins_found;
                progress.isins_found += isins_found;
                progress.parsing_errors += parsing_errors;
                progress.errors += parsing_errors;
                progress.last = Some(format!("{letter} page {page}"));
                if let Err(category) = result {
                    progress.record_scrape_error(*category);
                }
            }
            ProgressEvent::IsinLetterCompleted { letter: _ } => {
                let progress = self.active_or_default(ProgressPhase::ScrapeIsins, now);
                progress.letters_completed += 1;
            }
            ProgressEvent::IsinInserted { isin, successful } => {
                let progress = self.active_or_default(ProgressPhase::InsertIsins, now);
                progress.completed += 1;
                progress.last = Some(isin.clone());
                if *successful {
                    progress.successful += 1;
                } else {
                    progress.errors += 1;
                }
            }
        }
    }

    fn active_or_default(&mut self, phase: ProgressPhase, now: Instant) -> &mut PhaseProgress {
        let progress = self.phases.entry(phase).or_default();
        progress.started_at.get_or_insert(now);
        progress
    }

    fn complete(&mut self, now: Instant) {
        self.completed_at = Some(now);
    }

    fn phase(&self, phase: ProgressPhase) -> Option<&PhaseProgress> {
        self.phases.get(&phase)
    }

    fn status(&self, phase: ProgressPhase) -> PhaseStatus {
        match self.phase(phase) {
            Some(progress) if progress.finished_at.is_some() => PhaseStatus::Done,
            Some(progress) if progress.started_at.is_some() => PhaseStatus::Active,
            _ => PhaseStatus::Pending,
        }
    }

    fn elapsed(&self, now: Instant) -> Duration {
        self.completed_at.unwrap_or(now) - self.started_at
    }

    fn phase_elapsed(&self, phase: ProgressPhase, now: Instant) -> Option<Duration> {
        let progress = self.phase(phase)?;
        let started_at = progress.started_at?;
        Some(progress.finished_at.unwrap_or(now) - started_at)
    }

    fn active_phase(&self) -> Option<ProgressPhase> {
        expected_phases(self.operation)
            .iter()
            .copied()
            .find(|phase| self.status(*phase) == PhaseStatus::Active)
    }

    fn current_phase(&self) -> Option<ProgressPhase> {
        self.active_phase().or_else(|| {
            expected_phases(self.operation)
                .iter()
                .copied()
                .find(|phase| self.status(*phase) == PhaseStatus::Pending)
        })
    }

    fn scrape_errors(&self) -> u64 {
        scrape_phase(self.operation)
            .and_then(|phase| self.phase(phase))
            .map_or(0, |progress| progress.errors)
    }

    fn save_errors(&self) -> u64 {
        insert_phase(self.operation)
            .and_then(|phase| self.phase(phase))
            .map_or(0, |progress| progress.errors)
    }

    fn failed(&self) -> bool {
        self.scrape_errors() > 0 || self.save_errors() > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PhaseProgress {
    total: Option<u64>,
    completed: u64,
    successful: u64,
    errors: u64,
    network_errors: u64,
    invalid_pages: u64,
    timeouts: u64,
    max_retries: u64,
    parsing_errors: u64,
    isins_found: u64,
    letters_completed: u64,
    last: Option<String>,
    started_at: Option<Instant>,
    finished_at: Option<Instant>,
}

impl PhaseProgress {
    fn record_scrape_error(&mut self, category: ScrapeErrorCategory) {
        self.errors += 1;
        match category {
            ScrapeErrorCategory::NetworkError => self.network_errors += 1,
            ScrapeErrorCategory::InvalidPage => self.invalid_pages += 1,
            ScrapeErrorCategory::Timeout => self.timeouts += 1,
            ScrapeErrorCategory::MaxRetries => self.max_retries += 1,
            ScrapeErrorCategory::ParsingError => self.parsing_errors += 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhaseStatus {
    Pending,
    Active,
    Done,
}

impl PhaseStatus {
    fn label(self) -> &'static str {
        match self {
            PhaseStatus::Pending => "Pending",
            PhaseStatus::Active => "Active",
            PhaseStatus::Done => "Done",
        }
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

fn render_live(state: &ProgressState, now: Instant) -> String {
    [
        operation_title(state.operation).to_string(),
        String::new(),
        render_summary_table(state, now),
        String::new(),
        render_phase_table(state, now),
        String::new(),
        render_error_table(state),
        String::new(),
        "Current phase".to_string(),
        render_loader(state, now),
    ]
    .join("\n")
}

fn render_final(state: &ProgressState, now: Instant) -> String {
    let mut sections = vec![format!(
        "{} {}",
        operation_title(state.operation),
        if state.failed() {
            "failed"
        } else {
            "completed"
        }
    )];
    sections.push(format!("Duration: {}", format_duration(state.elapsed(now))));
    if let Some(reason) = failure_reason(state) {
        sections.push(format!("Reason: {reason}"));
    }
    sections.push(String::new());
    sections.push(render_phase_table(state, now));
    sections.push(String::new());
    sections.push(render_error_table(state));
    sections.join("\n")
}

fn render_summary_table(state: &ProgressState, now: Instant) -> String {
    let current_phase = state
        .current_phase()
        .map(|phase| phase_label(state.operation, phase))
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
            vec!["Elapsed".to_string(), format_duration(state.elapsed(now))],
            vec!["Current phase".to_string(), current_phase.to_string()],
        ],
    )
}

fn render_phase_table(state: &ProgressState, now: Instant) -> String {
    match state.operation {
        ScraperOperation::ScrapeAndInsertShares => {
            render_share_phase_table(state, now, "Shares scraped", ProgressPhase::LoadShareIsins)
        }
        ScraperOperation::RefreshShares => render_share_phase_table(
            state,
            now,
            "Shares refreshed",
            ProgressPhase::LoadStaleShares,
        ),
        ScraperOperation::ScrapeAndInsertIsins => render_isin_phase_table(state, now),
    }
}

fn render_share_phase_table(
    state: &ProgressState,
    now: Instant,
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
                phase_label(state.operation, load_phase).to_string(),
                state.status(load_phase).label().to_string(),
                phase_total_or_dash(state, load_phase),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                phase_elapsed_or_dash(state, load_phase, now),
            ],
            vec![
                phase_label(state.operation, scrape_phase).to_string(),
                state.status(scrape_phase).label().to_string(),
                phase_total_or_dash(state, scrape_phase),
                phase_success_or_dash(state, scrape_phase),
                "-".to_string(),
                phase_errors_or_dash(state, scrape_phase),
                phase_elapsed_or_dash(state, scrape_phase, now),
            ],
            vec![
                phase_label(state.operation, save_phase).to_string(),
                state.status(save_phase).label().to_string(),
                phase_total_or_dash(state, save_phase),
                "-".to_string(),
                phase_success_or_dash(state, save_phase),
                phase_errors_or_dash(state, save_phase),
                phase_elapsed_or_dash(state, save_phase, now),
            ],
        ],
    )
}

fn render_isin_phase_table(state: &ProgressState, now: Instant) -> String {
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
                phase_label(state.operation, discover_phase).to_string(),
                state.status(discover_phase).label().to_string(),
                phase_completed_or_dash(state, discover_phase),
                phase_isins_found_or_dash(state, discover_phase),
                "-".to_string(),
                phase_errors_or_dash(state, discover_phase),
                phase_elapsed_or_dash(state, discover_phase, now),
            ],
            vec![
                phase_label(state.operation, save_phase).to_string(),
                state.status(save_phase).label().to_string(),
                "-".to_string(),
                phase_total_or_dash(state, save_phase),
                phase_success_or_dash(state, save_phase),
                phase_errors_or_dash(state, save_phase),
                phase_elapsed_or_dash(state, save_phase, now),
            ],
        ],
    )
}

fn render_error_table(state: &ProgressState) -> String {
    let phase = scrape_phase(state.operation);
    let progress = phase.and_then(|phase| state.phase(phase));

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
            phase
                .map_or("-", |phase| phase_label(state.operation, phase))
                .to_string(),
            format_number(progress.map_or(0, |progress| progress.network_errors)),
            format_number(progress.map_or(0, |progress| progress.invalid_pages)),
            format_number(progress.map_or(0, |progress| progress.timeouts)),
            format_number(progress.map_or(0, |progress| progress.max_retries)),
            format_number(progress.map_or(0, |progress| progress.parsing_errors)),
        ]],
    )
}

fn render_loader(state: &ProgressState, now: Instant) -> String {
    let Some(phase) = state.active_phase() else {
        return "Waiting for next phase".to_string();
    };
    let Some(progress) = state.phase(phase) else {
        return "Waiting for progress events".to_string();
    };

    let label = loader_label(state.operation, phase);
    let elapsed = phase_elapsed_or_dash(state, phase, now);
    match progress.total {
        Some(total) if total > 0 => render_progress_loader(label, progress, total, &elapsed),
        _ => render_spinner_loader(label, progress, &elapsed, state.elapsed(now)),
    }
}

fn render_progress_loader(
    label: &str,
    progress: &PhaseProgress,
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
    progress: &PhaseProgress,
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

fn phase_total_or_dash(state: &ProgressState, phase: ProgressPhase) -> String {
    if state.status(phase) == PhaseStatus::Pending {
        return "-".to_string();
    }

    state
        .phase(phase)
        .and_then(|progress| progress.total)
        .map_or_else(|| "-".to_string(), format_number)
}

fn phase_completed_or_dash(state: &ProgressState, phase: ProgressPhase) -> String {
    phase_number_or_dash(state, phase, |progress| progress.completed)
}

fn phase_success_or_dash(state: &ProgressState, phase: ProgressPhase) -> String {
    phase_number_or_dash(state, phase, |progress| progress.successful)
}

fn phase_errors_or_dash(state: &ProgressState, phase: ProgressPhase) -> String {
    phase_number_or_dash(state, phase, |progress| progress.errors)
}

fn phase_isins_found_or_dash(state: &ProgressState, phase: ProgressPhase) -> String {
    phase_number_or_dash(state, phase, |progress| progress.isins_found)
}

fn phase_number_or_dash(
    state: &ProgressState,
    phase: ProgressPhase,
    value: impl FnOnce(&PhaseProgress) -> u64,
) -> String {
    if state.status(phase) == PhaseStatus::Pending {
        return "-".to_string();
    }

    state.phase(phase).map_or_else(
        || "-".to_string(),
        |progress| format_number(value(progress)),
    )
}

fn phase_elapsed_or_dash(state: &ProgressState, phase: ProgressPhase, now: Instant) -> String {
    state
        .phase_elapsed(phase, now)
        .map_or_else(|| "-".to_string(), format_duration)
}

fn failure_reason(state: &ProgressState) -> Option<String> {
    let scrape_errors = state.scrape_errors();
    let save_errors = state.save_errors();
    match (scrape_errors, save_errors) {
        (0, 0) => None,
        (scrape_errors, 0) => Some(format!(
            "{} {}",
            format_number(scrape_errors),
            scrape_error_label(state.operation, scrape_errors)
        )),
        (0, save_errors) => Some(format!(
            "{} {}",
            format_number(save_errors),
            pluralized(save_errors, "save error", "save errors")
        )),
        (scrape_errors, save_errors) => Some(format!(
            "{} {}, {} {}",
            format_number(scrape_errors),
            scrape_error_label(state.operation, scrape_errors),
            format_number(save_errors),
            pluralized(save_errors, "save error", "save errors")
        )),
    }
}

fn scrape_error_label(operation: ScraperOperation, count: u64) -> &'static str {
    match operation {
        ScraperOperation::ScrapeAndInsertShares => {
            pluralized(count, "scrape error", "scrape errors")
        }
        ScraperOperation::RefreshShares => pluralized(count, "refresh error", "refresh errors"),
        ScraperOperation::ScrapeAndInsertIsins => {
            pluralized(count, "discovery error", "discovery errors")
        }
    }
}

fn pluralized(count: u64, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 {
        singular
    } else {
        plural
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

fn operation_title(operation: ScraperOperation) -> &'static str {
    match operation {
        ScraperOperation::ScrapeAndInsertShares => "Share scrape",
        ScraperOperation::ScrapeAndInsertIsins => "ISIN discovery",
        ScraperOperation::RefreshShares => "Share refresh",
    }
}

fn phase_label(operation: ScraperOperation, phase: ProgressPhase) -> &'static str {
    match (operation, phase) {
        (ScraperOperation::ScrapeAndInsertShares, ProgressPhase::LoadShareIsins) => {
            "Load share ISINs"
        }
        (ScraperOperation::ScrapeAndInsertShares, ProgressPhase::ScrapeShares) => "Scrape shares",
        (ScraperOperation::RefreshShares, ProgressPhase::LoadStaleShares) => "Load stale shares",
        (ScraperOperation::RefreshShares, ProgressPhase::ScrapeShares) => "Refresh shares",
        (_, ProgressPhase::InsertShares) => "Save shares",
        (_, ProgressPhase::ScrapeIsins) => "Discover ISINs",
        (_, ProgressPhase::InsertIsins) => "Save ISINs",
        (_, ProgressPhase::LoadShareIsins) => "Load share ISINs",
        (_, ProgressPhase::LoadStaleShares) => "Load stale shares",
        (_, ProgressPhase::ScrapeShares) => "Scrape shares",
    }
}

fn loader_label(operation: ScraperOperation, phase: ProgressPhase) -> &'static str {
    match (operation, phase) {
        (ScraperOperation::ScrapeAndInsertShares, ProgressPhase::LoadShareIsins) => {
            "Loading share ISINs"
        }
        (ScraperOperation::ScrapeAndInsertShares, ProgressPhase::ScrapeShares) => "Scraping shares",
        (ScraperOperation::RefreshShares, ProgressPhase::LoadStaleShares) => "Loading stale shares",
        (ScraperOperation::RefreshShares, ProgressPhase::ScrapeShares) => "Refreshing shares",
        (_, ProgressPhase::InsertShares) => "Saving shares",
        (_, ProgressPhase::ScrapeIsins) => "Discovering ISINs",
        (_, ProgressPhase::InsertIsins) => "Saving ISINs",
        (_, ProgressPhase::LoadShareIsins) => "Loading share ISINs",
        (_, ProgressPhase::LoadStaleShares) => "Loading stale shares",
        (_, ProgressPhase::ScrapeShares) => "Scraping shares",
    }
}

fn expected_phases(operation: ScraperOperation) -> &'static [ProgressPhase] {
    match operation {
        ScraperOperation::ScrapeAndInsertShares => &SHARE_SCRAPE_PHASES,
        ScraperOperation::RefreshShares => &SHARE_REFRESH_PHASES,
        ScraperOperation::ScrapeAndInsertIsins => &ISIN_PHASES,
    }
}

fn load_phase_for_operation(operation: ScraperOperation) -> Option<ProgressPhase> {
    match operation {
        ScraperOperation::ScrapeAndInsertShares => Some(ProgressPhase::LoadShareIsins),
        ScraperOperation::RefreshShares => Some(ProgressPhase::LoadStaleShares),
        ScraperOperation::ScrapeAndInsertIsins => None,
    }
}

fn scrape_phase(operation: ScraperOperation) -> Option<ProgressPhase> {
    match operation {
        ScraperOperation::ScrapeAndInsertShares | ScraperOperation::RefreshShares => {
            Some(ProgressPhase::ScrapeShares)
        }
        ScraperOperation::ScrapeAndInsertIsins => Some(ProgressPhase::ScrapeIsins),
    }
}

fn insert_phase(operation: ScraperOperation) -> Option<ProgressPhase> {
    match operation {
        ScraperOperation::ScrapeAndInsertShares | ScraperOperation::RefreshShares => {
            Some(ProgressPhase::InsertShares)
        }
        ScraperOperation::ScrapeAndInsertIsins => Some(ProgressPhase::InsertIsins),
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

    #[test]
    fn share_progress_counts_successes_failures_and_last_completed_share() {
        let now = Instant::now();
        let mut state = ProgressState::new(ScraperOperation::ScrapeAndInsertShares, now);

        state.apply(
            &ProgressEvent::PhaseStarted {
                phase: ProgressPhase::ScrapeShares,
                total: Some(3),
            },
            now,
        );
        state.apply(
            &ProgressEvent::ShareScraped {
                isin: "IT0000000001".to_string(),
                result: Ok(()),
            },
            now,
        );
        state.apply(
            &ProgressEvent::ShareScraped {
                isin: "IT0000000002".to_string(),
                result: Err(ScrapeErrorCategory::Timeout),
            },
            now,
        );
        state.apply(
            &ProgressEvent::ShareScraped {
                isin: "IT0000000003".to_string(),
                result: Err(ScrapeErrorCategory::ParsingError),
            },
            now,
        );
        state.apply(
            &ProgressEvent::PhaseFinished {
                phase: ProgressPhase::ScrapeShares,
            },
            now,
        );

        let phase = state.phase(ProgressPhase::ScrapeShares).unwrap();
        assert_eq!(phase.total, Some(3));
        assert_eq!(phase.completed, 3);
        assert_eq!(phase.successful, 1);
        assert_eq!(phase.errors, 2);
        assert_eq!(phase.timeouts, 1);
        assert_eq!(phase.parsing_errors, 1);
        assert_eq!(phase.last, Some("IT0000000003".to_string()));
        assert_eq!(state.status(ProgressPhase::ScrapeShares), PhaseStatus::Done);
    }

    #[test]
    fn isin_discovery_counts_pages_letters_found_isins_and_errors() {
        let now = Instant::now();
        let mut state = ProgressState::new(ScraperOperation::ScrapeAndInsertIsins, now);

        state.apply(
            &ProgressEvent::PhaseStarted {
                phase: ProgressPhase::ScrapeIsins,
                total: None,
            },
            now,
        );
        state.apply(
            &ProgressEvent::IsinPageScraped {
                letter: 'A',
                page: 1,
                isins_found: 2,
                result: Ok(()),
                parsing_errors: 1,
            },
            now,
        );
        state.apply(
            &ProgressEvent::IsinPageScraped {
                letter: 'B',
                page: 1,
                isins_found: 0,
                result: Err(ScrapeErrorCategory::NetworkError),
                parsing_errors: 0,
            },
            now,
        );
        state.apply(&ProgressEvent::IsinLetterCompleted { letter: 'A' }, now);
        state.apply(
            &ProgressEvent::PhaseFinished {
                phase: ProgressPhase::ScrapeIsins,
            },
            now,
        );

        let phase = state.phase(ProgressPhase::ScrapeIsins).unwrap();
        assert_eq!(phase.completed, 2);
        assert_eq!(phase.successful, 2);
        assert_eq!(phase.errors, 2);
        assert_eq!(phase.network_errors, 1);
        assert_eq!(phase.parsing_errors, 1);
        assert_eq!(phase.isins_found, 2);
        assert_eq!(phase.letters_completed, 1);
        assert_eq!(phase.last, Some("B page 1".to_string()));
        assert_eq!(state.status(ProgressPhase::ScrapeIsins), PhaseStatus::Done);
    }

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

        let dashboard = render_live(&state, active);
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

        let report = render_final(&state, finished);
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
