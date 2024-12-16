use std::ops::Add;

use serde::Serialize;

use crate::errors::ScrapingError;

#[derive(Serialize)]
pub struct WithMetrics<T> {
    pub metrics: ScrapingMetrics,
    pub result: Option<T>,
}

impl<T> WithMetrics<T> {
    pub fn unmetric(&mut self) -> T {
        self.result.take().unwrap()
    }
    pub fn new(result: T, metrics: ScrapingMetrics) -> Self {
        WithMetrics {
            result: Some(result),
            metrics,
        }
    }
}
#[derive(Serialize, Debug)]
pub struct ScrapingMetrics {
    pub total: i32,
    pub successful: i32,
    pub errors: ScrapingErrorMetrics,
}

impl Add for ScrapingMetrics {
    type Output = ScrapingMetrics;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            total: self.total + rhs.total,
            successful: self.successful + rhs.successful,
            errors: self.errors + rhs.errors,
        }
    }
}

impl ScrapingMetrics {
    pub fn empty() -> Self {
        Self {
            total: 0,
            successful: 0,
            errors: ScrapingErrorMetrics::empty(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct ScrapingErrorMetrics {
    pub network_error: i32,
    pub invalid_page: i32,
    pub timeout: i32,
    pub max_retries: i32,
    pub parsing_error: i32,
}

impl Add for ScrapingErrorMetrics {
    type Output = ScrapingErrorMetrics;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            network_error: self.network_error + rhs.network_error,
            invalid_page: self.invalid_page + rhs.invalid_page,
            timeout: self.timeout + rhs.timeout,
            max_retries: self.max_retries + rhs.max_retries,
            parsing_error: self.parsing_error + rhs.parsing_error,
        }
    }
}

impl ScrapingErrorMetrics {
    pub fn empty() -> Self {
        Self {
            network_error: 0,
            invalid_page: 0,
            timeout: 0,
            max_retries: 0,
            parsing_error: 0,
        }
    }

    pub fn update(&mut self, error: ScrapingError) {
        match error {
            ScrapingError::NetworkError => self.network_error += 1,
            ScrapingError::InvalidPage => self.invalid_page += 1,
            ScrapingError::Timeout => self.timeout += 1,
            ScrapingError::MaxRetries => self.max_retries += 1,
            ScrapingError::ParsingErr => self.parsing_error += 1,
        };
    }
}
