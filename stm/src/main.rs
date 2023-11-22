// #![no_std]
// #![no_main]
// #![feature(type_alias_impl_trait)]

// use embassy_executor::Spawner;
// use embassy_stm32::flash::{Blocking, Flash};
// #[cfg(feature = "defmt")]
// use {defmt_rtt as _, panic_probe as _};
// #[cfg(not(feature = "defmt"))]
// use panic_halt as _;

// mod fmt;

// use fmt::{info, assert_eq, unwrap};

// #[embassy_executor::main]
// async fn main(_spawner: Spawner) {
//     let p = embassy_stm32::init(Default::default());

//     info!("Hello Flash!");

//     // Once can also call `into_regions()` to get access to NorFlash implementations
//     // for each of the unique characteristics.
//     let mut f = Flash::new_blocking(p.FLASH);

//     // Sector 5
//     test_flash(&mut f, 15 * 1024, 1024);
// }

// fn test_flash(f: &mut Flash<'_, Blocking>, offset: u32, size: u32) {
//     info!("Testing offset: {=u32:#X}, size: {=u32:#X}", offset, size);

//     info!("Reading...");
//     let mut buf = [0u8; 32];
//     unwrap!(f.read(offset, &mut buf));

//     info!("Read: {=[u8]:x}", buf);

//     info!("Erasing...");
//     unwrap!(f.blocking_erase(offset, offset + size));

//     info!("Reading...");
//     let mut buf = [0u8; 32];
//     unwrap!(f.read(offset, &mut buf));

//     info!("Read after erase: {=[u8]:x}", buf);

//     info!("Writing...");
//     unwrap!(f.blocking_write(
//         offset,
//         &[
//             1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
//             30, 31, 32
//         ]
//     ));

//     info!("Reading...");
//     let mut buf = [0u8; 32];
//     unwrap!(f.read(offset, &mut buf));

//     info!("Read: {=[u8]:x}", buf);
//     assert_eq!(
//         &buf[..],
//         &[
//             1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
//             30, 31, 32
//         ]
//     );
// }

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use cortex_m_rt::entry;
#[cfg(not(feature = "defmt"))]
use panic_halt as _;
use static_cell::StaticCell;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use common::assign_resources;
use core::cmp::min;
use embassy_executor::{Executor, InterruptExecutor};
use embassy_stm32::interrupt;
use embassy_stm32::{
    adc::{self, Adc, AdcPin, Vref},
    bind_interrupts,
    gpio::{Level, Output, OutputType, Speed},
    interrupt::{InterruptExt, Priority},
    peripherals::{self, ADC, PA12, PA6, TIM1, USART1},
    rcc,
    time::{khz, mhz},
    timer::{
        complementary_pwm::{ComplementaryPwm, ComplementaryPwmPin},
        simple_pwm::PwmPin,
        Channel as PwmChannel, CountingMode,
    },
    usart::BufferedUart,
    Config as PeripheralConfig,
};
use embassy_time::{Delay, Duration, Ticker};

mod commands_ext;
mod fmt;
mod pid;
mod uart;

use crate::fmt::{error, info};
use pid::PIDController;

bind_interrupts!(struct Irqs {
    ADC1 => adc::InterruptHandler<ADC>;
});

assign_resources! {
    status: StatusResources {
        led: PA6 = StatusPin
    }
    feedback: FeedbackResources {
        adc: ADC,
        sense: PA2
    }
    hb: HalfBridgeResources {
        timer: TIM1 = PWMTimer,
        control: PA8,
        sync: PA7,
        enable: PA12 = HBEnablePin
    }
    serial: SerialResources {
        uart: USART1,
        rx: PA10,
        tx: PA9
    }
}

static PRIORITY_EXECUTOR: InterruptExecutor = InterruptExecutor::new();
static NORMAL_EXECUTOR: StaticCell<Executor> = StaticCell::new();

static UART: StaticCell<BufferedUart<'static, USART1>> = StaticCell::new();

