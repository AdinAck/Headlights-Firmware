use crate::{
    command::commands::Properties,
    types::{Firmware, Hardware, Version},
    utils::thermistor::celsius_to_sample,
};

pub const PROPERTIES: Properties = Properties {
    version: Version {
        hw: Hardware::V2Rev3,
        fw: Firmware::V0P1,
    },
    abs_max_ma: 1000,
    abs_max_temp: celsius_to_sample(95),
    min_pwm_freq: 50,
    max_pwm_freq: 500,
    max_adc_error: 10,
};
