//! UDP-based transport implementation

use tokio::net::UdpSocket;
use std::time::{Duration, Instant};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use bytes::{Bytes, BytesMut};

const DEFAULT_MTU: usize = 1500;
const HEADER_SIZE: usize = 12; // 4 bytes each for seed, degree, and sequence number

/// Rate limiter for controlling bandwidth usage
struct RateLimiter {
    bytes_per_second: u64,
    last_check: Instant,
    bytes_sent: u64,
}

impl RateLimiter {
    fn new(mbps: u32) -> Self {
        Self {
            bytes_per_second: (mbps as u64) * 1024 * 1024 / 8,
            last_check: Instant::now(),
            bytes_sent: 0,
        }
    }

    async fn wait(&mut self, bytes: usize) {
        self.bytes_sent += bytes as u64;
        
        let elapsed = self.last_check.elapsed().as_secs_f64();
        let expected_time = self.bytes_sent as f64 / self.bytes_per_second as f64;
        
        if expected_time > elapsed {
            let sleep_duration = Duration::from_secs_f64(expected_time - elapsed);
            tokio::time::sleep(sleep_duration).await;
        }
        
        // Reset counter every second
        if elapsed >= 1.0 {
            self.last_check = Instant::now();
            self.bytes_sent = 0;
        }
    }
}

pub struct UdpTransport {
    socket: Arc<UdpSocket>,
    mtu: usize,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl UdpTransport {
    pub async fn new(bind_addr: &str, rate_limit_mbps: u32) -> Result<Self> {
        let socket = UdpSocket::bind(bind_addr).await?;
        let rate_limiter = RateLimiter::new(rate_limit_mbps);
        
        Ok(Self {
            socket: Arc::new(socket),
            mtu: DEFAULT_MTU,
            rate_limiter: Arc::new(Mutex::new(rate_limiter)),
        })
    }

    /// Send a block of data
    pub async fn send_block(&self, target: &str, block_data: &[u8], seed: u32, degree: usize, seq: u32) -> Result<()> {
        let mut buffer = BytesMut::with_capacity(HEADER_SIZE + block_data.len());
        
        // Add header
        buffer.extend_from_slice(&seed.to_be_bytes());
        buffer.extend_from_slice(&(degree as u32).to_be_bytes());
        buffer.extend_from_slice(&seq.to_be_bytes());
        
        // Add data
        buffer.extend_from_slice(block_data);
        
        // Apply rate limiting
        self.rate_limiter.lock().await.wait(buffer.len()).await;
        
        // Send data
        self.socket.send_to(&buffer, target).await?;
        
        Ok(())
    }

    /// Receive a block of data
    pub async fn receive_block(&self) -> Result<(Bytes, u32, usize, u32, std::net::SocketAddr)> {
        let mut buffer = vec![0u8; self.mtu];
        let (len, addr) = self.socket.recv_from(&mut buffer).await?;
        
        if len < HEADER_SIZE {
            anyhow::bail!("Received packet too small");
        }
        
        // Parse header
        let seed = u32::from_be_bytes(buffer[0..4].try_into()?);
        let degree = u32::from_be_bytes(buffer[4..8].try_into()?) as usize;
        let seq = u32::from_be_bytes(buffer[8..12].try_into()?);
        
        // Extract data
        let data = Bytes::copy_from_slice(&buffer[HEADER_SIZE..len]);
        
        Ok((data, seed, degree, seq, addr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_rate_limiter() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let mut limiter = RateLimiter::new(1); // 1 Mbps
            let start = Instant::now();
            
            // Try to send 1 MB
            limiter.wait(1024 * 1024).await;
            
            // Should take approximately 1 second
            let elapsed = start.elapsed().as_secs_f64();
            assert!(elapsed >= 0.9 && elapsed <= 1.1);
        });
    }

    #[test]
    fn test_transport_creation() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let transport = UdpTransport::new("127.0.0.1:0", 1000).await;
            assert!(transport.is_ok());
        });
    }
}
