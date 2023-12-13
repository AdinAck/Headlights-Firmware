#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]

use command::reader::receive_command_worker;
use command::writer::send_command_worker;
#[cfg(not(feature = "defmt"))]
use cortex_m::peripheral::SCB;
use cortex_m_rt::entry;
#[cfg(not(feature = "defmt"))]
use cortex_m_rt::exception;
use embassy_stm32::flash::Flash;
use fmt::info;
#[cfg(not(feature = "defmt"))]
use panic_halt as _;
use static_cell::StaticCell;
use utils::{
    adc::setup_adc,
    config::Configurator,
    hb::setup_hb,
    model::{model_worker, Model},
    regulation::{regulation_worker, Regulator, RegulatorHardware, RegulatorProxy},
    status::setup_status,
    uart::setup_uart,
};
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use common::{
    assign_resources,
    command::{commands::*, reader::HeadlightCommandReader, writer::HeadlightCommandWriter},
    types::Mode,
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
    pub status: FaultResources {
        led: PA6 = FaultLEDPin
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

static REG_PROXY: RegulatorProxy = RegulatorProxy::new();
static MODEL: StaticCell<Model> = StaticCell::new();

#[interrupt]
unsafe fn I2C1() {
    PRIORITY_EXECUTOR.on_interrupt()
}

#[entry]
fn main() -> ! {
    // interrupts
    interrupt::I2C1.set_priority(Priority::P1);
    interrupt::USART1.set_priority(Priority::P2);

    // configure rcc
    let mut rcc_config = rcc::Config::default();
    rcc_config.sys_ck = mhz(48).into();

    // configure peripherals
    let mut peripheral_config = PeripheralConfig::default();
    peripheral_config.rcc = rcc_config;

    // distribute peripherals
    let p = embassy_stm32::init(peripheral_config);
    let r = split_resources!(p);

    // setup minimal peripherals
    let flash = Flash::new_blocking(p.FLASH);
    let mut fault = setup_status(r.status);

    // read config from flash and any error that occured while doing so
    let mut configurator = Configurator::new(flash);
    let (headlight_config, maybe_error) = configurator.load_config(&mut fault);

    // initialize device model
    let model = MODEL.init(Model::new(
        headlight_config,
        configurator,
        maybe_error.map_or(Status::default(), |error| Status {
            mode: Mode::default(),
            error,
        }),
        &REG_PROXY,
    ));

    info!("Loaded headlight configuration: {}.", model.config);

    // setup comms peripherals
    let (tx, rx) = setup_uart(r.serial, 9600);

    // setup command reader/writer
    let reader = HeadlightCommandReader::new(rx);
    let writer = HeadlightCommandWriter::new(tx);

    if model.config.enabled {
        // setup regulation peripherals
        let (adc, vref) = setup_adc(p.ADC);
        let (pwm, enable) = setup_hb(r.hb, model.config.pwm_freq);

        let regulator = Regulator::new(
            RegulatorHardware {
                adc,
                vref,
                measure: r.measure,
                pwm,
                enable,
                fault,
            },
            &model.config,
        );

        // start high priority executor for regulation
        let priority_executor = PRIORITY_EXECUTOR.start(interrupt::I2C1);
        priority_executor.must_spawn(regulation_worker(regulator, &REG_PROXY));
    } else {
        info!("Regulation is disabled.");
    }

    // start low priority executor for comms
    let normal_executor = NORMAL_EXECUTOR.init(Executor::new());
    normal_executor.run(|spawner| {
        spawner.must_spawn(model_worker(model));
        spawner.must_spawn(receive_command_worker(reader, model));
        spawner.must_spawn(send_command_worker(writer, &model.send_queue));
    });
}
