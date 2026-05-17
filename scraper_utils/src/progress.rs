use scraper::{errors::ScrapingError, shares::ShareScrapeCompletion};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgressPhase {
    LoadShareIsins,
    ScrapeShares,
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
}

#[derive(Clone)]
pub struct ProgressSender {
    sender: mpsc::Sender<ProgressEvent>,
}

impl ProgressSender {
    pub fn new(sender: mpsc::Sender<ProgressEvent>) -> Self {
        Self { sender }
    }

    pub async fn phase_started(&self, phase: ProgressPhase, total: Option<u64>) {
        let _ = self
            .sender
            .send(ProgressEvent::PhaseStarted { phase, total })
            .await;
    }

    pub async fn phase_finished(&self, phase: ProgressPhase) {
        let _ = self
            .sender
            .send(ProgressEvent::PhaseFinished { phase })
            .await;
    }

    pub fn share_scraped(&self, completion: ShareScrapeCompletion) {
        let result = completion.result.map_err(ScrapeErrorCategory::from);
        let _ = self.sender.try_send(ProgressEvent::ShareScraped {
            isin: completion.isin,
            result,
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
