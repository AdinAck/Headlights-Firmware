use embassy_nrf::{
    buffered_uarte::{Baudrate, BufferedUarte, BufferedUarteRx, BufferedUarteTx},
    peripherals::{TIMER1, UARTE0},
    uarte,
};
use static_cell::StaticCell;

use crate::{Irqs, SerialResource};

pub const BUF_SIZE: usize = 64;

static RX_BUF: StaticCell<[u8; BUF_SIZE]> = StaticCell::new();
static TX_BUF: StaticCell<[u8; BUF_SIZE]> = StaticCell::new();

static GLOBAL_UART: StaticCell<BufferedUarte<'static, UARTE0, TIMER1>> = StaticCell::new();

pub fn setup_uart(
    serial: SerialResource,
    baudrate: Baudrate,
) -> (
    BufferedUarteRx<'static, 'static, UARTE0, TIMER1>,
    BufferedUarteTx<'static, 'static, UARTE0, TIMER1>,
) {
    let mut uart_config = uarte::Config::default();
    uart_config.baudrate = baudrate;

    let rx_buf = RX_BUF.init([0; BUF_SIZE]);
    let tx_buf = TX_BUF.init([0; BUF_SIZE]);

    let uart = GLOBAL_UART.init(BufferedUarte::new(
        serial.uart,
        serial.timer,
        serial.ppi0,
        serial.ppi1,
        serial.ppi_group,
        Irqs,
        serial.rx,
        serial.tx,
        uart_config,
        rx_buf,
        tx_buf,
    ));

    uart.split()
}
