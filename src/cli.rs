use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Limit {
    /// Show the five-hour limit.
    #[value(name = "5h")]
    FiveHour,
    /// Show the weekly limit.
    Weekly,
    /// Show the five-hour limit on LED 1 and the weekly limit on LED 2.
    Both,
}

#[derive(Debug, Parser)]
#[command(about = "Show Codex usage remaining on a blink(1) LED")]
pub struct Cli {
    /// Select the blink(1) by serial number.
    #[arg(long, conflicts_with = "blink_index")]
    pub blink_serial: Option<String>,

    /// Select the blink(1) by its Blink1::list() index.
    #[arg(long, conflicts_with = "blink_serial")]
    pub blink_index: Option<usize>,

    /// Select the Codex limit to display: 5h, weekly, or both.
    #[arg(long, value_enum, default_value_t = Limit::FiveHour)]
    pub limit: Limit,

    /// Color usage relative to the straight-line quota glidepath.
    #[arg(long)]
    pub glidepath: bool,

    /// Set the initial poll interval in seconds.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..))]
    pub interval_secs: u64,

    /// Set the blink(1) fade time in milliseconds.
    #[arg(long, default_value_t = 200)]
    pub fade_ms: u64,

    /// Show command, usage, and blink(1) status messages.
    #[arg(short, long)]
    pub verbose: bool,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn defaults_to_the_five_hour_limit_and_thirty_second_interval() {
        let cli = Cli::try_parse_from(["codex-status-blink"]).unwrap();
        assert_eq!(cli.limit, Limit::FiveHour);
        assert_eq!(cli.interval_secs, 30);
    }

    #[test]
    fn parses_glidepath() {
        let cli =
            Cli::try_parse_from(["codex-status-blink", "--glidepath", "--limit", "both"]).unwrap();
        assert!(cli.glidepath);
        assert_eq!(cli.limit, Limit::Both);
        assert!(
            !Cli::try_parse_from(["codex-status-blink"])
                .unwrap()
                .glidepath
        );
    }

    #[test]
    fn parses_weekly_and_both_limits() {
        assert_eq!(
            Cli::try_parse_from(["codex-status-blink", "--limit", "weekly"])
                .unwrap()
                .limit,
            Limit::Weekly
        );
        assert_eq!(
            Cli::try_parse_from(["codex-status-blink", "--limit", "both"])
                .unwrap()
                .limit,
            Limit::Both
        );
    }
}
