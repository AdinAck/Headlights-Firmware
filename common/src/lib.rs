#![no_std]

pub mod assign_resources;
pub mod bundles;
pub mod command_reader;
pub mod command_writer;
pub mod commands;
mod fmt;
mod scan_buf;
pub mod types;

use crc::{Crc, CRC_8_AUTOSAR};
use types::CRCRepr;
pub(crate) const CRC: Crc<CRCRepr> = Crc::<CRCRepr>::new(&CRC_8_AUTOSAR);