const TARGET_MA: u16 = 50;
const ABS_MAX_MA: u16 = 100;
const EPSILON: u16 = 50;

const PWM_FREQ: u16 = 300;

#[interrupt]
unsafe fn I2C1() {
    PRIORITY_EXECUTOR.on_interrupt()
}

#[allow(non_snake_case)]
fn adc_to_mV(sample: u16, vref: u16) -> Option<u16> {
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

#[allow(non_snake_case)]
fn mV_to_mA(mv: u16) -> Option<u16> {
    const GAIN: u16 = 10;
    const R_SHUNT: u16 = 33;

    mv.checked_mul(GAIN)?.checked_div(R_SHUNT)
}

async fn get_current<'a, P>(adc: &mut Adc<'a, ADC>, vref: &mut Vref, sense_p: &mut P) -> Option<u16>
where
    P: AdcPin<ADC>,
{
    let vref_sample = adc.read(vref).await;
    let raw = adc.read(sense_p).await;

    mV_to_mA(adc_to_mV(raw, vref_sample)?)
}

#[embassy_executor::task]
async fn run_comms() {}

fn shutdown<'a>(
    mut pwm: ComplementaryPwm<'a, PWMTimer>,
    mut enable: Output<'static, HBEnablePin>,
    mut status: Output<'static, StatusPin>,
    reason: &str,
) {
    enable.set_low();
    pwm.disable(PwmChannel::Ch1);
    status.set_high();

    error!(
        "The current state was determined to be unsafe for reason: {}. Shutting down.",
        reason
    );
}

fn setup_adc<'a>(hw_adc: ADC) -> (Adc<'a, ADC>, Vref) {
    let mut adc = Adc::new(hw_adc, Irqs, &mut Delay);
    adc.set_resolution(adc::Resolution::TwelveBit);
    adc.set_sample_time(adc::SampleTime::Cycles13_5);

    let vref = adc.enable_vref(&mut Delay);

    (adc, vref)
}

fn setup_hb<'a>(
    hb: HalfBridgeResources,
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
        khz(PWM_FREQ.into()),
        CountingMode::EdgeAlignedUp,
    );

    let enable = Output::new(hb.enable, Level::Low, Speed::Low);

    (pwm, enable)
}

#[embassy_executor::task]
async fn run_loop(mut f: FeedbackResources, hb: HalfBridgeResources, s: StatusResources) {
    let status = Output::new(s.led, Level::Low, Speed::Low);

    let (mut adc, mut vref) = setup_adc(f.adc);
    let (mut pwm, mut enable) = setup_hb(hb);

    let max_duty = pwm.get_max_duty() - 1;
    let min_duty = 1;
    info!("min/max duty: {}/{}", min_duty, max_duty);

    enable.set_high();
    pwm.enable(PwmChannel::Ch1);

    let mut duty = 1;

    let mut pid = PIDController::<i32>::new(10, 4, 0, (PWM_FREQ).into(), PWM_FREQ.into());

    let mut ticker = Ticker::every(Duration::from_ticks(4));

    let reason = loop {
        if let Some(current) = get_current(&mut adc, &mut vref, &mut f.sense).await {
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

            pwm.set_duty(PwmChannel::Ch1, duty);

            // wait for next tick
            ticker.next().await;
        } else {
            break "arithmentic error (adc)";
        }
    };

    shutdown(pwm, enable, status, reason);
}

#[entry]
fn main() -> ! {
    let mut rcc_config = rcc::Config::default();
    rcc_config.sys_ck = mhz(48).into();

    let mut peripheral_config = PeripheralConfig::default();
    peripheral_config.rcc = rcc_config;

    let p = embassy_stm32::init(peripheral_config);
    let r = split_resources!(p);

    interrupt::I2C1.set_priority(Priority::P2);
    let spawner = PRIORITY_EXECUTOR.start(interrupt::I2C1);
    spawner.must_spawn(run_loop(r.feedback, r.hb, r.status));

    // low priority
    let executor = NORMAL_EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.must_spawn(run_comms());
    });
}
