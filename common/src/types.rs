use crate::command::HeadlightCommand;
use tiny_serde::{prelude::*, Deserialize, Serialize};
use tiny_serde_macros::{Deserialize, Serialize};

#[cfg(feature = "defmt")]
use defmt::Format;

// types
pub type CRCRepr = u8;
pub type CommandID = u8;

#[derive(Serialize, Deserialize)]
pub struct CommandHeader {
    pub id: CommandID,
    pub crc: CRCRepr,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
#[repr(u8)]
pub enum ConfigError {
    Gain,
    PWMFreq,
    MaxTarget,
    StartupTarget,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
#[repr(u8)]
pub enum RuntimeError {
    Flash = 0x10,
    Overcurrent = 0x20,
    InvariantLoad,
    ArithmeticError,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum Error {
    #[default]
    None = 0x00,
    Config(ConfigError) = 0x20,
    Runtime(RuntimeError) = 0x30,
}

impl From<ConfigError> for Error {
    fn from(value: ConfigError) -> Self {
        Self::Config(value)
    }
}

impl From<RuntimeError> for Error {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum Mode {
    #[default]
    Idle = 0xf0,
    Running = 0xfa,
    Throttling = 0xf2,
    Fault = 0xf3,
}

// diagnostic -- should not occur in prod
#[derive(Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum AppErrorData {
    None = 0x00,
    /// app sent a packet that could not be serialized
    InvalidPacket = 0x10,
    /// command built from app data could not be sent
    SendFault = 0x11,
    /// app sent a command before the previous finished dispatching
    TooFast = 0x12,
}

#[derive(Serialize, Deserialize)]
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
pub struct Status {
    pub mode: Mode,
    pub error: Error,
}

impl HeadlightCommand for Status {
    const ID: CommandID = 0x1f;
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Control {
    pub target: u16,
}

impl HeadlightCommand for Control {
    const ID: CommandID = 0xaa;
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Monitor {
    /// Duty cycle of regulation PWM
    pub duty: u16,
    /// Load current
    pub current: u16,
    /// Temperature of FETs (not load)
    pub temperature: u8,
}

impl HeadlightCommand for Monitor {
    const ID: CommandID = 0xab;
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Config {
    /// Indicates whether or not to begin regulation
    pub enabled: bool,
    /// Default control scheme before any user control
    pub startup_control: Control,
    /// The gain or sensitivity of the regulation feedback loop
    pub gain: u16,
    /// Frequency of PWM control signal for regulation
    pub pwm_freq: u16,
    /// User-defined maximum regulation current for the load
    pub max_target: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            startup_control: Control { target: 50 },
            gain: 1,
            pwm_freq: 300,
            max_target: 100,
        }
    }
}

impl HeadlightCommand for Config {
    const ID: CommandID = 0xac;
}

#[derive(Serialize, Deserialize)]
#[repr(u8)]
pub enum Reset {
    Now,
}

impl HeadlightCommand for Reset {
    const ID: CommandID = 0xff;
}
