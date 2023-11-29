use common::types::{Config, Control, Request, Reset, RuntimeError, Status};
use cortex_m::peripheral::SCB;

use crate::{
    fmt::{error, info, warn},
    utils::model::Model,
};
#[cfg(feature = "defmt")]
use defmt::Format;

#[cfg_attr(feature = "defmt", derive(Format))]
pub enum Error {
    RequestUnavailable,
}

pub trait Execute {
    async fn run(self, model: &Model) -> Result<(), Error>;
}

impl Execute for Request {
    async fn run(self, model: &Model) -> Result<(), Error> {
        let bundle = match self {
            Request::Status => Status {
                mode: model.get_mode().await,
                error: model.get_error().await,
            }
            .into(),
            Request::Control => model.get_control().await.into(),
            Request::Monitor => model
                .get_monitor_immediately()
                .await
                .ok_or(Error::RequestUnavailable)?
                .into(),
            Request::Config => model.config.clone().into(),
        };

        model.send_queue.send(bundle).await;

        Ok(())
    }
}

impl Execute for Control {
    async fn run(self, model: &Model) -> Result<(), Error> {
        model.set_control(self).await;
        Ok(())
    }
}

impl Execute for Config {
    async fn run(self, model: &Model) -> Result<(), Error> {
        match self.try_into() {
            Ok(valid_config) => {
                if model.config.enabled {
                    model.shutdown_regulation().await;
                }
                let mut lock = model.configurator.lock().await;
                match lock.write_config(valid_config) {
                    Ok(_) => {
                        info!("Config write complete, resetting!");
                        Reset::Now.run(model).await.ok(); // infallible
                    }
                    Err(e) => {
                        model.set_error(RuntimeError::Flash.into()).await;
                        error!("Failed to write config with error: {}.", e);
                    }
                }
            }
            Err(e) => {
                model.set_error(e.into()).await;
                warn!("Received config was invalid for reason: {}.", e);
            }
        }

        Ok(())
    }
}

impl Execute for Reset {
    async fn run(self, _model: &Model) -> Result<(), Error> {
        match self {
            Self::Now => SCB::sys_reset(),
        }
    }
}
