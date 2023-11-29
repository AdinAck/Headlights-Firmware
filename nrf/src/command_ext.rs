use crate::ble::Server;
use common::types::*;
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

impl Execute for Status {
    fn run(self, server: &Server, conn: &Connection) -> Result<(), CommandExecutionError> {
        server.headlight.status_notify(&conn, &self.serialize())?;

        Ok(())
    }
}

impl Execute for Control {
    fn run(self, server: &Server, conn: &Connection) -> Result<(), CommandExecutionError> {
        server.headlight.control_notify(&conn, &self.serialize())?;

        Ok(())
    }
}

impl Execute for Monitor {
    fn run(self, server: &Server, conn: &Connection) -> Result<(), CommandExecutionError> {
        server.headlight.monitor_notify(&conn, &self.serialize())?;

        Ok(())
    }
}

impl Execute for Config {
    fn run(self, server: &Server, conn: &Connection) -> Result<(), CommandExecutionError> {
        server.headlight.config_notify(&conn, &self.serialize())?;

        Ok(())
    }
}
