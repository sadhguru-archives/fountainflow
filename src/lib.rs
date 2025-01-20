//! Fountainflow: High-performance file transfer using fountain codes
//! Based on RFC 5053 (Raptor Forward Error Correction)

pub mod fountain;
pub mod transport;
pub mod cli;
pub mod decoder;

pub use crate::cli::Cli;
pub use crate::fountain::Encoder;
pub use crate::decoder::Decoder;
pub use crate::transport::UdpTransport;