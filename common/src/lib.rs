#![cfg_attr(target_os = "none", no_std)]

#[cfg(not(target_os = "none"))]
uniffi::include_scaffolding!("lib");

#[cfg(not(target_os = "none"))]
pub mod std_serde_impls;

pub mod command;
mod fmt;
pub mod properties;
pub mod types;
pub mod utils;

use crc::{Crc, CRC_8_AUTOSAR};
use types::CRCRepr;
pub(crate) const CRC: Crc<CRCRepr> = Crc::<CRCRepr>::new(&CRC_8_AUTOSAR);
