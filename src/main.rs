use anyhow::Result;
use clap::Parser;
use fountainflow::{Cli, Encoder};
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

    // Create transport
    let transport = fountainflow::transport::UdpTransport::new("0.0.0.0:0", cli.rate_limit).await?;

    // Start sending blocks
    let mut sequence = 0u32;
    loop {
        let block = encoder.next_block();
        transport
            .send_block(&cli.target, block.data(), block.seed(), block.degree(), sequence)
            .await?;
        sequence = sequence.wrapping_add(1);

        if cli.verbose {
            println!(
                "Sent block {} (degree: {}, size: {})",
                sequence,
                block.degree(),
                block.data().len()
            );
        }
    }
}

async fn receive_file(cli: &Cli) -> Result<()> {
    // Create transport
    let transport = fountainflow::transport::UdpTransport::new(&format!("0.0.0.0:{}", cli.target), cli.rate_limit).await?;

    // TODO: Implement decoder
    println!("Receiving not yet implemented");

    Ok(())
}
