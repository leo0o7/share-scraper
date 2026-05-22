use std::collections::HashMap;
use std::time::{Duration, Instant};

use scraper_utils::progress::{ProgressEvent, ProgressPhase, ScrapeErrorCategory};

use crate::operation::ScraperOperation;

#[derive(Debug)]
pub(crate) struct ProgressState {
    pub(crate) operation: ScraperOperation,
    started_at: Instant,
    completed_at: Option<Instant>,
    phases: HashMap<ProgressPhase, PhaseProgress>,
}

impl ProgressState {
    pub(crate) fn new(operation: ScraperOperation, now: Instant) -> Self {
        Self {
            operation,
            started_at: now,
            completed_at: None,
            phases: HashMap::new(),
        }
    }

    pub(crate) fn apply(&mut self, event: &ProgressEvent, now: Instant) {
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
                    if let Some(load_phase) = self.operation.metadata().load_phase {
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

    pub(crate) fn complete(&mut self, now: Instant) {
        self.completed_at = Some(now);
    }

    pub(crate) fn phase(&self, phase: ProgressPhase) -> Option<&PhaseProgress> {
        self.phases.get(&phase)
    }

    pub(crate) fn status(&self, phase: ProgressPhase) -> PhaseStatus {
        match self.phase(phase) {
            Some(progress) if progress.finished_at.is_some() => PhaseStatus::Done,
            Some(progress) if progress.started_at.is_some() => PhaseStatus::Active,
            _ => PhaseStatus::Pending,
        }
    }

    pub(crate) fn elapsed(&self, now: Instant) -> Duration {
        self.completed_at.unwrap_or(now) - self.started_at
    }

    pub(crate) fn phase_elapsed(&self, phase: ProgressPhase, now: Instant) -> Option<Duration> {
        let progress = self.phase(phase)?;
        let started_at = progress.started_at?;
        Some(progress.finished_at.unwrap_or(now) - started_at)
    }

    pub(crate) fn active_phase(&self) -> Option<ProgressPhase> {
        self.operation
            .metadata()
            .expected_phases
            .iter()
            .copied()
            .find(|phase| self.status(*phase) == PhaseStatus::Active)
    }

    pub(crate) fn current_phase(&self) -> Option<ProgressPhase> {
        self.active_phase().or_else(|| {
            self.operation
                .metadata()
                .expected_phases
                .iter()
                .copied()
                .find(|phase| self.status(*phase) == PhaseStatus::Pending)
        })
    }

    pub(crate) fn scrape_errors(&self) -> u64 {
        self.phase(self.operation.metadata().scrape_phase)
            .map_or(0, |progress| progress.errors)
    }

    pub(crate) fn save_errors(&self) -> u64 {
        self.phase(self.operation.metadata().insert_phase)
            .map_or(0, |progress| progress.errors)
    }

    pub(crate) fn failed(&self) -> bool {
        self.scrape_errors() > 0 || self.save_errors() > 0
    }

    pub(crate) fn snapshot(&self, now: Instant) -> ProgressSnapshot {
        let phases = self
            .phases
            .iter()
            .map(|(phase, progress)| {
                (
                    *phase,
                    PhaseSnapshot {
                        total: progress.total,
                        completed: progress.completed,
                        successful: progress.successful,
                        errors: progress.errors,
                        network_errors: progress.network_errors,
                        invalid_pages: progress.invalid_pages,
                        timeouts: progress.timeouts,
                        max_retries: progress.max_retries,
                        parsing_errors: progress.parsing_errors,
                        isins_found: progress.isins_found,
                        letters_completed: progress.letters_completed,
                        last: progress.last.clone(),
                        status: self.status(*phase),
                        elapsed: self.phase_elapsed(*phase, now),
                    },
                )
            })
            .collect();

        ProgressSnapshot {
            elapsed: self.elapsed(now),
            phases,
            active_phase: self.active_phase(),
            current_phase: self.current_phase(),
            scrape_errors: self.scrape_errors(),
            save_errors: self.save_errors(),
            failed: self.failed(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProgressSnapshot {
    pub(crate) elapsed: Duration,
    phases: HashMap<ProgressPhase, PhaseSnapshot>,
    pub(crate) active_phase: Option<ProgressPhase>,
    pub(crate) current_phase: Option<ProgressPhase>,
    pub(crate) scrape_errors: u64,
    pub(crate) save_errors: u64,
    pub(crate) failed: bool,
}

impl ProgressSnapshot {
    pub(crate) fn phase(&self, phase: ProgressPhase) -> Option<&PhaseSnapshot> {
        self.phases.get(&phase)
    }

    pub(crate) fn status(&self, phase: ProgressPhase) -> PhaseStatus {
        self.phase(phase)
            .map_or(PhaseStatus::Pending, |progress| progress.status)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PhaseSnapshot {
    pub(crate) total: Option<u64>,
    pub(crate) completed: u64,
    pub(crate) successful: u64,
    pub(crate) errors: u64,
    pub(crate) network_errors: u64,
    pub(crate) invalid_pages: u64,
    pub(crate) timeouts: u64,
    pub(crate) max_retries: u64,
    pub(crate) parsing_errors: u64,
    pub(crate) isins_found: u64,
    pub(crate) letters_completed: u64,
    pub(crate) last: Option<String>,
    pub(crate) status: PhaseStatus,
    pub(crate) elapsed: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PhaseProgress {
    pub(crate) total: Option<u64>,
    pub(crate) completed: u64,
    pub(crate) successful: u64,
    pub(crate) errors: u64,
    pub(crate) network_errors: u64,
    pub(crate) invalid_pages: u64,
    pub(crate) timeouts: u64,
    pub(crate) max_retries: u64,
    pub(crate) parsing_errors: u64,
    pub(crate) isins_found: u64,
    pub(crate) letters_completed: u64,
    pub(crate) last: Option<String>,
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
pub(crate) enum PhaseStatus {
    Pending,
    Active,
    Done,
}

impl PhaseStatus {
    pub(crate) fn label(self) -> &'static str {
        match self {
            PhaseStatus::Pending => "Pending",
            PhaseStatus::Active => "Active",
            PhaseStatus::Done => "Done",
        }
    }
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
    fn reducer_tracks_active_completion_and_failure_state() {
        let start = Instant::now();
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
            start,
        );
        state.apply(
            &ProgressEvent::PhaseStarted {
                phase: ProgressPhase::ScrapeShares,
                total: Some(2),
            },
            start,
        );
        assert_eq!(state.active_phase(), Some(ProgressPhase::ScrapeShares));
        assert_eq!(state.current_phase(), Some(ProgressPhase::ScrapeShares));
        assert!(!state.failed());

        state.apply(
            &ProgressEvent::ShareScraped {
                isin: "IT0000000001".to_string(),
                result: Err(ScrapeErrorCategory::InvalidPage),
            },
            start,
        );
        state.apply(
            &ProgressEvent::PhaseFinished {
                phase: ProgressPhase::ScrapeShares,
            },
            start + Duration::from_secs(2),
        );
        state.complete(start + Duration::from_secs(3));

        assert_eq!(state.active_phase(), None);
        assert_eq!(state.current_phase(), Some(ProgressPhase::InsertShares));
        assert_eq!(
            state.elapsed(start + Duration::from_secs(4)),
            Duration::from_secs(3)
        );
        assert_eq!(state.scrape_errors(), 1);
        assert_eq!(state.save_errors(), 0);
        assert!(state.failed());
    }

    #[test]
    fn snapshot_exposes_observable_progress_without_live_reducer_access() {
        let start = Instant::now();
        let scrape_done = start + Duration::from_secs(2);
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
            start,
        );
        state.apply(
            &ProgressEvent::PhaseStarted {
                phase: ProgressPhase::ScrapeShares,
                total: Some(2),
            },
            start,
        );
        state.apply(
            &ProgressEvent::ShareScraped {
                isin: "IT0000000001".to_string(),
                result: Ok(()),
            },
            start + Duration::from_secs(1),
        );
        state.apply(
            &ProgressEvent::PhaseFinished {
                phase: ProgressPhase::ScrapeShares,
            },
            scrape_done,
        );

        let snapshot = state.snapshot(scrape_done + Duration::from_secs(1));
        let scrape = snapshot.phase(ProgressPhase::ScrapeShares).unwrap();

        assert_eq!(snapshot.elapsed, Duration::from_secs(3));
        assert_eq!(snapshot.active_phase, None);
        assert_eq!(snapshot.current_phase, Some(ProgressPhase::InsertShares));
        assert_eq!(scrape.total, Some(2));
        assert_eq!(scrape.completed, 1);
        assert_eq!(scrape.successful, 1);
        assert_eq!(scrape.status, PhaseStatus::Done);
        assert_eq!(scrape.elapsed, Some(Duration::from_secs(2)));
        assert_eq!(scrape.last, Some("IT0000000001".to_string()));
    }
}
