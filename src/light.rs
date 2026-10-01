use std::time::Duration;

use blink1rs::{Blink1, Color, DeviceKind, Error, Led};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum LightError {
    #[error(transparent)]
    Blink(#[from] Error),
    #[error("--limit both needs a blink(1) mk2 or later; this device is {kind}")]
    PerLedUnsupported { kind: DeviceKind },
}

fn supports_both(kind: DeviceKind) -> bool {
    matches!(kind, DeviceKind::Mk2 | DeviceKind::Mk3 | DeviceKind::Mk4)
}

pub struct Light {
    device: Blink1,
    fade: Duration,
    firmware: u16,
    previous: [Option<Color>; 2],
}

impl Light {
    pub fn open(serial: Option<&str>, index: Option<usize>, fade_ms: u64) -> Result<Self, Error> {
        let mut device = match (serial, index) {
            (Some(serial), None) => Blink1::open_serial(serial)?,
            (None, Some(index)) => Blink1::open_index(index)?,
            (None, None) => Blink1::open()?,
            (Some(_), Some(_)) => unreachable!("clap rejects conflicting blink(1) selectors"),
        };

        // A stored pattern can overwrite immediate color commands.
        device.stop()?;
        let firmware = device.firmware_version()?;

        Ok(Self {
            device,
            fade: Duration::from_millis(fade_ms),
            firmware,
            previous: [None, None],
        })
    }

    pub fn description(&self) -> String {
        format!(
            "serial {}, kind {}, firmware {}",
            self.device.serial(),
            self.device.kind(),
            self.firmware,
        )
    }

    pub fn show(&mut self, color: Color) -> Result<(), LightError> {
        if self.previous != [Some(color), Some(color)] {
            self.device.fade(color, self.fade)?;
            self.previous = [Some(color), Some(color)];
        }
        Ok(())
    }

    pub fn show_both(&mut self, first: Color, second: Color) -> Result<(), LightError> {
        let kind = self.device.kind();
        if !supports_both(kind) {
            return Err(LightError::PerLedUnsupported { kind });
        }

        for (index, color) in [first, second].into_iter().enumerate() {
            if self.previous[index] != Some(color) {
                self.device
                    .fade_led(color, self.fade, Led::N((index + 1) as u8))?;
                self.previous[index] = Some(color);
            }
        }
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), Error> {
        self.device.off()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_depends_on_hardware_not_firmware() {
        assert!(supports_both(DeviceKind::Mk2));
        assert!(supports_both(DeviceKind::Mk3));
        assert!(supports_both(DeviceKind::Mk4));
        assert!(!supports_both(DeviceKind::Mk1));
        assert!(!supports_both(DeviceKind::Unknown));
    }
}
