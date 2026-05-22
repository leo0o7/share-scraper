use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgressPhase {
    LoadShareIsins,
    LoadStaleShares,
    ScrapeShares,
    InsertShares,
    ScrapeIsins,
    InsertIsins,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrapeErrorCategory {
    NetworkError,
    InvalidPage,
    Timeout,
    MaxRetries,
    ParsingError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEvent {
    PhaseStarted {
        phase: ProgressPhase,
        total: Option<u64>,
    },
    PhaseFinished {
        phase: ProgressPhase,
    },
    ShareScraped {
        isin: String,
        result: Result<(), ScrapeErrorCategory>,
    },
    ShareInserted {
        isin: String,
        successful: bool,
    },
    IsinPageScraped {
        letter: char,
        page: u8,
        isins_found: u64,
        result: Result<(), ScrapeErrorCategory>,
        parsing_errors: u64,
    },
    IsinLetterCompleted {
        letter: char,
    },
    IsinInserted {
        isin: String,
        successful: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareScrapeProgress {
    pub isin: String,
    pub result: Result<(), ScrapeErrorCategory>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareInsertProgress {
    pub isin: String,
    pub successful: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsinPageScrapeProgress {
    pub letter: char,
    pub page: u8,
    pub isins_found: u64,
    pub result: Result<(), ScrapeErrorCategory>,
    pub parsing_errors: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsinInsertProgress {
    pub isin: String,
    pub successful: bool,
}

#[derive(Clone)]
pub struct ProgressSender {
    sender: mpsc::UnboundedSender<ProgressEvent>,
}

impl ProgressSender {
    pub fn new(sender: mpsc::UnboundedSender<ProgressEvent>) -> Self {
        Self { sender }
    }

    pub async fn phase_started(&self, phase: ProgressPhase, total: Option<u64>) {
        let _ = self
            .sender
            .send(ProgressEvent::PhaseStarted { phase, total });
    }

    pub async fn phase_finished(&self, phase: ProgressPhase) {
        let _ = self.sender.send(ProgressEvent::PhaseFinished { phase });
    }

    pub fn share_scraped(&self, completion: ShareScrapeProgress) {
        let _ = self.sender.send(ProgressEvent::ShareScraped {
            isin: completion.isin,
            result: completion.result,
        });
    }

    pub fn share_inserted(&self, completion: ShareInsertProgress) {
        let _ = self.sender.send(ProgressEvent::ShareInserted {
            isin: completion.isin,
            successful: completion.successful,
        });
    }

    pub fn isin_page_scraped(&self, completion: IsinPageScrapeProgress) {
        let _ = self.sender.send(ProgressEvent::IsinPageScraped {
            letter: completion.letter,
            page: completion.page,
            isins_found: completion.isins_found,
            result: completion.result,
            parsing_errors: completion.parsing_errors,
        });
    }

    pub fn isin_letter_completed(&self, letter: char) {
        let _ = self
            .sender
            .send(ProgressEvent::IsinLetterCompleted { letter });
    }

    pub fn isin_inserted(&self, completion: IsinInsertProgress) {
        let _ = self.sender.send(ProgressEvent::IsinInserted {
            isin: completion.isin,
            successful: completion.successful,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_sender_preserves_bursty_share_events() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let progress = ProgressSender::new(sender);

        for index in 0..1_000 {
            progress.share_scraped(ShareScrapeProgress {
                isin: format!("IT{index:010}"),
                result: Err(ScrapeErrorCategory::Timeout),
            });
        }
        drop(progress);

        let mut received = 0;
        while receiver.try_recv().is_ok() {
            received += 1;
        }

        assert_eq!(received, 1_000);
    }

    #[tokio::test]
    async fn progress_sender_tolerates_closed_channels_for_lifecycle_and_updates() {
        let (sender, receiver) = mpsc::unbounded_channel();
        drop(receiver);
        let progress = ProgressSender::new(sender);

        progress
            .phase_started(ProgressPhase::ScrapeShares, Some(1))
            .await;
        progress.phase_finished(ProgressPhase::ScrapeShares).await;
        progress.share_scraped(ShareScrapeProgress {
            isin: "IT0000000001".to_string(),
            result: Err(ScrapeErrorCategory::Timeout),
        });
        progress.share_inserted(ShareInsertProgress {
            isin: "IT0000000001".to_string(),
            successful: false,
        });
        progress.isin_page_scraped(IsinPageScrapeProgress {
            letter: 'A',
            page: 1,
            isins_found: 0,
            result: Ok(()),
            parsing_errors: 0,
        });
        progress.isin_letter_completed('A');
        progress.isin_inserted(IsinInsertProgress {
            isin: "IT0000000001".to_string(),
            successful: true,
        });
    }
}
