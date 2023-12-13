use crate::{
    command::extension::Execute,
    fmt::warn,
    utils::{model::Model, uart::BUF_SIZE},
};
use common::{
    command::reader::HeadlightCommandReader, use_to_headlight_bundle,
    utils::bundles::ToHeadlightBundle,
};
use embassy_stm32::{peripherals::USART1, usart::BufferedUartRx};

#[embassy_executor::task]
pub async fn receive_command_worker(
    mut reader: HeadlightCommandReader<BufferedUartRx<'static, USART1>, BUF_SIZE>,
    model: &'static Model,
) {
    reader
        .dispatch(|bundle| async {
            use_to_headlight_bundle!(bundle, |cmd| {
                if let Err(e) = cmd.run(model).await {
                    warn!("Command failed to dispatch with error: {}.", e);
                }
            });
        })
        .await;
}
