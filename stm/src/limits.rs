use crate::utils::thermistor::celsius_to_sample;

/// min configurable PWM frequency
pub const MIN_PWM_FREQ: u16 = 50;
/// max configurable PWM frequency
pub const MAX_PWM_FREQ: u16 = 500;
/// absolute maximum current
pub const ABS_MAX_MA: u16 = 1_000;
/// max mA measurement (adc) error
pub const EPSILON: u16 = 10;
/// absolute maximum temperature
pub const ABS_MAX_TEMP: u16 = celsius_to_sample(95);
