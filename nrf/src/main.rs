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
mod bundles;
mod command_ext;
mod command_reader;
mod command_writer;
mod fmt;
mod scan_buf;
mod uart;

use ble::BLE;
use command_reader::{receive_command_worker, HeadlightCommandReader};
use command_writer::{send_command_worker, HeadlightCommandWriter, WriterQueue};
use common::types::CRCRepr;
use crc::{Crc, CRC_8_AUTOSAR};
use uart::UART;

pub const CRC: Crc<CRCRepr> = Crc::<CRCRepr>::new(&CRC_8_AUTOSAR);

static SEND_QUEUE: WriterQueue = WriterQueue::new();

bind_interrupts!(struct Irqs {
    UARTE0_UART0 => buffered_uarte::InterruptHandler<UARTE0>;
});

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

    let (rx, tx) = UART::init(
        p.UARTE0,
        p.TIMER1,
        p.PPI_CH0,
        p.PPI_CH1,
        p.PPI_GROUP0,
        p.P1_12,
        p.P1_11,
        Baudrate::BAUD9600,
    );

    let reader = HeadlightCommandReader::new(rx);
    let writer = HeadlightCommandWriter::new(tx);

    let ble = BLE::init(&spawner).await;

    spawner.must_spawn(receive_command_worker(reader, ble));
    spawner.must_spawn(send_command_worker(writer, &SEND_QUEUE));

    ble.run(&SEND_QUEUE).await
}
