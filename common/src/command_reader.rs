use crate::{
    commands::CommandHeader,
    fmt::{error, unwrap, warn},
    scan_buf::ScanBuf,
    types::{CRCRepr, CommandID},
    CRC,
};
use crc::Digest;
use embedded_io_async::BufRead;
use pattern::{Pattern, PatternError};

pub trait ParseCommandBundle
where
    Self: Sized,
{
    fn parse<I: Iterator<Item = u8>>(
        id: CommandID,
        pattern: &mut Pattern<I>,
        digest: &mut Digest<CRCRepr>,
    ) -> Result<Option<Self>, PatternError>;
}

struct ParsedCommand<Bundle>(usize, CommandHeader, CRCRepr, Bundle);

pub struct HeadlightCommandReader<HWReader, const N: usize>
where
    HWReader: BufRead,
{
    rx: HWReader,
    buf: ScanBuf<N>,
}

impl<HWReader, const N: usize> HeadlightCommandReader<HWReader, N>
where
    HWReader: BufRead,
{
    pub const fn new(rx: HWReader) -> Self {
        Self {
            rx,
            buf: ScanBuf::new(),
        }
    }

    pub async fn poll(&mut self) {
        let incoming = unwrap!(self.rx.fill_buf().await);
        let n = incoming.len();

        if self.buf.push_slice(incoming).is_err() {
            error!("Incoming UART buffer full, data is being lost.");
            self.buf.clear();
        }

        self.rx.consume(n);
    }

    fn try_parse_cmd<Bundle>(&mut self) -> Result<ParsedCommand<Bundle>, PatternError>
    where
        Bundle: ParseCommandBundle,
    {
        let mut pattern = Pattern::new(self.buf.inner().iter().copied());

        loop {
            let mut digest = CRC.digest();
            let [id] = pattern.get().extract_and(|bytes| digest.update(bytes))?;

            let mut lookahead = pattern.clone();
            let [crc] = lookahead.get().extract()?;

            if let Some(bundle) = Bundle::parse(id, &mut lookahead, &mut digest)? {
                break Ok(ParsedCommand(
                    lookahead.count(),
                    CommandHeader { id, crc },
                    digest.finalize(),
                    bundle,
                ));
            } else {
                continue;
            }
        }
    }

    fn validate_crc<Bundle>(
        header: CommandHeader,
        observed: CRCRepr,
        bundle: Bundle,
    ) -> Option<Bundle>
    where
        Bundle: ParseCommandBundle,
    {
        if header.crc == observed {
            Some(bundle)
        } else {
            warn!("Comms error (CRC) occurred when parsing incoming command.");
            None
        }
    }

    pub fn recognizes<Bundle>(&mut self) -> Option<Bundle>
    where
        Bundle: ParseCommandBundle,
    {
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
