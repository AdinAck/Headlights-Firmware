use crate::{
    ble::BLE,
    bundles::{match_receive_bundle, use_receive_bundle, ReceiveBundle},
    command_ext::Execute,
    fmt::{error, unwrap, warn},
    scan_buf::ScanBuf,
    uart::BUF_SIZE,
    CRCRepr, CRC,
};
use common::commands::*;
use embassy_nrf::{
    buffered_uarte::BufferedUarteRx,
    peripherals::{TIMER1, UARTE0},
};
use pattern::{Pattern, PatternError};

struct ParsedCommand(usize, CommandHeader, CRCRepr, ReceiveBundle);

pub struct HeadlightCommandReader<'a> {
    rx: BufferedUarteRx<'a, 'a, UARTE0, TIMER1>,
    buf: ScanBuf<BUF_SIZE>,
}

impl<'a> HeadlightCommandReader<'a> {
    pub const fn new(rx: BufferedUarteRx<'a, 'a, UARTE0, TIMER1>) -> Self {
        Self {
            rx,
            buf: ScanBuf::new(),
        }
    }

    async fn poll(&mut self) {
        let incoming = unwrap!(self.rx.fill_buf().await);
        let n = incoming.len();

        if self.buf.push_slice(incoming).is_err() {
            error!("Incoming UART buffer full, data is being lost.");
            self.buf.clear();
        }

        self.rx.consume(n);
    }

    fn try_parse_cmd(&mut self) -> Result<ParsedCommand, PatternError> {
        let mut pattern = Pattern::new(self.buf.inner().iter().copied());

        loop {
            let mut digest = CRC.digest();
            let [id] = pattern.get().extract_and(|bytes| digest.update(bytes))?;

            match_receive_bundle!(id, Cmd::ID => {
                let mut lookahead = pattern.clone();
                let [crc] = lookahead.get().extract()?;
                let [cmd]: [Cmd; 1] = lookahead.get().extract_and(|bytes| digest.update(bytes) )?;

                break Ok(ParsedCommand(lookahead.count(), CommandHeader { id, crc }, digest.finalize(), cmd.into()))
            } else {
                continue;
            })
        }
    }

    fn validate_crc(
        header: CommandHeader,
        observed: CRCRepr,
        bundle: ReceiveBundle,
    ) -> Option<ReceiveBundle> {
        if header.crc == observed {
            Some(bundle)
        } else {
            warn!("Comms error (CRC) occurred when parsing incoming command.");
            None
        }
    }

    fn recognizes(&mut self) -> Option<ReceiveBundle> {
        match self.try_parse_cmd() {
            Ok(ParsedCommand(count, header, observed_crc, bundle)) => {
                self.buf.eat(count);

                Self::validate_crc(header, observed_crc, bundle)
            }
            Err(PatternError::FailedDeserialize(count)) => {
                self.buf.eat(count);

                warn!("Received command was malformed, ignoring...");

                None
            }
            Err(PatternError::NotFound) => None,
        }
    }
}

#[embassy_executor::task]
pub async fn receive_command_worker(mut reader: HeadlightCommandReader<'static>, ble: &'static BLE) {
    loop {
        reader.poll().await;

        if let Some(bundle) = reader.recognizes() {
            use_receive_bundle!(bundle, |cmd| {
                if let Some(conn) = ble.get_conn().await {
                    if let Err(e) = cmd.run(ble.get_server(), &conn) {
                        warn!("Command failed to dispatch with error: {}", e);
                    }
                } else {
                    warn!("Attempted to dispatch command while BLE client is not connected.")
                }
            });
        }
    }
}
