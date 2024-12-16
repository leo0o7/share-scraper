use crate::exponential_backoff::BackoffError;

pub type ScraperResult<T> = std::result::Result<T, ScrapingError>;

#[derive(Debug, Clone)]
pub enum ScrapingError {
    NetworkError,
    InvalidPage,
    Timeout,
    MaxRetries,
    ParsingErr,
}

impl From<BackoffError> for ScrapingError {
    fn from(err: BackoffError) -> ScrapingError {
        match err {
            BackoffError::MaxRetries => ScrapingError::MaxRetries,
            BackoffError::Exit => ScrapingError::NetworkError,
        }
    }
}
