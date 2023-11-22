use core::mem::MaybeUninit;

use embassy_nrf::{
    buffered_uarte::{Baudrate, BufferedUarte, BufferedUarteRx, BufferedUarteTx},
    gpio::Pin,
    peripherals::{TIMER1, UARTE0},
    ppi::{ConfigurableChannel, Group as PPIGroup},
    uarte,
};

use crate::Irqs;

pub const BUF_SIZE: usize = 64;

static mut RX_BUF: [u8; BUF_SIZE] = [0u8; BUF_SIZE];
static mut TX_BUF: [u8; BUF_SIZE] = [0u8; BUF_SIZE];

static mut GLOBAL_UART: MaybeUninit<BufferedUarte<'static, UARTE0, TIMER1>> = MaybeUninit::uninit();

pub struct UART {}

impl UART {
    pub fn init<PPICHA, PPICHB, PPIGroupA, Rx, Tx>(
        uarte: UARTE0,
        tim: TIMER1,
        ppi0: PPICHA,
        ppi1: PPICHB,
        ppi_group: PPIGroupA,
        rx_pin: Rx,
        tx_pin: Tx,
        baudrate: Baudrate,
    ) -> (
        BufferedUarteRx<'static, 'static, UARTE0, TIMER1>,
        BufferedUarteTx<'static, 'static, UARTE0, TIMER1>,
    )
    where
        PPICHA: ConfigurableChannel,
        PPICHB: ConfigurableChannel,
        PPIGroupA: PPIGroup,
        Rx: Pin,
        Tx: Pin,
    {
        let mut uart_config = uarte::Config::default();
        uart_config.baudrate = baudrate;

        // buffers are independent so this is safe
        unsafe {
            GLOBAL_UART.write(BufferedUarte::new(
                uarte,
                tim,
                ppi0,
                ppi1,
                ppi_group,
                Irqs,
                rx_pin,
                tx_pin,
                uart_config,
                &mut RX_BUF,
                &mut TX_BUF,
            ))
        };

        unsafe { GLOBAL_UART.assume_init_mut() }.split()
    }
}
