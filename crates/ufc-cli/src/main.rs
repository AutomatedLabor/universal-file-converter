mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// Universal File Converter — Convert files across formats offline.
#[derive(Parser)]
#[command(name = "ufc", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
    /// Output format (json, text)
    #[arg(long, global = true, default_value = "text")]
    output: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a single file
    Convert(commands::convert::ConvertArgs),
    /// Batch convert multiple files
    Batch(commands::batch::BatchArgs),
    /// Detect the format of a file
    Detect(commands::detect::DetectArgs),
    /// List supported formats and plugins
    List(commands::list::ListArgs),
    /// Show conversion history
    History(commands::history::HistoryArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level)))
        .init();

    match cli.command {
        Commands::Convert(args) => commands::convert::run(args).await,
        Commands::Batch(args) => commands::batch::run(args).await,
        Commands::Detect(args) => commands::detect::run(args).await,
        Commands::List(args) => commands::list::run(args).await,
        Commands::History(args) => commands::history::run(args).await,
    }
}
