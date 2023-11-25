use embassy_stm32::{
    peripherals::USART1,
    usart::{self, BufferedUart, BufferedUartRx, BufferedUartTx},
};
use static_cell::StaticCell;

use crate::{fmt::unwrap, Irqs, SerialResources};

pub const BUF_SIZE: usize = 64;

static RX_BUF: StaticCell<[u8; BUF_SIZE]> = StaticCell::new();
static TX_BUF: StaticCell<[u8; BUF_SIZE]> = StaticCell::new();

pub fn setup_uart(
    serial: SerialResources,
    baudrate: u32,
) -> (
    BufferedUartTx<'static, USART1>,
    BufferedUartRx<'static, USART1>,
) {
    let mut uart_config = usart::Config::default();
    uart_config.baudrate = baudrate;

    let rx_buf = RX_BUF.init([0; BUF_SIZE]);
    let tx_buf = TX_BUF.init([0; BUF_SIZE]);

    let uart = unwrap!(BufferedUart::new(
        serial.uart,
        Irqs,
        serial.rx,
        serial.tx,
        tx_buf,
        rx_buf,
        uart_config,
    ));

    uart.split()
}
