use common::{command::commands::*, types::*};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

use crate::command::writer::WriterQueue;

use super::{
    config::{Configurator, ValidatedConfig},
    regulation::RegulatorProxy,
};

type ModelMutex<T> = Mutex<CriticalSectionRawMutex, T>;

pub struct Model {
    /// Configuration loaded on boot
    pub config: Config,
    /// Configurator for writing a new configuration to flash
    pub configurator: ModelMutex<Configurator<'static>>,
    /// Queue for commands to be sent
    pub send_queue: WriterQueue,
    /// Current status of the device
    status: ModelMutex<Status>,
    /// Copy of the regulator's active control scheme
    control: ModelMutex<Control>,
    /// A proxy for the regulator to push and pull directives/information
    regulator_proxy: &'static RegulatorProxy,
}

impl Model {
    pub fn new(
        config: ValidatedConfig,
        configurator: Configurator<'static>,
        initial_status: Status,
        regulator_proxy: &'static RegulatorProxy,
    ) -> Self {
        let config = config.inner();
        let control = config.startup_control.clone();

        regulator_proxy.set_control(config.startup_control.clone());

        Self {
            config,
            configurator: Mutex::new(configurator),
            status: Mutex::new(initial_status),
            control: Mutex::new(control),
            regulator_proxy,
            send_queue: WriterQueue::new(),
        }
    }

    pub async fn get_mode(&self) -> Mode {
        let lock = self.status.lock().await;
        lock.mode.clone()
    }

    pub async fn set_mode(&self, mode: Mode, notify: bool) {
        let mut lock = self.status.lock().await;
        lock.mode = mode;

        if notify {
            self.send_queue.send(lock.clone().into()).await;
        }
    }

    pub async fn get_error(&self) -> HeadlightError {
        let lock = self.status.lock().await;
        lock.error.clone()
    }

    pub async fn set_error(&self, error: HeadlightError, notify: bool) {
        let mut lock = self.status.lock().await;
        lock.error = error;

        if notify {
            self.send_queue.send(lock.clone().into()).await;
        }
    }

    pub async fn get_control(&self) -> Control {
        let lock = self.control.lock().await;
        lock.clone()
    }

    // wrap on top of regulator proxy

    pub async fn set_control(&self, control: Control, notify: bool) {
        let mut lock = self.control.lock().await;
        *lock = control;
        self.regulator_proxy.set_control(lock.clone());

        if notify {
            self.send_queue.send(lock.clone().into()).await;
        }
    }

    pub async fn get_monitor_immediately(&self) -> Option<Monitor> {
        self.regulator_proxy.get_monitor_immediately().await
    }

    pub async fn shutdown_regulation(&self) {
        self.regulator_proxy.shutdown().await;
    }

    pub async fn observe_regulator(&self) -> ! {
        loop {
            let status = self.regulator_proxy.wait_for_new_status().await;
            self.set_mode(status.mode, true).await;
            self.set_error(status.error, true).await;
        }
    }
}

#[embassy_executor::task]
pub async fn model_worker(model: &'static Model) -> ! {
    // notify status on startup
    model
        .send_queue
        .send(
            Status {
                mode: model.get_mode().await,
                error: model.get_error().await,
            }
            .into(),
        )
        .await;

    model.observe_regulator().await
}
