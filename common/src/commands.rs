use crate::types::{CRCRepr, CommandID, ErrorData, StateData};
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
    state: StateData,
    error: ErrorData,
}

impl HeadlightCommand for StatusCommand {
    const ID: CommandID = 0x1f;
}

#[derive(Serialize, Deserialize)]
pub struct BrightnessCommand {
    brightness: u8,
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
pub struct PIDCommand {
    k_p: u8,
    k_i: u8,
    k_d: u8,
    div: u16,
}

impl HeadlightCommand for PIDCommand {
    const ID: CommandID = 0xac;
}
