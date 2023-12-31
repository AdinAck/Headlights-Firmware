use crate::{
    command::extension::Execute,
    fmt::warn,
    utils::{ble::BLE, uart::BUF_SIZE},
};
use common::{command::reader::*, use_from_headlight_bundle, utils::bundles::FromHeadlightBundle};
use embassy_nrf::{
    buffered_uarte::BufferedUarteRx,
    peripherals::{TIMER1, UARTE0},
};

#[embassy_executor::task]
pub async fn receive_command_worker(
    mut reader: HeadlightCommandReader<BufferedUarteRx<'static, 'static, UARTE0, TIMER1>, BUF_SIZE>,
    ble: &'static BLE,
) {
    reader
        .dispatch(|bundle| async {
            use_from_headlight_bundle!(bundle, |cmd| {
                if let Some(conn) = ble.get_conn().await {
                    if let Err(e) = cmd.run(ble.get_server(), &conn) {
                        warn!("Command failed to dispatch with error: {}", e);
                    }
                } else {
                    warn!("Attempted to dispatch command while BLE client is not connected.")
                }
            });
        })
        .await;
}
