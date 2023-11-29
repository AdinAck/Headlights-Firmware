use crate::{
    fmt::{error, info},
    FaultLEDPin, HBEnablePin, MeasureResources, PWMTimer, EPSILON,
};
use common::types::{Config, Control, Mode, Monitor, RuntimeError};
use core::cmp::min;
use embassy_stm32::{
    adc::{Adc, Vref},
    gpio::Output,
    peripherals::ADC,
    timer::{complementary_pwm::ComplementaryPwm, Channel},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Ticker};
use pid::PIDController;

use super::{
    adc::{mv_to_ma, sample_to_mv},
    model::Model,
};

pub struct RegulatorHardware<'a> {
    pub adc: Adc<'a, ADC>,
    pub vref: Vref,
    pub measure: MeasureResources,

    pub pwm: ComplementaryPwm<'a, PWMTimer>,
    pub enable: Output<'a, HBEnablePin>,

    pub fault: Output<'a, FaultLEDPin>,
}

pub struct RegulatorProxy {
    control: Signal<CriticalSectionRawMutex, Control>,
    monitor: Signal<CriticalSectionRawMutex, Monitor>,
    shutdown_start: Signal<CriticalSectionRawMutex, ()>,
    shutdown_confirm: Signal<CriticalSectionRawMutex, ()>,
}

impl RegulatorProxy {
    pub const fn new() -> Self {
        Self {
            control: Signal::new(),
            monitor: Signal::new(),
            shutdown_start: Signal::new(),
            shutdown_confirm: Signal::new(),
        }
    }

    pub fn set_control(&self, control: Control) {
        self.control.signal(control);
    }

    pub async fn get_monitor_immediately(&self) -> Option<Monitor> {
        if self.monitor.signaled() {
            // wait will be instantaneous because nothing else can wait for this signal
            Some(self.monitor.wait().await)
        } else {
            None
        }
    }

    /// Instruct regulator to shutdown and wait for confirmation.
    ///
    /// Note: This function will only exit once the regulator is confirmed to have shutdown.
    pub async fn shutdown(&self) {
        self.shutdown_start.signal(());
        self.shutdown_confirm.wait().await;
    }
}

pub struct Regulator<'a> {
    hw: RegulatorHardware<'a>,

    pid: PIDController<i32>,

    max_duty: u16,
}

impl<'a> Regulator<'a> {
    const CHANNEL: Channel = Channel::Ch1;

    pub fn new(hw: RegulatorHardware<'a>, config: &Config) -> Self {
        let max_duty = hw.pwm.get_max_duty() - 1;

        Self {
            hw,
            pid: PIDController::new(
                0,
                config.gain.into(),
                0,
                (config.pwm_freq / config.gain).into(),
                config.pwm_freq.into(),
            ),
            max_duty,
        }
    }

    fn startup(&mut self) {
        self.hw.enable.set_high();
        self.hw.pwm.enable(Self::CHANNEL);
    }

    fn shutdown(&mut self) {
        self.hw.pwm.disable(Self::CHANNEL);
        self.hw.enable.set_low();
    }

    pub async fn get_reading(&mut self) -> Option<(u16, u8)> {
        let vref_sample = self.hw.adc.read(&mut self.hw.vref).await;
        let raw_temp = self.hw.adc.read(&mut self.hw.measure.temp).await;
        let raw_current = self.hw.adc.read(&mut self.hw.measure.cur_sense).await; // measure current last to be more fresh ;)

        Some((
            mv_to_ma(sample_to_mv(raw_current, vref_sample)?)?,
            sample_to_mv(raw_temp, vref_sample)? as u8, /* TODO */
        ))
    }

    fn check_fault(&self, monitor: &Monitor, control: &Control) -> Option<RuntimeError> {
        if monitor.current > control.target + EPSILON {
            // current is sufficiently over target to be considered unsafe
            Some(RuntimeError::Overcurrent)
        } else if monitor.current < control.target && monitor.duty == self.max_duty {
            // current is low at max duty (load is disconnected or supply voltage is too low)
            Some(RuntimeError::InvariantLoad)
        } else {
            None
        }
    }

    fn next_duty(&mut self, monitor: &Monitor, control: &Control) -> Result<u16, RuntimeError> {
        if let Some(delta) = self.pid.run(control.target.into(), monitor.current.into()) {
            Ok(min(
                self.max_duty,
                monitor.duty.saturating_add_signed(delta),
            ))
        } else {
            Err(RuntimeError::ArithmeticError)
        }
    }

    async fn run(&mut self, proxy: &RegulatorProxy, model: &Model) {
        let mut control = proxy.control.wait().await;
        let mut monitor;
        let mut duty = 0;

        self.startup();
        model.set_mode(Mode::Running).await;

        let mut ticker = Ticker::every(Duration::from_ticks(4));

        let error = loop {
            // get a reading (current and temp)
            if let Some((current, temperature)) = self.get_reading().await {
                monitor = Monitor {
                    duty,
                    current,
                    temperature,
                };

                // check for any fault from reading
                if let Some(error) = self.check_fault(&monitor, &control) {
                    break Some(error);
                }

                // update pid/pwm
                match self.next_duty(&monitor, &control) {
                    Ok(new_duty) => {
                        self.hw.pwm.set_duty(Self::CHANNEL, new_duty);
                        duty = new_duty;
                    }
                    Err(e) => {
                        break Some(e);
                    }
                }

                // TODO: model.set_control_immediately(...) for thermal throttling

                // update local control with global immediately or skip
                if proxy.control.signaled() {
                    // wait will be instantaneous because nothing else can wait for this signal
                    control = proxy.control.wait().await;
                }

                // push local monitor to global
                proxy.monitor.signal(monitor);

                // check for shutdown signal
                if proxy.shutdown_start.signaled() {
                    break None;
                }

                // wait for next cycle
                ticker.next().await;
            } else {
                break Some(RuntimeError::ArithmeticError);
            }
        };

        self.shutdown();
        proxy.shutdown_confirm.signal(());

        if let Some(error) = error {
            model.set_mode(Mode::Fault).await;
            self.hw.fault.set_high();
            error!(
                "The current state was determined to be unsafe for reason: {}. Shutting down.",
                error
            );
        } else {
            model.set_mode(Mode::Idle).await;
        }
    }
}

#[embassy_executor::task]
pub async fn regulation_worker(
    mut regulator: Regulator<'static>,
    proxy: &'static RegulatorProxy,
    model: &'static Model,
) {
    regulator.run(proxy, model).await;
    info!("Regulation has ended.");
}
