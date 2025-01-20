use anyhow::Result;
use clap::Parser;
use fountainflow::{Cli, Encoder, fountain::Block, Decoder};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Parse command line arguments
    let cli = Cli::parse();

    match cli.mode.as_str() {
        "send" => send_file(&cli).await?,
        "receive" => receive_file(&cli).await?,
        _ => {
            anyhow::bail!("Invalid mode. Use 'send' or 'receive'");
        }
    }

    Ok(())
}

async fn send_file(cli: &Cli) -> Result<()> {
    // Read the file
    let path = Path::new(&cli.file);
    let mut file = File::open(path).await?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).await?;

    // Calculate optimal block size based on MTU
    let block_size = 1400; // MTU (1500) - UDP header (28) - Our header (72)

    // Create encoder
    let mut encoder = Encoder::new(&contents, block_size)?;
    let source_blocks = (contents.len() + block_size - 1) / block_size;
    
    // Create transport
    let transport = fountainflow::transport::UdpTransport::new("0.0.0.0:0", cli.rate_limit).await?;

    // Send approximately 1.5x the number of source blocks for reliable decoding
    let target_blocks = source_blocks + (source_blocks / 2);
    let mut sequence = 0u32;
    
    println!("Sending {} blocks ({} bytes) to {}", target_blocks, contents.len(), cli.target);
    
    for _ in 0..target_blocks {
        let block = encoder.next_block()?;
        transport
            .send_block(&cli.target, block.data(), block.seed(), block.degree(), sequence)
            .await?;
        sequence = sequence.wrapping_add(1);

        if cli.verbose {
            println!(
                "Sent block {} of {} (degree: {}, size: {})",
                sequence,
                target_blocks,
                block.degree(),
                block.data().len()
            );
        }
    }
    
    println!("Finished sending {} blocks", target_blocks);
    Ok(())
}

async fn receive_file(cli: &Cli) -> Result<()> {
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;
    use std::time::Duration;
    
    // Create transport
    let transport = fountainflow::transport::UdpTransport::new(&format!("0.0.0.0:{}", cli.target), cli.rate_limit).await?;
    
    println!("Listening on port {}", cli.target);
    
    // We'll determine block size and count from the first received block
    let mut decoder = None;
    let mut received_count = 0;
    let start_time = std::time::Instant::now();
    
    // Receive blocks for up to 30 seconds
    while start_time.elapsed() < Duration::from_secs(30) {
        let (data, seed, degree, sequence, _addr) = transport.receive_block().await?;
        received_count += 1;
        
        // Initialize decoder from first block
        if decoder.is_none() {
            let block_size = data.len();
            // Estimate source blocks based on block size (assuming typical file sizes)
            let estimated_blocks = 100; // Conservative estimate
            decoder = Some(Decoder::new(estimated_blocks, block_size)?);
            println!("Initialized decoder with block size {}", block_size);
        }
        
        if let Some(decoder) = decoder.as_mut() {
            let block = Block::new(data.to_vec(), seed, degree);
            decoder.add_block(block, sequence)?;
            
            // Try decoding periodically
            if received_count % 10 == 0 {
                if decoder.try_decode()? {
                    if let Some(decoded_data) = decoder.get_decoded_data() {
                        // Write decoded data to file
                        let mut file = File::create(&cli.file).await?;
                        file.write_all(&decoded_data).await?;
                        println!("Successfully decoded and saved {} bytes to {}", decoded_data.len(), cli.file);
                        return Ok(());
                    }
                }
            }
            
            if cli.verbose {
                println!("Received block {} (degree: {}, size: {})", sequence, degree, data.len());
            }
        }
    }
    
    anyhow::bail!("Failed to decode file within timeout")
}
