use common::types::{Config, Control, Request, RuntimeError, Status};

use crate::{fmt::error, utils::model::Model};
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
                model.shutdown_regulation().await;
                let mut lock = model.configurator.lock().await;
                if let Err(e) = lock.write_config(valid_config) {
                    model.set_error(RuntimeError::Flash.into()).await;
                    error!("Failed to write config with error: {}.", e);
                }
            }
            Err(e) => model.set_error(e.into()).await,
        }

        Ok(())
    }
}
