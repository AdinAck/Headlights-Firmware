use embassy_stm32::{
    gpio::{Level, Output, OutputType, Speed},
    time::khz,
    timer::{
        complementary_pwm::{ComplementaryPwm, ComplementaryPwmPin},
        simple_pwm::PwmPin,
        CountingMode,
    },
};

use crate::{HBEnablePin, HalfBridgeResources, PWMTimer};

pub fn setup_hb<'a>(
    hb: HalfBridgeResources,
    freq: u16,
) -> (ComplementaryPwm<'a, PWMTimer>, Output<'a, HBEnablePin>) {
    let control = PwmPin::new_ch1(hb.control, OutputType::PushPull);
    let sync = ComplementaryPwmPin::new_ch1(hb.sync, OutputType::PushPull);

    let pwm = ComplementaryPwm::new(
        hb.timer,
        Some(control),
        Some(sync),
        None,
        None,
        None,
        None,
        None,
        None,
        khz(freq.into()),
        CountingMode::EdgeAlignedUp,
    );

    let enable = Output::new(hb.enable, Level::Low, Speed::Low);

    (pwm, enable)
}
