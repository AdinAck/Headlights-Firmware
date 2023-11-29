use embassy_stm32::gpio::{Level, Output, Speed};

use crate::{FaultLEDPin, FaultResources};

pub fn setup_status<'a>(s: FaultResources) -> Output<'a, FaultLEDPin> {
    Output::new(s.led, Level::Low, Speed::Low)
}
