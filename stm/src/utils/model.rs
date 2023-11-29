use common::types::{Config, Control, Error as HeadlightError, Mode, Monitor, Status};
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

    pub async fn set_mode(&self, mode: Mode) {
        let mut lock = self.status.lock().await;
        lock.mode = mode;

        self.send_queue.send(lock.clone().into()).await;
    }

    pub async fn get_error(&self) -> HeadlightError {
        let lock = self.status.lock().await;
        lock.error.clone()
    }

    pub async fn set_error(&self, error: HeadlightError) {
        let mut lock = self.status.lock().await;
        lock.error = error;

        self.send_queue.send(lock.clone().into()).await;
    }

    pub async fn get_control(&self) -> Control {
        let lock = self.control.lock().await;
        lock.clone()
    }

    // wrap on top of regulator proxy

    pub async fn set_control(&self, control: Control) {
        let mut lock = self.control.lock().await;
        *lock = control;
        self.regulator_proxy.set_control(lock.clone());

        self.send_queue.send(lock.clone().into()).await;
    }

    /// Attempts to update the model's control attribute and push the command the send queue
    /// without waiting. If waiting is required, skip.
    pub fn set_control_immediately(&self, control: Control) {
        if let Ok(mut lock) = self.control.try_lock() {
            *lock = control;
            self.regulator_proxy.set_control(lock.clone());

            self.send_queue.try_send(lock.clone().into()).ok();
        }
    }

    pub async fn get_monitor_immediately(&self) -> Option<Monitor> {
        self.regulator_proxy.get_monitor_immediately().await
    }

    pub async fn shutdown_regulation(&self) {
        self.regulator_proxy.shutdown().await;
    }
}
