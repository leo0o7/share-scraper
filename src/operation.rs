use scraper_utils::progress::ProgressPhase;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScraperOperation {
    ScrapeAndInsertShares,
    ScrapeAndInsertIsins,
    RefreshShares,
}

impl ScraperOperation {
    pub(crate) fn from_cli_name(value: &str) -> Option<Self> {
        match value {
            "scrape-shares" => Some(Self::ScrapeAndInsertShares),
            "scrape-isins" => Some(Self::ScrapeAndInsertIsins),
            "refresh-shares" => Some(Self::RefreshShares),
            _ => None,
        }
    }

    pub(crate) fn metadata(self) -> OperationMetadata {
        match self {
            Self::ScrapeAndInsertShares => OperationMetadata {
                operation: self,
                cli_name: "scrape-shares",
                title: "Share scrape",
                expected_phases: &SHARE_SCRAPE_PHASES,
                load_phase: Some(ProgressPhase::LoadShareIsins),
                scrape_phase: ProgressPhase::ScrapeShares,
                insert_phase: ProgressPhase::InsertShares,
            },
            Self::ScrapeAndInsertIsins => OperationMetadata {
                operation: self,
                cli_name: "scrape-isins",
                title: "ISIN discovery",
                expected_phases: &ISIN_PHASES,
                load_phase: None,
                scrape_phase: ProgressPhase::ScrapeIsins,
                insert_phase: ProgressPhase::InsertIsins,
            },
            Self::RefreshShares => OperationMetadata {
                operation: self,
                cli_name: "refresh-shares",
                title: "Share refresh",
                expected_phases: &SHARE_REFRESH_PHASES,
                load_phase: Some(ProgressPhase::LoadStaleShares),
                scrape_phase: ProgressPhase::ScrapeShares,
                insert_phase: ProgressPhase::InsertShares,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OperationMetadata {
    operation: ScraperOperation,
    pub(crate) cli_name: &'static str,
    pub(crate) title: &'static str,
    pub(crate) expected_phases: &'static [ProgressPhase],
    pub(crate) load_phase: Option<ProgressPhase>,
    pub(crate) scrape_phase: ProgressPhase,
    pub(crate) insert_phase: ProgressPhase,
}

impl OperationMetadata {
    pub(crate) fn phase_label(self, phase: ProgressPhase) -> &'static str {
        match (self.operation, phase) {
            (ScraperOperation::ScrapeAndInsertShares, ProgressPhase::LoadShareIsins) => {
                "Load share ISINs"
            }
            (ScraperOperation::ScrapeAndInsertShares, ProgressPhase::ScrapeShares) => {
                "Scrape shares"
            }
            (ScraperOperation::RefreshShares, ProgressPhase::LoadStaleShares) => {
                "Load stale shares"
            }
            (ScraperOperation::RefreshShares, ProgressPhase::ScrapeShares) => "Refresh shares",
            (_, ProgressPhase::InsertShares) => "Save shares",
            (_, ProgressPhase::ScrapeIsins) => "Discover ISINs",
            (_, ProgressPhase::InsertIsins) => "Save ISINs",
            (_, ProgressPhase::LoadShareIsins) => "Load share ISINs",
            (_, ProgressPhase::LoadStaleShares) => "Load stale shares",
            (_, ProgressPhase::ScrapeShares) => "Scrape shares",
        }
    }

    pub(crate) fn loader_label(self, phase: ProgressPhase) -> &'static str {
        match (self.operation, phase) {
            (ScraperOperation::ScrapeAndInsertShares, ProgressPhase::LoadShareIsins) => {
                "Loading share ISINs"
            }
            (ScraperOperation::ScrapeAndInsertShares, ProgressPhase::ScrapeShares) => {
                "Scraping shares"
            }
            (ScraperOperation::RefreshShares, ProgressPhase::LoadStaleShares) => {
                "Loading stale shares"
            }
            (ScraperOperation::RefreshShares, ProgressPhase::ScrapeShares) => "Refreshing shares",
            (_, ProgressPhase::InsertShares) => "Saving shares",
            (_, ProgressPhase::ScrapeIsins) => "Discovering ISINs",
            (_, ProgressPhase::InsertIsins) => "Saving ISINs",
            (_, ProgressPhase::LoadShareIsins) => "Loading share ISINs",
            (_, ProgressPhase::LoadStaleShares) => "Loading stale shares",
            (_, ProgressPhase::ScrapeShares) => "Scraping shares",
        }
    }

    pub(crate) fn scrape_error_label(self, count: u64) -> &'static str {
        match self.operation {
            ScraperOperation::ScrapeAndInsertShares => {
                pluralized(count, "scrape error", "scrape errors")
            }
            ScraperOperation::RefreshShares => pluralized(count, "refresh error", "refresh errors"),
            ScraperOperation::ScrapeAndInsertIsins => {
                pluralized(count, "discovery error", "discovery errors")
            }
        }
    }

    pub(crate) fn save_error_label(self, count: u64) -> &'static str {
        pluralized(count, "save error", "save errors")
    }
}

