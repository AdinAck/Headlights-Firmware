use crate::ble::Server;
use common::commands::*;
#[cfg(feature = "defmt")]
use defmt::Format;
use nrf_softdevice::ble::{gatt_server::NotifyValueError, Connection};
use tiny_serde::Serialize;

#[cfg_attr(feature = "defmt", derive(Format))]
pub enum CommandExecutionError {
    NotifyValueError(NotifyValueError),
}

impl From<NotifyValueError> for CommandExecutionError {
    fn from(value: NotifyValueError) -> Self {
        Self::NotifyValueError(value)
    }
}

pub trait Execute {
    fn run(self, server: &Server, conn: &Connection) -> Result<(), CommandExecutionError>;
}

impl Execute for StatusCommand {
    fn run(self, server: &Server, conn: &Connection) -> Result<(), CommandExecutionError> {
        server
            .headlight
            .status_notify(&conn, &self.serialize())?;

        Ok(())
    }
}

impl Execute for BrightnessCommand {
    fn run(self, server: &Server, conn: &Connection) -> Result<(), CommandExecutionError> {
        server
            .headlight
            .brightness_notify(&conn, &self.serialize())?;

        Ok(())
    }
}

impl Execute for MonitorCommand {
    fn run(self, server: &Server, conn: &Connection) -> Result<(), CommandExecutionError> {
        server
            .headlight
            .monitor_notify(&conn, &self.serialize())?;

        Ok(())
    }
}

impl Execute for PIDCommand {
    fn run(self, server: &Server, conn: &Connection) -> Result<(), CommandExecutionError> {
        server
            .headlight
            .pid_notify(&conn, &self.serialize())?;

        Ok(())
    }
}
