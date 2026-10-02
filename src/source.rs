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
    #[error("the {0} limit needs reset_at_unix for --glidepath")]
    MissingReset(&'static str),
    #[error("the {0} limit has remaining_percent {1}, outside the range 0 through 100")]
    InvalidPercent(&'static str, u64),
}

#[derive(Debug, PartialEq, Eq)]
pub struct LimitUsage {
    pub remaining: u8,
    pub resets: String,
    pub reset_at_unix: Option<u64>,
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
    reset_at_unix: Option<u64>,
}

pub fn read_usage(limit: Limit, glidepath: bool) -> Result<Usage, SourceError> {
    let output = Command::new(COMMAND).output().map_err(SourceError::Start)?;
    if !output.status.success() {
        return Err(SourceError::Status(output.status));
    }

    let usage = extract_usage(&String::from_utf8_lossy(&output.stdout), limit)?;
    if glidepath {
        validate_reset_times(&usage, limit)?;
    }
    Ok(usage)
}

fn validate_reset_times(usage: &Usage, limit: Limit) -> Result<(), SourceError> {
    for (kind, selected, value) in [
        ("5h", limit != Limit::Weekly, usage.five_hour.as_ref()),
        ("weekly", limit != Limit::FiveHour, usage.weekly.as_ref()),
    ] {
        if selected && value.and_then(|usage| usage.reset_at_unix).is_none() {
            return Err(SourceError::MissingReset(kind));
        }
    }
    Ok(())
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
                    reset_at_unix: usage_limit.reset_at_unix,
                });
            }
            "weekly" if weekly.is_none() => {
                weekly = Some(LimitUsage {
                    remaining: percent("weekly", usage_limit.remaining_percent)?,
                    resets: usage_limit.resets,
                    reset_at_unix: usage_limit.reset_at_unix,
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
                    reset_at_unix: None,
                }),
                weekly: Some(LimitUsage {
                    remaining: 47,
                    resets: "13:51 on 4 Oct".to_string(),
                    reset_at_unix: None,
                }),
            }
        );
        assert_eq!(
            extract_usage(VALID, Limit::Both).unwrap(),
            Usage {
                five_hour: Some(LimitUsage {
                    remaining: 17,
                    resets: "11:44am".to_string(),
                    reset_at_unix: None,
                }),
                weekly: Some(LimitUsage {
                    remaining: 47,
                    resets: "13:51 on 4 Oct".to_string(),
                    reset_at_unix: None,
                }),
            }
        );
    }

    #[test]
    fn glidepath_requires_selected_reset_times() {
        let usage = extract_usage(VALID, Limit::Both).unwrap();
        assert!(matches!(
            validate_reset_times(&usage, Limit::Both),
            Err(SourceError::MissingReset("5h"))
        ));
        let json = r#"{"buckets":[{"limits":[{"kind":"5h","remaining_percent":50,"resets":"exact text","reset_at_unix":1800000000}]}]}"#;
        let usage = extract_usage(json, Limit::FiveHour).unwrap();
        assert_eq!(
            usage.five_hour.as_ref().unwrap().reset_at_unix,
            Some(1800000000)
        );
        assert!(validate_reset_times(&usage, Limit::FiveHour).is_ok());
        assert!(validate_reset_times(&usage, Limit::Both).is_err());
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
