use crate::{
    command::{commands::*, reader::ParseCommandBundle},
    types::{CRCRepr, CommandID},
};
use bundle::bundle;
use crc::Digest;
use pattern::{Pattern, PatternError};

#[bundle(export)]
pub enum FromHeadlightBundle {
    Status(Status),
    Control(Control),
    Monitor(Monitor),
    Config(Config),
}

#[bundle(export)]
pub enum ToHeadlightBundle {
    Request(Request),
    Control(Control),
    Config(Config),
    Reset(Reset),
}

macro_rules! impl_parse {
    ($BUNDLE:ty, $MATCH:ident) => {
        impl ParseCommandBundle for $BUNDLE {
            fn parse<I: Iterator<Item = u8>>(
                id: CommandID,
                pattern: &mut Pattern<I>,
                digest: &mut Digest<CRCRepr>,
            ) -> Result<Option<Self>, PatternError> {
                $MATCH!(id, Cmd::ID => {
                    let [cmd]: [Cmd; 1] = pattern.get().extract_and(|bytes| digest.update(bytes) )?;
                    Ok(Some(cmd.into()))
                } else {
                    Ok(None)
                })
            }
        }

    };
}

impl_parse!(FromHeadlightBundle, match_from_headlight_bundle);
impl_parse!(ToHeadlightBundle, match_to_headlight_bundle);
