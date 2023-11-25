use embassy_stm32::gpio::{Level, Output, Speed};

use crate::{StatusPin, StatusResources};

pub fn setup_status<'a>(s: StatusResources) -> Output<'a, StatusPin> {
    Output::new(s.led, Level::Low, Speed::Low)
}
