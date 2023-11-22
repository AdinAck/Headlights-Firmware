use bundle::bundle;
use common::commands::*;

#[bundle]
pub enum ReceiveBundle {
    StatusCommand,
    BrightnessCommand,
    MonitorCommand,
    PIDCommand,
}

#[bundle]
pub enum SendBundle {
    RequestCommand,
    BrightnessCommand,
    PIDCommand,
}

pub(crate) use match_receive_bundle;
pub(crate) use use_receive_bundle;
pub(crate) use use_send_bundle;
