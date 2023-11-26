use common::command_writer::HeadlightCommandWriter;
use common::{bundles::ToHeadlightBundle, use_to_headlight_bundle};
use embassy_nrf::{
    buffered_uarte::BufferedUarteTx,
    peripherals::{TIMER1, UARTE0},
};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

use crate::fmt::error;

pub type WriterQueue = Channel<ThreadModeRawMutex, ToHeadlightBundle, 8>;

#[embassy_executor::task]
pub async fn send_command_worker(
    mut writer: HeadlightCommandWriter<BufferedUarteTx<'static, 'static, UARTE0, TIMER1>>,
    queue: &'static WriterQueue,
) {
    loop {
        let bundle = queue.recv().await;

        use_to_headlight_bundle!(bundle, |cmd| {
            if let Err(e) = writer.send(cmd).await {
                error!("Command failed to send with error: {}", e);
            }
        })
    }
}
