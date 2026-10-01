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
