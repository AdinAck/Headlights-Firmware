use tiny_serde::{prelude::*, Deserialize, Serialize};
use tiny_serde_macros::{Deserialize, Serialize};

#[cfg(feature = "defmt")]
use defmt::Format;

pub type CRCRepr = u8;
pub type CommandID = u8;

#[derive(Serialize, Deserialize)]
pub struct CommandHeader {
    pub id: CommandID,
    pub crc: CRCRepr,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Enum))]
#[cfg_attr(feature = "defmt", derive(Format))]
#[repr(u8)]
pub enum ConfigError {
    Gain,
    PWMFreq,
    MaxTarget,
    StartupTarget,
    ThrottleBounds,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Enum))]
#[cfg_attr(feature = "defmt", derive(Format))]
#[repr(u8)]
pub enum RuntimeError {
    Flash = 0x10,
    Overcurrent = 0x20,
    Overtemperature,
    InvariantLoad,
    ArithmeticError,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Enum))]
#[repr(u8)]
/// cannot be named "Error" because of Swift :/
pub enum HeadlightError {
    #[default]
    None = 0x00,
    Config {
        e: ConfigError,
    } = 0x20,
    Runtime {
        e: RuntimeError,
    } = 0x30,
}

impl From<ConfigError> for HeadlightError {
    fn from(value: ConfigError) -> Self {
        Self::Config { e: value }
    }
}

impl From<RuntimeError> for HeadlightError {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime { e: value }
    }
}

#[derive(Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Enum))]
#[repr(u8)]
pub enum Mode {
    #[default]
    Idle = 0xf0,
    Running = 0xfa,
    Throttling = 0xf2,
    Fault = 0xf3,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Enum))]
#[repr(u8)]
pub enum Hardware {
    V2Rev0,
    V2Rev1,
    V2Rev3,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Enum))]
#[repr(u8)]
pub enum Firmware {
    V0P1,
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(not(target_os = "none"), derive(uniffi::Record))]
pub struct Version {
    pub hw: Hardware,
    pub fw: Firmware,
}
