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

use command::reader::receive_command_worker;
use command::writer::{send_command_worker, WriterQueue};
use cortex_m::peripheral::SCB;
use cortex_m_rt::{entry, exception};
use embassy_stm32::{flash::Flash, gpio::Output};
use fmt::info;
#[cfg(not(feature = "defmt"))]
use panic_halt as _;
use pid::PIDController;
use static_cell::StaticCell;
use utils::{
    adc::setup_adc, config::load_config, hb::setup_hb, regulation::regulation_worker,
    status::setup_status, uart::setup_uart,
};
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use common::{
    assign_resources, command_reader::HeadlightCommandReader,
    command_writer::HeadlightCommandWriter,
};
use embassy_executor::{Executor, InterruptExecutor};
use embassy_stm32::{
    adc, bind_interrupts,
    interrupt::{InterruptExt, Priority},
    peripherals::{self, ADC, PA12, PA6, TIM1, USART1},
    rcc,
    time::mhz,
    Config as PeripheralConfig,
};
use embassy_stm32::{interrupt, usart};

mod command;
mod fmt;
mod utils;

#[cfg(not(feature = "defmt"))]
#[exception]
unsafe fn DefaultHandler(_irqn: i16) -> ! {
    SCB::sys_reset()
}

bind_interrupts!(struct Irqs {
    ADC1 => adc::InterruptHandler<ADC>;
    USART1 => usart::BufferedInterruptHandler<USART1>;
});

assign_resources! {
    pub status: StatusResources {
        led: PA6 = StatusPin
    }
    pub measure: MeasureResources {
        cur_sense: PA2,
        temp: PA5
    }
    pub hb: HalfBridgeResources {
        timer: TIM1 = PWMTimer,
        control: PA8,
        sync: PA7,
        enable: PA12 = HBEnablePin
    }
    pub serial: SerialResources {
        uart: USART1,
        rx: PA10,
        tx: PA9
    }
}

static PRIORITY_EXECUTOR: InterruptExecutor = InterruptExecutor::new();
static NORMAL_EXECUTOR: StaticCell<Executor> = StaticCell::new();

static SEND_QUEUE: WriterQueue = WriterQueue::new();
static STATUS: StaticCell<Output<'static, StatusPin>> = StaticCell::new();

const MIN_PWM_FREQ: u16 = 50;
const MAX_PWM_FREQ: u16 = 500;
const TARGET_MA: u16 = 50;
const ABS_MAX_MA: u16 = 100;
const EPSILON: u16 = 50;

#[interrupt]
unsafe fn I2C1() {
    PRIORITY_EXECUTOR.on_interrupt()
}

#[entry]
fn main() -> ! {
    // interrupts
    interrupt::I2C1.set_priority(Priority::P1);
    interrupt::USART1.set_priority(Priority::P2);

    // config rcc
    let mut rcc_config = rcc::Config::default();
    rcc_config.sys_ck = mhz(48).into();

    // config peripherals
    let mut peripheral_config = PeripheralConfig::default();
    peripheral_config.rcc = rcc_config;

    // distribute peripherals
    let p = embassy_stm32::init(peripheral_config);
    let r = split_resources!(p);

    // setup minimal peripherals
    let mut flash = Flash::new_blocking(p.FLASH);
    let status = STATUS.init(setup_status(r.status));

    // TODO: this needs to update the STATE and ERROR flags depending on what happens
    let headlight_config = load_config(&mut flash, status);

    info!("Loaded headlight configuration: {}.", headlight_config);

    // setup comms peripherals
    let (tx, rx) = setup_uart(r.serial, 9600);

    // setup command reader/writer
    let reader = HeadlightCommandReader::new(rx);
    let writer = HeadlightCommandWriter::new(tx);

    if headlight_config.enabled {
        let pids = headlight_config.pid;

        let pid = PIDController::new(
            pids.k_p.into(),
            pids.k_i.into(),
            pids.k_d.into(),
            pids.windup_limit.into(),
            pids.div.into(),
        );

        // setup regulation peripherals
        let (adc, vref) = setup_adc(p.ADC);
        let (pwm, enable) = setup_hb(r.hb, headlight_config.pwm.freq);

        // start high priority executor for regulation
        let spawner = PRIORITY_EXECUTOR.start(interrupt::I2C1);
        spawner.must_spawn(regulation_worker(
            adc, vref, pwm, enable, status, r.measure, pid,
        ));
    }

    // start low priority executor for comms
    let executor = NORMAL_EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.must_spawn(receive_command_worker(reader));
        spawner.must_spawn(send_command_worker(writer, &SEND_QUEUE));
    });
}
