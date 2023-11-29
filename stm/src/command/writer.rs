use common::{
    bundles::FromHeadlightBundle, command_writer::HeadlightCommandWriter, use_from_headlight_bundle,
};
use embassy_stm32::{peripherals::USART1, usart::BufferedUartTx};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

use crate::fmt::error;

pub type WriterQueue = Channel<CriticalSectionRawMutex, FromHeadlightBundle, 8>;

#[embassy_executor::task]
pub async fn send_command_worker(
    mut writer: HeadlightCommandWriter<BufferedUartTx<'static, USART1>>,
    queue: &'static WriterQueue,
) {
    loop {
        let bundle = queue.recv().await;

        use_from_headlight_bundle!(bundle, |cmd| {
            if let Err(e) = writer.send(cmd).await {
                error!("Command failed to send with error: {}", e);
            }
        });
    }
}
