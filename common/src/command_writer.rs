use crate::{command::HeadlightCommand, types::CommandHeader, CRC};
use embedded_io_async::{ErrorType, Write};
use tiny_serde::Serialize;

pub struct HeadlightCommandWriter<HWWriter> {
    tx: HWWriter,
}

impl<HWWriter> HeadlightCommandWriter<HWWriter>
where
    HWWriter: Write,
{
    pub const fn new(tx: HWWriter) -> Self {
        Self { tx }
    }

    pub async fn send<C, const N: usize>(
        &mut self,
        cmd: C,
    ) -> Result<(), <HWWriter as ErrorType>::Error>
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

        self.tx.write(&header.serialize()).await?;
        self.tx.write(&payload).await?;

        Ok(())
    }
}
