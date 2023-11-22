#![allow(unused)]

use crate::bundles::{use_send_bundle, SendBundle};
use crate::fmt::unwrap;
use crate::CRC;
use common::commands::{CommandHeader, HeadlightCommand};
use core::mem::MaybeUninit;
use embassy_nrf::{
    buffered_uarte::BufferedUarteTx,
    peripherals::{TIMER1, UARTE0},
};
use embassy_sync::channel::{Channel, TrySendError};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use tiny_serde::Serialize;

pub type WriterQueue = Channel<ThreadModeRawMutex, SendBundle, 8>;

pub struct HeadlightCommandWriter<'a> {
    tx: BufferedUarteTx<'a, 'a, UARTE0, TIMER1>,
}

impl<'a> HeadlightCommandWriter<'a> {
    pub const fn new(tx: BufferedUarteTx<'a, 'a, UARTE0, TIMER1>) -> Self {
        Self { tx }
    }

    async fn send<C, const N: usize>(&mut self, cmd: C)
    where
        C: HeadlightCommand + Serialize<N>,
    {
        let mut digest = CRC.digest();

        let payload = cmd.serialize();
        digest.update(&[C::ID]);
        digest.update(&payload);

        let header = CommandHeader {
            id: C::ID,
            crc: digest.finalize(),
        };

        unwrap!(self.tx.write(&header.serialize()).await);
        unwrap!(self.tx.write(&payload).await);
    }
}

#[embassy_executor::task]
pub async fn send_command_worker(mut writer: HeadlightCommandWriter<'static>, queue: &'static WriterQueue) {
    loop {
        let bundle = queue.recv().await;

        use_send_bundle!(bundle, |cmd| {
            writer.send(cmd).await;
        })
    }
}
