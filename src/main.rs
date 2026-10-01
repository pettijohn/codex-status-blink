mod cli;
mod color;
mod light;
mod source;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use clap::Parser;

use crate::cli::{Cli, Limit};
use crate::color::remaining_to_color;
use crate::light::Light;
use crate::source::{Usage, read_usage};

const MAX_POLL_DELAY: Duration = Duration::from_secs(5 * 60);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let stop = Arc::new(AtomicBool::new(false));
    let signal_stop = Arc::clone(&stop);
    ctrlc::set_handler(move || signal_stop.store(true, Ordering::SeqCst))?;

    let mut light: Option<Light> = None;
    let initial_delay = Duration::from_secs(cli.interval_secs);
    let mut delay = initial_delay;
    let mut previous_usage = None;

    while !stop.load(Ordering::SeqCst) {
        if light.is_none() {
            verbose(cli.verbose, "opening blink(1)");
            match Light::open(cli.blink_serial.as_deref(), cli.blink_index, cli.fade_ms) {
                Ok(opened_light) => {
                    verbose(
                        cli.verbose,
                        format!("blink(1) opened: {}", opened_light.description()),
                    );
                    light = Some(opened_light);
                }
                Err(error) => {
                    eprintln!("cannot open blink(1): {error}");
                    wait(&stop, initial_delay);
                    continue;
                }
            }
        }

        match read_usage(cli.limit) {
            Ok(usage) => {
                if let Err(error) = render(
                    light.as_mut().expect("the light is open"),
                    cli.limit,
                    &usage,
                ) {
                    eprintln!("cannot update blink(1): {error}");
                    if matches!(error, light::LightError::PerLedUnsupported { .. }) {
                        return Err(error.into());
                    }
                    light = None;
                    wait(&stop, initial_delay);
                    continue;
                }

                let displayed = displayed_values(cli.limit, &usage);
                verbose(
                    cli.verbose,
                    format!(
                        "usage remaining: 5h {}, weekly {}",
                        format_limit_usage(usage.five_hour.as_ref()),
                        format_limit_usage(usage.weekly.as_ref()),
                    ),
                );
                delay = if previous_usage == Some(displayed) {
                    next_poll_delay(delay)
                } else {
                    initial_delay
                };
                previous_usage = Some(displayed);
            }
            Err(error) => {
                eprintln!("cannot read Codex status: {error}");
                delay = initial_delay;
            }
        }

        wait(&stop, delay);
    }

    if let Some(mut light) = light
        && let Err(error) = light.shutdown()
    {
        eprintln!("cannot turn off blink(1): {error}");
    }

    Ok(())
}

fn render(light: &mut Light, limit: Limit, usage: &Usage) -> Result<(), light::LightError> {
    match limit {
        Limit::FiveHour => light.show(remaining_to_color(
            usage
                .five_hour
                .as_ref()
                .expect("source checked 5h")
                .remaining,
        )),
        Limit::Weekly => light.show(remaining_to_color(
            usage
                .weekly
                .as_ref()
                .expect("source checked weekly")
                .remaining,
        )),
        Limit::Both => light.show_both(
            remaining_to_color(
                usage
                    .five_hour
                    .as_ref()
                    .expect("source checked 5h")
                    .remaining,
            ),
            remaining_to_color(
                usage
                    .weekly
                    .as_ref()
                    .expect("source checked weekly")
                    .remaining,
            ),
        ),
    }
}

fn displayed_values(limit: Limit, usage: &Usage) -> [Option<u8>; 2] {
    match limit {
        Limit::FiveHour => [usage.five_hour.as_ref().map(|usage| usage.remaining), None],
        Limit::Weekly => [usage.weekly.as_ref().map(|usage| usage.remaining), None],
        Limit::Both => [
            usage.five_hour.as_ref().map(|usage| usage.remaining),
            usage.weekly.as_ref().map(|usage| usage.remaining),
        ],
    }
}

fn format_limit_usage(usage: Option<&source::LimitUsage>) -> String {
    usage.map_or_else(
        || "unavailable".to_string(),
        |usage| format!("{}% resets {}", usage.remaining, usage.resets),
    )
}

fn next_poll_delay(current: Duration) -> Duration {
    if current < Duration::from_secs(60) {
        Duration::from_secs(60)
    } else if current < Duration::from_secs(120) {
        Duration::from_secs(120)
    } else {
        MAX_POLL_DELAY
    }
}

fn wait(stop: &AtomicBool, delay: Duration) {
    let step = Duration::from_millis(100);
    let mut remaining = delay;

    while remaining > Duration::ZERO && !stop.load(Ordering::SeqCst) {
        let sleep_for = remaining.min(step);
        std::thread::sleep(sleep_for);
        remaining = remaining.saturating_sub(sleep_for);
    }
}

fn verbose(enabled: bool, message: impl std::fmt::Display) {
    if enabled {
        eprintln!("verbose: {message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backs_off_from_thirty_seconds_to_five_minutes() {
        assert_eq!(
            next_poll_delay(Duration::from_secs(30)),
            Duration::from_secs(60)
        );
        assert_eq!(
            next_poll_delay(Duration::from_secs(60)),
            Duration::from_secs(120)
        );
        assert_eq!(
            next_poll_delay(Duration::from_secs(120)),
            Duration::from_secs(300)
        );
        assert_eq!(
            next_poll_delay(Duration::from_secs(240)),
            Duration::from_secs(300)
        );
        assert_eq!(
            next_poll_delay(Duration::from_secs(300)),
            Duration::from_secs(300)
        );
    }

    #[test]
    fn only_a_displayed_limit_resets_the_backoff() {
        let usage = Usage {
            five_hour: Some(source::LimitUsage {
                remaining: 17,
                resets: "11:44am".to_string(),
            }),
            weekly: Some(source::LimitUsage {
                remaining: 47,
                resets: "13:51 on 4 Oct".to_string(),
            }),
        };
        let changed_weekly = Usage {
            five_hour: Some(source::LimitUsage {
                remaining: 17,
                resets: "11:44am".to_string(),
            }),
            weekly: Some(source::LimitUsage {
                remaining: 46,
                resets: "13:51 on 4 Oct".to_string(),
            }),
        };
        assert_eq!(
            displayed_values(Limit::FiveHour, &usage),
            displayed_values(Limit::FiveHour, &changed_weekly)
        );
        assert_ne!(
            displayed_values(Limit::Both, &usage),
            displayed_values(Limit::Both, &changed_weekly)
        );
    }
}
