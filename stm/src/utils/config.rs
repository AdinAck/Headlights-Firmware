use common::commands::{BrightnessCommand, PIDCommand, PWMCommand};
#[cfg(feature = "defmt")]
use defmt::Format;
use embassy_stm32::{
    flash::{Blocking, Error as FlashError, Flash, WRITE_SIZE},
    gpio::Output,
};
use tiny_serde::{prelude::*, Deserialize, Serialize};
use tiny_serde_macros::{Deserialize, Serialize};

use crate::{
    fmt::{error, info, trace, unwrap, warn},
    StatusPin, MAX_PWM_FREQ, MIN_PWM_FREQ,
};

const CONFIG_SECTOR: u32 = 31;
const KIBBI: u32 = 1024;

#[cfg_attr(feature = "defmt", derive(Format))]
pub enum Error {
    Flash(FlashError),
    Deserialize,
    InvalidPWMFreq,
    InvalidPIDITerm,
}

impl From<FlashError> for Error {
    fn from(value: FlashError) -> Self {
        Self::Flash(value)
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct PIDTerms {
    pub k_p: u8,
    pub k_i: u8,
    pub k_d: u8,
    pub windup_limit: u16,
    pub div: u16,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Config {
    pub enabled: bool,
    pub startup_brightness: BrightnessCommand,
    pub pid: PIDCommand,
    pub pwm: PWMCommand,
}

#[derive(Clone)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct ValidatedConfig {
    pub enabled: bool,
    pub startup_brightness: BrightnessCommand,
    pub pid: PIDTerms,
    pub pwm: PWMCommand,
}

impl Config {
    // rust isn't ready for making this const
    // https://github.com/rust-lang/rust/issues/74935
    pub fn validated(self) -> Result<ValidatedConfig, Error> {
        (MIN_PWM_FREQ..=MAX_PWM_FREQ)
            .contains(&self.pwm.freq)
            .then_some(())
            .ok_or(Error::InvalidPWMFreq)?;

        Ok(ValidatedConfig {
            enabled: self.enabled,
            startup_brightness: self.startup_brightness,
            pid: PIDTerms {
                k_p: self.pid.k_p.into(),
                k_i: self.pid.k_i.into(),
                k_d: self.pid.k_d.into(),
                windup_limit: self
                    .pwm
                    .freq
                    .checked_div(self.pid.k_i.into())
                    .ok_or(Error::InvalidPIDITerm)?,
                div: self.pwm.freq,
            },
            pwm: self.pwm,
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            startup_brightness: BrightnessCommand { brightness: 16 },
            pid: PIDCommand {
                k_p: 0,
                k_i: 1,
                k_d: 0,
            },
            pwm: PWMCommand { freq: 300 },
        }
    }
}

impl Into<Config> for ValidatedConfig {
    fn into(self) -> Config {
        Config {
            enabled: self.enabled,
            startup_brightness: self.startup_brightness,
            pid: PIDCommand {
                k_p: self.pid.k_p,
                k_i: self.pid.k_i,
                k_d: self.pid.k_d,
            },
            pwm: self.pwm,
        }
    }
}

pub fn read_config<'a>(flash: &mut Flash<'a, Blocking>) -> Result<Config, Error> {
    const CFG_SIZE: usize = <Config as _TinyDeSized>::SIZE;
    let mut buf = [0u8; CFG_SIZE];
    flash.read(CONFIG_SECTOR * KIBBI, &mut buf)?;
    trace!("Read config buffer from flash: {}.", buf);

    Config::deserialize(unwrap!(buf[..CFG_SIZE].try_into())).ok_or(Error::Deserialize)
}

pub fn write_config<'a>(config: Config, flash: &mut Flash<'a, Blocking>) -> Result<(), Error> {
    const CFG_SIZE: usize = <Config as _TinySerSized>::SIZE;
    let mut buf = [0u8; CFG_SIZE + (WRITE_SIZE - CFG_SIZE % WRITE_SIZE)];
    buf[..CFG_SIZE].copy_from_slice(&config.serialize());
    flash.blocking_erase(CONFIG_SECTOR * KIBBI, CONFIG_SECTOR * KIBBI + KIBBI - 1)?;
    flash.blocking_write(CONFIG_SECTOR * KIBBI, &buf)?;

    Ok(())
}

/// Loads the configuration from flash.
///
/// If an error occurs with flash or validation, the default configuration is returned.
///
/// # Panics
/// Panics and raises the status LED if the default configuration is invalid.
pub fn load_config<'a>(
    flash: &mut Flash<'a, Blocking>,
    status: &mut Output<'a, StatusPin>,
) -> ValidatedConfig {
    match match read_config(flash) {
        Ok(config) => match config.validated() {
            Ok(valid_config) => {
                info!("Stored configuration is valid.");
                Ok(valid_config)
            }
            Err(e) => {
                warn!(
                    "Stored configuration is invalid for reason: {}. Loading default.",
                    e
                );
                Config::default().validated()
            }
        },
        Err(e) => {
            warn!("Failed to read config from flash with error: {}.", e);
            Config::default().validated()
        }
    } {
        Ok(valid_config) => valid_config,
        Err(e) => {
            status.set_high();
            error!("Failed to load default configuration with error: {}.", e);
            panic!();
        }
    }
}
