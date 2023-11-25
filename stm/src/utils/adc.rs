use embassy_stm32::{
    adc::{Adc, AdcPin, Resolution, SampleTime, Vref},
    peripherals::ADC,
};
use embassy_time::Delay;

use crate::Irqs;

fn sample_to_mv(sample: u16, vref: u16) -> Option<u16> {
    // From https://www.st.com/resource/en/datasheet/stm32f031c6.pdf
    // 6.3.4 Embedded reference voltage
    const VREFINT_MV: u32 = 1230; // mV

    u16::try_from(
        u32::try_from(sample)
            .ok()?
            .checked_mul(VREFINT_MV)?
            .checked_div(u32::try_from(vref).ok()?)?,
    )
    .ok()
}

fn mv_to_ma(mv: u16) -> Option<u16> {
    const GAIN: u16 = 10;
    const R_SHUNT: u16 = 33;

    mv.checked_mul(GAIN)?.checked_div(R_SHUNT)
}

pub async fn get_current<'a, P>(
    adc: &mut Adc<'a, ADC>,
    vref: &mut Vref,
    sense_p: &mut P,
) -> Option<u16>
where
    P: AdcPin<ADC>,
{
    let vref_sample = adc.read(vref).await;
    let raw = adc.read(sense_p).await;

    mv_to_ma(sample_to_mv(raw, vref_sample)?)
}

pub fn setup_adc<'a>(hw_adc: ADC) -> (Adc<'a, ADC>, Vref) {
    let mut adc = Adc::new(hw_adc, Irqs, &mut Delay);
    adc.set_resolution(Resolution::TwelveBit);
    adc.set_sample_time(SampleTime::Cycles239_5);

    let vref = adc.enable_vref(&mut Delay);

    (adc, vref)
}
