use crate::utils::uart::BUF_SIZE;
use common::{
    bundles::ToHeadlightBundle, command_reader::HeadlightCommandReader, use_to_headlight_bundle,
};
use embassy_stm32::{peripherals::USART1, usart::BufferedUartRx};

#[embassy_executor::task]
pub async fn receive_command_worker(
    mut reader: HeadlightCommandReader<BufferedUartRx<'static, USART1>, BUF_SIZE>,
    // headlight model?
) {
    loop {
        reader.poll().await;

        if let Some(bundle) = reader.recognizes() {
            use_to_headlight_bundle!(bundle, |cmd| {});
        }
    }
}
