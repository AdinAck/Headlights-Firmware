use tiny_serde::{prelude::*, Deserialize, Serialize};
use tiny_serde_macros::{Deserialize, Serialize};

// types
pub type CRCRepr = u8;
pub type CommandID = u8;

#[derive(Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum StateData {
    Idle = 0xf0,
    Running = 0xfa,
    Throttling = 0xf2,
    Fault = 0xf3,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum ErrorData {
    None = 0x00,
    Flash = 0x10,
    PIDTerms = 0x20,
    PWMFreq = 0x21,
}

// diagnostic -- should not occur in prod
#[derive(Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum AppErrorData {
    None = 0x00,
    InvalidPacket = 0x10, // app sent a packet that could not be serialized
    SendFault = 0x11,     // command built from app data could not be sent
    TooFast = 0x12,       // app sent a command before the previous finished dispatching
}
