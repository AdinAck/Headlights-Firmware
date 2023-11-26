use crate::types::{CRCRepr, CommandID, ErrorData, StateData};
#[cfg(feature = "defmt")]
use defmt::Format;
use tiny_serde::{prelude::*, Deserialize, Serialize};
use tiny_serde_macros::{Deserialize, Serialize};

pub trait HeadlightCommand {
    const ID: CommandID;
}

#[derive(Serialize, Deserialize)]
pub struct CommandHeader {
    pub id: CommandID,
    pub crc: CRCRepr,
}

#[derive(Serialize, Deserialize)]
#[repr(u8)]
pub enum RequestCommand {
    Noop = 0x00,
    Status = 0x1f,
    Brightness = 0xaa,
    Monitor = 0xab,
    PID = 0xac,
}

impl HeadlightCommand for RequestCommand {
    const ID: CommandID = 0x10;
}

#[derive(Serialize, Deserialize)]
pub struct StatusCommand {
    pub state: StateData,
    pub error: ErrorData,
}

impl HeadlightCommand for StatusCommand {
    const ID: CommandID = 0x1f;
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct BrightnessCommand {
    pub brightness: u8,
}

impl HeadlightCommand for BrightnessCommand {
    const ID: CommandID = 0xaa;
}

#[derive(Serialize, Deserialize)]
pub struct MonitorCommand {
    duty: u8,
    current: u8,
    temperature: u8,
}

impl HeadlightCommand for MonitorCommand {
    const ID: CommandID = 0xab;
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct PIDCommand {
    pub k_p: u8,
    pub k_i: u8,
    pub k_d: u8,
}

impl HeadlightCommand for PIDCommand {
    const ID: CommandID = 0xac;
}

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct PWMCommand {
    pub freq: u16,
}

impl HeadlightCommand for PWMCommand {
    const ID: CommandID = 0xad;
}
