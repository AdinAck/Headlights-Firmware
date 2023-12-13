use crate::{
    fmt::{error, info},
    FaultLEDPin, HBEnablePin, MeasureResources, PWMTimer,
};
use common::{
    command::commands::*, properties::PROPERTIES, types::*, utils::thermistor::celsius_to_sample,
};
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

use super::adc::{mv_to_ma, sample_to_mv};

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
    status: Signal<CriticalSectionRawMutex, Status>,
    shutdown_start: Signal<CriticalSectionRawMutex, ()>,
    shutdown_confirm: Signal<CriticalSectionRawMutex, ()>,
}

impl RegulatorProxy {
    pub const fn new() -> Self {
        Self {
            control: Signal::new(),
            monitor: Signal::new(),
            status: Signal::new(),
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

    pub async fn wait_for_new_status(&self) -> Status {
        self.status.wait().await
    }
}

pub struct Regulator<'a> {
    hw: RegulatorHardware<'a>,

    pid: PIDController<i32>,

    max_target: u16,
    max_current: u16,
    max_duty: u16,
    throttle_start: u16,
    throttle_stop: u16,
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
                (config.pwm_freq.div_ceil(config.gain.into())).into(),
                config.pwm_freq.into(),
            ),
            max_target: config.max_target_current,
            max_current: config.abs_max_load_current,
            max_duty,
            throttle_start: celsius_to_sample(config.throttle_start),
            throttle_stop: celsius_to_sample(config.throttle_stop),
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

    pub async fn get_reading(&mut self) -> Option<(u16, u16)> {
        let vref_sample = self.hw.adc.read(&mut self.hw.vref).await;
        let raw_temp = self.hw.adc.read(&mut self.hw.measure.temp).await;
        let raw_current = self.hw.adc.read(&mut self.hw.measure.cur_sense).await; // measure current last to be more fresh ;)

        // raw temp is more accurate than comparing to vref since the measurement is a voltage divider from VDD
        Some((mv_to_ma(sample_to_mv(raw_current, vref_sample)?)?, raw_temp))
    }

    fn check_fault(
        &self,
        current: u16,
        target: u16,
        temperature: u16,
        duty: u16,
    ) -> Option<RuntimeError> {
        if current > self.max_current + PROPERTIES.max_adc_error {
            // current is sufficiently over max target to be considered unsafe
            Some(RuntimeError::Overcurrent)
        } else if current < target && duty == self.max_duty {
            // current is low at max duty (load is disconnected or supply voltage is too low)
            Some(RuntimeError::InvariantLoad)
        } else if temperature > self.throttle_stop {
            Some(RuntimeError::Overtemperature)
        } else {
            None
        }
    }

    fn thermal_throttle(&self, temperature: u16, target: u16) -> Result<(u16, bool), RuntimeError> {
        if temperature >= self.throttle_start {
            Ok((
                min(
                    target,
                    // linear ramp
                    u32::from(self.throttle_stop - temperature)
                        .checked_mul(u32::from(self.max_target))
                        .ok_or(RuntimeError::ArithmeticError)?
                        .checked_div(u32::from(self.throttle_stop - self.throttle_start))
                        .ok_or(RuntimeError::ArithmeticError)?
                        .try_into()
                        .map_err(|_| RuntimeError::ArithmeticError)?,
                ),
                true,
            ))
        } else {
            Ok((target, false))
        }
    }

    fn next_duty(
        &mut self,
        current: u16,
        upper: &mut u16,
        lower: &mut u16,
        target: u16,
        prev_duty: u16,
    ) -> Result<u16, RuntimeError> {
        if let Some(delta) = self.pid.run(target.into(), current.into()) {
            if current < *lower || delta > 0 {
                // if pid has decided to go up or recent current
                // is less than lower, we hit the lower current bound
                *lower = current;
            }

            if current > *upper || delta < 0 {
                // if pid has decided to go down or recent current
                // is greater than upper, we hit the upper current bound
                *upper = current;
            }

            Ok(min(self.max_duty, prev_duty.saturating_add_signed(delta)))
        } else {
            Err(RuntimeError::ArithmeticError)
        }
    }

    async fn run(&mut self, proxy: &RegulatorProxy) {
        let mut control = proxy.control.wait().await;
        let mut duty = 0;
        let mut upper_current = 0;
        let mut lower_current = 0;

        let mut status = Status {
            mode: Mode::Running,
            error: HeadlightError::None,
        };

        self.startup();
        proxy.status.signal(status);

        let mut ticker = Ticker::every(Duration::from_ticks(4));

        let error = loop {
            if let Some((current, temperature)) = self.get_reading().await {
                if let Some(error) = self.check_fault(current, control.target, temperature, duty) {
                    break Some(error);
                }

                // get throttled target
                let target = match self.thermal_throttle(temperature, control.target) {
                    Ok((throttled_target, throttling)) => {
                        if throttling && status.mode != Mode::Throttling {
                            status.mode = Mode::Throttling;
                            proxy.status.signal(status);
                        } else if !throttling && status.mode != Mode::Running {
                            status.mode = Mode::Running;
                            proxy.status.signal(status);
                        }

                        throttled_target
                    }
                    Err(e) => {
                        break Some(e);
                    }
                };

                // update pid/pwm
                match self.next_duty(
                    current,
                    &mut upper_current,
                    &mut lower_current,
                    target,
                    duty,
                ) {
                    Ok(new_duty) => {
                        self.hw.pwm.set_duty(Self::CHANNEL, new_duty);
                        duty = new_duty;
                    }
                    Err(e) => {
                        break Some(e);
                    }
                }

                // update local control with global immediately or skip
                if proxy.control.signaled() {
                    // wait will be instantaneous because nothing else can wait for this signal
                    control = proxy.control.wait().await;
                }

                // push local monitor to proxy
                proxy.monitor.signal(Monitor {
                    duty,
                    upper_current,
                    lower_current,
                    temperature,
                });

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
            status.mode = Mode::Fault;
            status.error = error.into();
            self.hw.fault.set_high();
            error!(
                "The current state was determined to be unsafe for reason: {}. Shutting down.",
                error
            );
        } else {
            status.mode = Mode::Idle;
        }

        proxy.status.signal(status);
    }
}

#[embassy_executor::task]
pub async fn regulation_worker(mut regulator: Regulator<'static>, proxy: &'static RegulatorProxy) {
    regulator.run(proxy).await;
    info!("Regulation has ended.");
}
