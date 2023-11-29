#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    buffered_uarte::{self, Baudrate},
    config::Config as PeripheralConfig,
    interrupt,
    interrupt::InterruptExt,
    peripherals::UARTE0,
};

mod ble;
mod command_ext;
mod command_reader;
mod command_writer;
mod fmt;
mod uart;

use ble::BLE;
use command_reader::receive_command_worker;
use command_writer::{send_command_worker, WriterQueue};
use common::{
    assign_resources, command_reader::HeadlightCommandReader,
    command_writer::HeadlightCommandWriter,
};
#[cfg(not(feature = "defmt"))]
use cortex_m::peripheral::SCB;
#[cfg(not(feature = "defmt"))]
use cortex_m_rt::exception;
use embassy_nrf::peripherals;
use uart::setup_uart;

static SEND_QUEUE: WriterQueue = WriterQueue::new();

#[cfg(not(feature = "defmt"))]
#[exception]
unsafe fn DefaultHandler(_irqn: i16) -> ! {
    SCB::sys_reset()
}

bind_interrupts!(struct Irqs {
    UARTE0_UART0 => buffered_uarte::InterruptHandler<UARTE0>;
});

assign_resources! {
    pub serial: SerialResource {
        uart: UARTE0,
        timer: TIMER1,
        ppi0: PPI_CH0,
        ppi1: PPI_CH1,
        ppi_group: PPI_GROUP0,
        rx: P1_12,
        tx: P1_11
    }
}

fn setup_interrupts(config: &mut PeripheralConfig) {
    config.gpiote_interrupt_priority = interrupt::Priority::P2;
    config.time_interrupt_priority = interrupt::Priority::P2;
    interrupt::UARTE0_UART0.set_priority(interrupt::Priority::P2);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut peripheral_config = PeripheralConfig::default();

    setup_interrupts(&mut peripheral_config);

    let p = embassy_nrf::init(peripheral_config);
    let r = split_resources!(p);

    let (rx, tx) = setup_uart(r.serial, Baudrate::BAUD9600);

    let reader = HeadlightCommandReader::new(rx);
    let writer = HeadlightCommandWriter::new(tx);

    let ble = BLE::init(&spawner).await;

    spawner.must_spawn(receive_command_worker(reader, ble));
    spawner.must_spawn(send_command_worker(writer, &SEND_QUEUE));

    ble.run(&SEND_QUEUE).await
}
