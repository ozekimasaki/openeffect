//! After Effects の `.aep` / `.aepx` / `.aet` をチャンクの木として読み書きする。
//!
//! 構造（RIFX、LIST、偽 LIST）と、二つの調査で一致したフィールドだけを解釈する。
//! オフセットが要確認のフィールドは決めず、ペイロードをバイト列のまま往復する。

#![forbid(unsafe_code)]

mod aepx;
mod chunk;
mod error;
mod fourcc;
mod rifx;
mod xml;

pub mod fields;

pub use aepx::{read_aepx, write_aepx};
pub use chunk::Chunk;
pub use error::Error;
pub use fourcc::FourCc;
pub use rifx::{read_aep, read_aet, write_aep, write_aet, Document};
