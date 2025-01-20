//! Fountainflow: High-performance file transfer using fountain codes
//! Based on RFC 5053 (Raptor Forward Error Correction)

pub mod block;
pub mod cli;
pub mod decoder;
pub mod distribution;
pub mod encoder;
pub mod fountain;
pub mod linear_algebra;
pub mod systematic;
pub mod tables;
pub mod transport;

pub use crate::cli::Cli;
pub use crate::fountain::Encoder;
pub use crate::decoder::Decoder;
pub use crate::transport::UdpTransport;