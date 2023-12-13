use crate::command::commands::*;
use tiny_serde::{Deserialize, Serialize};

macro_rules! ser_std_impl {
    ($CMD:ty, $F:ident) => {
        #[uniffi::export]
        fn $F(cmd: &$CMD) -> Vec<u8> {
            cmd.clone().serialize().into()
        }
    };
}

macro_rules! de_std_impl {
    ($CMD:ty, $F:ident) => {
        #[uniffi::export]
        fn $F(buf: Vec<u8>) -> Option<$CMD> {
            <$CMD>::deserialize(buf.try_into().ok()?)
        }
    };
}

ser_std_impl!(Request, serialize_std_request);
ser_std_impl!(Control, serialize_std_control);
ser_std_impl!(Config, serialize_std_config);
ser_std_impl!(Reset, serialize_std_reset);

de_std_impl!(Properties, deserialize_std_properties);
de_std_impl!(AppError, deserialize_std_app_error);
de_std_impl!(Status, deserialize_std_status);
de_std_impl!(Control, deserialize_std_control);
de_std_impl!(Monitor, deserialize_std_monitor);
de_std_impl!(Config, deserialize_std_config);
