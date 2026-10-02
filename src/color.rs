use blink1rs::Color;

/// Convert a remaining-usage percentage to the indicator color.
///
/// Zero percent is red, 50 percent is yellow, and 100 percent is green.
pub fn remaining_to_color(remaining: u8) -> Color {
    let remaining = remaining.min(100);

    match remaining {
        0..=50 => Color::rgb(255, interpolate(0, 255, remaining, 50), 0),
        _ => Color::rgb(interpolate(255, 0, remaining - 50, 50), 255, 0),
    }
}

/// Expected quota remaining, clamped to the window's 0–100% range.
pub fn expected_remaining(reset_at_unix: u64, now_unix: u64, window_secs: u64) -> f64 {
    reset_at_unix.saturating_sub(now_unix).min(window_secs) as f64 / window_secs as f64 * 100.0
}

/// A deficit of 0, 7.5, or 15 percentage points is green, yellow, or red.
pub fn glidepath_to_color(remaining: u8, expected: f64) -> Color {
    let deficit = (expected - f64::from(remaining)).clamp(0.0, 15.0);
    if deficit <= 7.5 {
        Color::rgb((255.0 * deficit / 7.5).round() as u8, 255, 0)
    } else {
        Color::rgb(255, (255.0 * (15.0 - deficit) / 7.5).round() as u8, 0)
    }
}

fn interpolate(start: u8, end: u8, position: u8, length: u8) -> u8 {
    let length = i32::from(length);
    let value =
        i32::from(start) * length + (i32::from(end) - i32::from(start)) * i32::from(position);
    ((value + length / 2) / length).clamp(0, 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glidepath_anchors_and_interpolation() {
        assert_eq!(glidepath_to_color(50, 50.0), Color::GREEN);
        assert_eq!(glidepath_to_color(60, 50.0), Color::GREEN);
        assert_eq!(glidepath_to_color(50, 57.5), Color::YELLOW);
        assert_eq!(glidepath_to_color(35, 50.0), Color::RED);
        assert_eq!(glidepath_to_color(0, 50.0), Color::RED);
        assert_eq!(glidepath_to_color(50, 53.75), Color::rgb(128, 255, 0));
        assert_eq!(glidepath_to_color(50, 61.25), Color::rgb(255, 128, 0));
    }

    #[test]
    fn glidepath_time_is_clamped_for_both_windows() {
        for window in [5 * 60 * 60, 7 * 24 * 60 * 60] {
            let now = 1800000000;
            assert_eq!(expected_remaining(now + window, now, window), 100.0);
            assert_eq!(expected_remaining(now + window / 2, now, window), 50.0);
            assert_eq!(expected_remaining(now, now, window), 0.0);
            assert_eq!(expected_remaining(now - 1, now, window), 0.0);
            assert_eq!(expected_remaining(now + window * 2, now, window), 100.0);
        }
    }

    #[test]
    fn maps_anchor_values() {
        assert_eq!(remaining_to_color(0), Color::RED);
        assert_eq!(remaining_to_color(50), Color::YELLOW);
        assert_eq!(remaining_to_color(100), Color::GREEN);
    }

    #[test]
    fn interpolates_between_anchors() {
        assert_eq!(remaining_to_color(25), Color::rgb(255, 128, 0));
        assert_eq!(remaining_to_color(75), Color::rgb(128, 255, 0));
    }

    #[test]
    fn clamps_values_above_one_hundred() {
        assert_eq!(remaining_to_color(101), Color::GREEN);
        assert_eq!(remaining_to_color(u8::MAX), Color::GREEN);
    }
}
