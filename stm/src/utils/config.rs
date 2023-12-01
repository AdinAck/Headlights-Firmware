use common::types::{Config, ConfigError, Error as HeadlightError, RuntimeError};
#[cfg(feature = "defmt")]
use defmt::Format;
use embassy_stm32::{
    flash::{Blocking, Error as FlashError, Flash, WRITE_SIZE},
    gpio::Output,
};
use tiny_serde::{prelude::*, Deserialize, Serialize};

use crate::{
    fmt::{error, info, trace, unwrap, warn},
    limits::{ABS_MAX_MA, ABS_MAX_TEMP, MAX_PWM_FREQ, MIN_PWM_FREQ},
    FaultLEDPin,
};

use super::thermistor::celsius_to_sample;

const CONFIG_SECTOR: u32 = 31;
const KIBBI: u32 = 1024;

#[cfg_attr(feature = "defmt", derive(Format))]
pub enum Error {
    Flash(FlashError),
    Deserialize,
    Validation(ConfigError),
}

impl From<FlashError> for Error {
    fn from(value: FlashError) -> Self {
        Self::Flash(value)
    }
}

impl From<ConfigError> for Error {
    fn from(value: ConfigError) -> Self {
        Self::Validation(value)
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct ValidatedConfig {
    inner: Config,
}

impl TryFrom<Config> for ValidatedConfig {
    type Error = ConfigError;
    fn try_from(config: Config) -> Result<Self, Self::Error> {
        (MIN_PWM_FREQ..=MAX_PWM_FREQ)
            .contains(&config.pwm_freq)
            .then_some(())
            .ok_or(ConfigError::PWMFreq)?;

        (config.abs_max_load_current < ABS_MAX_MA)
            .then_some(())
            .ok_or(ConfigError::MaxTarget)?;

        (config.startup_control.target <= config.abs_max_load_current)
            .then_some(())
            .ok_or(ConfigError::StartupTarget)?;

        (config.gain >= 1).then_some(()).ok_or(ConfigError::Gain)?;

        (config.throttle_start < config.throttle_stop
            && celsius_to_sample(config.throttle_stop) < ABS_MAX_TEMP)
            .then_some(())
            .ok_or(ConfigError::ThrottleBounds)?;

        Ok(Self { inner: config })
    }
}

impl ValidatedConfig {
    pub fn inner(self) -> Config {
        self.inner
    }
}

pub struct Configurator<'a> {
    flash: Flash<'a, Blocking>,
}

impl<'a> Configurator<'a> {
    pub const fn new(flash: Flash<'a, Blocking>) -> Self {
        Self { flash }
    }

    fn read_config(&mut self) -> Result<Config, Error> {
        const CFG_SIZE: usize = <Config as _TinyDeSized>::SIZE;
        let mut buf = [0u8; CFG_SIZE];
        self.flash.read(CONFIG_SECTOR * KIBBI, &mut buf)?;
        trace!("Read config buffer from flash: {}.", buf);

        Config::deserialize(unwrap!(buf[..CFG_SIZE].try_into())).ok_or(Error::Deserialize)
    }

    pub fn write_config(&mut self, config: ValidatedConfig) -> Result<(), Error> {
        const CFG_SIZE: usize = <Config as _TinySerSized>::SIZE;
        let mut buf = [0u8; CFG_SIZE + (WRITE_SIZE - CFG_SIZE % WRITE_SIZE)];
        buf[..CFG_SIZE].copy_from_slice(&config.inner().serialize());
        self.flash
            .blocking_erase(CONFIG_SECTOR * KIBBI, CONFIG_SECTOR * KIBBI + KIBBI)?;
        self.flash.blocking_write(CONFIG_SECTOR * KIBBI, &buf)?;

        Ok(())
    }

    /// Loads the configuration from flash.
    ///
    /// If an error occurs with flash or validation, the default configuration is returned.
    ///
    /// # Panics
    /// Panics and raises the status LED if the default configuration is invalid.
    pub fn load_config<'b>(
        &mut self,
        fault: &mut Output<'b, FaultLEDPin>,
    ) -> (ValidatedConfig, Option<HeadlightError>) {
        let mut error = None;

        let maybe_config = match self.read_config() {
            Ok(config) => match config.try_into() {
                Ok(valid_config) => {
                    info!("Stored configuration is valid.");
                    Ok(valid_config)
                }
                Err(e) => {
                    warn!(
                        "Stored configuration is invalid for reason: {}. Loading default.",
                        e
                    );

                    error = Some(HeadlightError::Config(e));

                    Config::default().try_into()
                }
            },
            Err(e) => {
                warn!("Failed to read config from flash with error: {}.", e);
                error = Some(RuntimeError::Flash.into());
                Config::default().try_into()
            }
        };

        (
            // by this point if the config is not present,
            // the default must have been invalid
            match maybe_config {
                Ok(valid_config) => valid_config,
                Err(e) => {
                    fault.set_high();
                    error!("Failed to load default configuration with error: {}.", e);
                    panic!();
                }
            },
            error,
        )
    }
}