fn pluralized(count: u64, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_scrape_metadata_describes_phase_relationships_and_labels() {
        let metadata = ScraperOperation::ScrapeAndInsertShares.metadata();

        assert_eq!(metadata.cli_name, "scrape-shares");
        assert_eq!(metadata.title, "Share scrape");
        assert_eq!(
            metadata.expected_phases,
            &[
                ProgressPhase::LoadShareIsins,
                ProgressPhase::ScrapeShares,
                ProgressPhase::InsertShares,
            ]
        );
        assert_eq!(metadata.load_phase, Some(ProgressPhase::LoadShareIsins));
        assert_eq!(metadata.scrape_phase, ProgressPhase::ScrapeShares);
        assert_eq!(metadata.insert_phase, ProgressPhase::InsertShares);
        assert_eq!(
            metadata.phase_label(ProgressPhase::ScrapeShares),
            "Scrape shares"
        );
        assert_eq!(
            metadata.loader_label(ProgressPhase::ScrapeShares),
            "Scraping shares"
        );
        assert_eq!(metadata.scrape_error_label(2), "scrape errors");
        assert_eq!(metadata.save_error_label(1), "save error");
    }

    #[test]
    fn isin_metadata_describes_phases_and_failure_wording() {
        let metadata = ScraperOperation::ScrapeAndInsertIsins.metadata();

        assert_eq!(metadata.cli_name, "scrape-isins");
        assert_eq!(metadata.title, "ISIN discovery");
        assert_eq!(
            metadata.expected_phases,
            &[ProgressPhase::ScrapeIsins, ProgressPhase::InsertIsins]
        );
        assert_eq!(metadata.load_phase, None);
        assert_eq!(metadata.scrape_phase, ProgressPhase::ScrapeIsins);
        assert_eq!(metadata.insert_phase, ProgressPhase::InsertIsins);
        assert_eq!(
            metadata.phase_label(ProgressPhase::ScrapeIsins),
            "Discover ISINs"
        );
        assert_eq!(
            metadata.loader_label(ProgressPhase::ScrapeIsins),
            "Discovering ISINs"
        );
        assert_eq!(metadata.scrape_error_label(1), "discovery error");
    }

    #[test]
    fn refresh_metadata_describes_load_scrape_insert_relationships() {
        let metadata = ScraperOperation::RefreshShares.metadata();

        assert_eq!(metadata.cli_name, "refresh-shares");
        assert_eq!(metadata.title, "Share refresh");
        assert_eq!(
            metadata.expected_phases,
            &[
                ProgressPhase::LoadStaleShares,
                ProgressPhase::ScrapeShares,
                ProgressPhase::InsertShares,
            ]
        );
        assert_eq!(metadata.load_phase, Some(ProgressPhase::LoadStaleShares));
        assert_eq!(metadata.scrape_phase, ProgressPhase::ScrapeShares);
        assert_eq!(metadata.insert_phase, ProgressPhase::InsertShares);
        assert_eq!(
            metadata.phase_label(ProgressPhase::ScrapeShares),
            "Refresh shares"
        );
        assert_eq!(
            metadata.loader_label(ProgressPhase::ScrapeShares),
            "Refreshing shares"
        );
        assert_eq!(metadata.scrape_error_label(2), "refresh errors");
    }

    #[test]
    fn cli_names_parse_to_stable_operations() {
        assert_eq!(
            ScraperOperation::from_cli_name("scrape-shares"),
            Some(ScraperOperation::ScrapeAndInsertShares)
        );
        assert_eq!(
            ScraperOperation::from_cli_name("scrape-isins"),
            Some(ScraperOperation::ScrapeAndInsertIsins)
        );
        assert_eq!(
            ScraperOperation::from_cli_name("refresh-shares"),
            Some(ScraperOperation::RefreshShares)
        );
        assert_eq!(ScraperOperation::from_cli_name("unknown"), None);
    }
}
