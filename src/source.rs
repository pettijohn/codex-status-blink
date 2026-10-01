use std::process::Command;

use serde::Deserialize;
use thiserror::Error;

use crate::cli::Limit;

const COMMAND: &str = "codex-status-json";

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("cannot start {COMMAND}: {0}")]
    Start(#[source] std::io::Error),
    #[error("{COMMAND} failed with status {0}")]
    Status(std::process::ExitStatus),
    #[error("{COMMAND} wrote invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{COMMAND} returned no {0} limit")]
    MissingLimit(&'static str),
    #[error("the {0} limit has remaining_percent {1}, outside the range 0 through 100")]
    InvalidPercent(&'static str, u64),
}

#[derive(Debug, PartialEq, Eq)]
pub struct LimitUsage {
    pub remaining: u8,
    pub resets: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Usage {
    pub five_hour: Option<LimitUsage>,
    pub weekly: Option<LimitUsage>,
}

#[derive(Deserialize)]
struct Status {
    buckets: Vec<Bucket>,
}

#[derive(Deserialize)]
struct Bucket {
    limits: Vec<UsageLimit>,
}

#[derive(Deserialize)]
struct UsageLimit {
    kind: String,
    remaining_percent: u64,
    resets: String,
}

pub fn read_usage(limit: Limit) -> Result<Usage, SourceError> {
    let output = Command::new(COMMAND).output().map_err(SourceError::Start)?;
    if !output.status.success() {
        return Err(SourceError::Status(output.status));
    }

    extract_usage(&String::from_utf8_lossy(&output.stdout), limit)
}

pub fn extract_usage(json: &str, selection: Limit) -> Result<Usage, SourceError> {
    let status: Status = serde_json::from_str(json)?;
    let mut five_hour = None;
    let mut weekly = None;

    for usage_limit in status.buckets.into_iter().flat_map(|bucket| bucket.limits) {
        match usage_limit.kind.as_str() {
            "5h" if five_hour.is_none() => {
                five_hour = Some(LimitUsage {
                    remaining: percent("5h", usage_limit.remaining_percent)?,
                    resets: usage_limit.resets,
                });
            }
            "weekly" if weekly.is_none() => {
                weekly = Some(LimitUsage {
                    remaining: percent("weekly", usage_limit.remaining_percent)?,
                    resets: usage_limit.resets,
                });
            }
            _ => {}
        }
    }

    match selection {
        Limit::FiveHour if five_hour.is_none() => Err(SourceError::MissingLimit("5h")),
        Limit::Weekly if weekly.is_none() => Err(SourceError::MissingLimit("weekly")),
        Limit::Both if five_hour.is_none() => Err(SourceError::MissingLimit("5h")),
        Limit::Both if weekly.is_none() => Err(SourceError::MissingLimit("weekly")),
        _ => Ok(Usage { five_hour, weekly }),
    }
}

fn percent(kind: &'static str, value: u64) -> Result<u8, SourceError> {
    value
        .try_into()
        .map_err(|_| SourceError::InvalidPercent(kind, value))
        .and_then(|value: u8| {
            if value <= 100 {
                Ok(value)
            } else {
                Err(SourceError::InvalidPercent(kind, u64::from(value)))
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{
        "buckets": [{"limits": [
            {"kind": "5h", "remaining_percent": 17, "resets": "11:44am"},
            {"kind": "weekly", "remaining_percent": 47, "resets": "13:51 on 4 Oct"}
        ]}]
    }"#;

    #[test]
    fn extracts_requested_limits() {
        assert_eq!(
            extract_usage(VALID, Limit::FiveHour).unwrap(),
            Usage {
                five_hour: Some(LimitUsage {
                    remaining: 17,
                    resets: "11:44am".to_string(),
                }),
                weekly: Some(LimitUsage {
                    remaining: 47,
                    resets: "13:51 on 4 Oct".to_string(),
                }),
            }
        );
        assert_eq!(
            extract_usage(VALID, Limit::Both).unwrap(),
            Usage {
                five_hour: Some(LimitUsage {
                    remaining: 17,
                    resets: "11:44am".to_string(),
                }),
                weekly: Some(LimitUsage {
                    remaining: 47,
                    resets: "13:51 on 4 Oct".to_string(),
                }),
            }
        );
    }

    #[test]
    fn rejects_missing_or_invalid_limits() {
        assert!(matches!(
            extract_usage(r#"{"buckets":[]}"#, Limit::Weekly),
            Err(SourceError::MissingLimit("weekly"))
        ));
        assert!(matches!(
            extract_usage(
                r#"{"buckets":[{"limits":[{"kind":"5h","remaining_percent":101,"resets":"soon"}]}]}"#,
                Limit::FiveHour
            ),
            Err(SourceError::InvalidPercent("5h", 101))
        ));
    }
}
