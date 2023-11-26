use crate::{
    fmt::{error, info},
    utils::adc::get_current,
    HBEnablePin, MeasureResources, PWMTimer, StatusPin, ABS_MAX_MA, EPSILON, TARGET_MA,
};
use core::cmp::min;
use embassy_stm32::{
    adc::{Adc, Vref},
    gpio::Output,
    peripherals::ADC,
    timer::{complementary_pwm::ComplementaryPwm, Channel},
};
use embassy_time::{Duration, Ticker};
use pid::PIDController;

fn startup<'a>(
    pwm: &mut ComplementaryPwm<'a, PWMTimer>,
    enable: &mut Output<'static, HBEnablePin>,
    status: &mut Output<'static, StatusPin>,
) {
    enable.set_high();
    pwm.enable(Channel::Ch1);
    status.set_low();

    info!("Starting up regulated output...");
}

fn shutdown<'a>(
    mut pwm: ComplementaryPwm<'a, PWMTimer>,
    mut enable: Output<'static, HBEnablePin>,
    status: &mut Output<'static, StatusPin>,
    reason: &str,
) {
    enable.set_low();
    pwm.disable(Channel::Ch1);
    status.set_high();

    error!(
        "The current state was determined to be unsafe for reason: {}. Shutting down.",
        reason
    );
}

#[embassy_executor::task]
pub async fn regulation_worker(
    mut adc: Adc<'static, ADC>,
    mut vref: Vref,
    mut pwm: ComplementaryPwm<'static, PWMTimer>,
    mut enable: Output<'static, HBEnablePin>,
    status: &'static mut Output<'static, StatusPin>,
    mut m: MeasureResources,
    mut pid: PIDController<i32>,
) {
    let max_duty = pwm.get_max_duty() - 1;
    let min_duty = 1;
    info!("min/max duty: {}/{}", min_duty, max_duty);

    startup(&mut pwm, &mut enable, status);

    let mut duty = 1;

    let mut ticker = Ticker::every(Duration::from_ticks(4));

    let reason = loop {
        if let Some(current) = get_current(&mut adc, &mut vref, &mut m.cur_sense).await {
            // check for fault

            if current > ABS_MAX_MA + EPSILON {
                // current is sufficiently over target to be considered unsafe
                break "overcurrent";
            } else if current < TARGET_MA && duty == max_duty {
                // current is low at max duty (load is disconnected or supply voltage is too low)
                break "invariant load";
            } else if duty < min_duty {
                // if too small duty is determined to be needed to achieve target current the load or supply is too volatile
                break "hypervariant load";
            }

            // update pid/pwm

            if let Some(delta) = pid.run(TARGET_MA.into(), current.into()) {
                duty = min(max_duty, duty.saturating_add_signed(delta));
            } else {
                break "arithmetic error (pid)";
            }

            pwm.set_duty(Channel::Ch1, duty);

            // wait for next tick
            ticker.next().await;
        } else {
            break "arithmentic error (adc)";
        }
    };

    shutdown(pwm, enable, status, reason);
}
