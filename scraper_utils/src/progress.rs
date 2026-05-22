use db::{isins::IsinInsertCompletion, shares::ShareInsertCompletion};
use scraper::{errors::ScrapingError, isins::IsinScrapeCompletion, shares::ShareScrapeCompletion};
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

    pub fn share_scraped(&self, completion: ShareScrapeCompletion) {
        let result = completion.result.map_err(ScrapeErrorCategory::from);
        let _ = self.sender.send(ProgressEvent::ShareScraped {
            isin: completion.isin,
            result,
        });
    }

    pub fn share_inserted(&self, completion: ShareInsertCompletion) {
        let _ = self.sender.send(ProgressEvent::ShareInserted {
            isin: completion.isin,
            successful: completion.successful,
        });
    }

    pub fn isin_page_scraped(&self, completion: IsinScrapeCompletion) {
        let result = completion.result.map_err(ScrapeErrorCategory::from);
        let _ = self.sender.send(ProgressEvent::IsinPageScraped {
            letter: completion.letter,
            page: completion.page,
            isins_found: completion.isins_found,
            result,
            parsing_errors: completion.parsing_errors,
        });
    }

    pub fn isin_letter_completed(&self, letter: char) {
        let _ = self
            .sender
            .send(ProgressEvent::IsinLetterCompleted { letter });
    }

    pub fn isin_inserted(&self, completion: IsinInsertCompletion) {
        let _ = self.sender.send(ProgressEvent::IsinInserted {
            isin: completion.isin,
            successful: completion.successful,
        });
    }
}

impl From<ScrapingError> for ScrapeErrorCategory {
    fn from(value: ScrapingError) -> Self {
        match value {
            ScrapingError::NetworkError => ScrapeErrorCategory::NetworkError,
            ScrapingError::InvalidPage => ScrapeErrorCategory::InvalidPage,
            ScrapingError::Timeout => ScrapeErrorCategory::Timeout,
            ScrapingError::MaxRetries => ScrapeErrorCategory::MaxRetries,
            ScrapingError::ParsingErr => ScrapeErrorCategory::ParsingError,
        }
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
            progress.share_scraped(ShareScrapeCompletion {
                isin: format!("IT{index:010}"),
                result: Err(ScrapingError::Timeout),
            });
        }
        drop(progress);

        let mut received = 0;
        while receiver.try_recv().is_ok() {
            received += 1;
        }

        assert_eq!(received, 1_000);
    }
}
