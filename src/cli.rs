//! Command-line interface

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Operation mode: 'send' or 'receive'
    #[arg(short, long)]
    pub mode: String,

    /// File path (source for send, destination for receive)
    #[arg(short, long)]
    pub file: String,

    /// Target address for send mode (e.g., "192.168.1.100:3000")
    /// or port for receive mode (e.g., "3000")
    #[arg(short, long)]
    pub target: String,

    /// Maximum transfer rate in Mbps
    #[arg(short, long, default_value = "1000")]
    pub rate_limit: u32,

    /// Enable verbose output
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,

    /// Disable checksum verification
    #[arg(long, default_value = "false")]
    pub no_checksum: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }

    #[test]
    fn cli_parse_send() {
        let cli = Cli::parse_from([
            "fountainflow",
            "--mode", "send",
            "--file", "test.txt",
            "--target", "192.168.1.100:3000",
            "--rate-limit", "500",
        ]);

        assert_eq!(cli.mode, "send");
        assert_eq!(cli.file, "test.txt");
        assert_eq!(cli.target, "192.168.1.100:3000");
        assert_eq!(cli.rate_limit, 500);
        assert!(!cli.verbose);
        assert!(!cli.no_checksum);
    }
}