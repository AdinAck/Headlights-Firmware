use tiny_serde::{prelude::*, Deserialize, Serialize};
use tiny_serde_macros::{Deserialize, Serialize};

#[cfg(feature = "defmt")]
use defmt::Format;

use crate::types::{CommandID, HeadlightError, Mode, Version};

pub trait HeadlightCommand {
    const ID: CommandID;
}

// diagnostic -- should not occur in prod
#[derive(Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Enum))]
#[repr(u8)]
pub enum AppError {
    None = 0x00,
    /// app sent a packet that could not be serialized
    InvalidPacket = 0x10,
    /// command built from app data could not be sent
    SendFault = 0x11,
    /// app sent a command before the previous finished dispatching
    TooFast = 0x12,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Enum))]
#[repr(u8)]
pub enum Request {
    Status = Status::ID,
    Control = Control::ID,
    Monitor = Monitor::ID,
    Config = Config::ID,
}

impl HeadlightCommand for Request {
    const ID: CommandID = 0x10;
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Record))]
pub struct Status {
    pub mode: Mode,
    pub error: HeadlightError,
}

impl HeadlightCommand for Status {
    const ID: CommandID = 0x1f;
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Record))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Control {
    pub target: u16,
}

impl HeadlightCommand for Control {
    const ID: CommandID = 0xaa;
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Record))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Monitor {
    /// Duty cycle of regulation PWM
    pub duty: u16,
    /// Upper load current
    pub upper_current: u16,
    /// Lower load current
    pub lower_current: u16,
    /// Temperature of FETs (not load)
    pub temperature: u16,
}

impl HeadlightCommand for Monitor {
    const ID: CommandID = 0xab;
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Record))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Config {
    /// Indicates whether or not to begin regulation
    pub enabled: bool,
    /// Default control scheme before any user control
    pub startup_control: Control,
    /// The gain or sensitivity of the regulation feedback loop
    pub gain: u8,
    /// Frequency of PWM control signal for regulation
    pub pwm_freq: u16,
    /// Maximum target output current
    pub max_target_current: u16,
    /// Maximum regulation current for the load
    pub abs_max_load_current: u16,
    /// Temperature to start throttling at
    pub throttle_start: u8,
    /// Temperature to stop throttling at (overheating)
    pub throttle_stop: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            startup_control: Control { target: 0 },
            gain: 1,
            pwm_freq: 300,
            max_target_current: 50,
            abs_max_load_current: 100,
            throttle_start: 50,
            throttle_stop: 60,
        }
    }
}

impl HeadlightCommand for Config {
    const ID: CommandID = 0xac;
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Enum))]
#[repr(u8)]
pub enum Reset {
    Now = 0x10,
    Factory = 0x11,
}

impl HeadlightCommand for Reset {
    const ID: CommandID = 0xff;
}

// this need not conform to HeadlightCommand
// as it is not exchanged between the devices
// (reflected by the fact it is not present
// in either command bundle)
#[derive(Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Record))]
pub struct Properties {
    /// device version
    pub version: Version,

    /// absolute maximum current
    pub abs_max_ma: u16,
    /// absolute maximum temperature
    pub abs_max_temp: u16,

    /// min configurable PWM frequency
    pub min_pwm_freq: u16,
    /// max configurable PWM frequency
    pub max_pwm_freq: u16,

    /// max mA measurement (adc) error
    pub max_adc_error: u16,
}
