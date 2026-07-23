use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use colored::*;

#[derive(Args)]
pub struct DetectArgs {
    /// File to detect
    pub file: PathBuf,
}

pub async fn run(args: DetectArgs) -> Result<()> {
    if !args.file.exists() {
        anyhow::bail!("File not found: {}", args.file.display());
    }

    let detector = ufc_core::FormatDetector::new();
    let header = read_header(&args.file, 64)?;

    println!("{} {}", "Detecting format for".green().bold(), args.file.display());

    match detector.detect(&args.file, &header) {
        Ok(format) => {
            println!("  Format:    {}", format.display_name.cyan().bold());
            println!("  MIME:      {}", format.mime);
            println!("  Extension: {}", format.extensions.join(", "));
        }
        Err(e) => {
            println!("  {} {}", "Unknown format:".red().bold(), e);
        }
    }

    Ok(())
}

fn read_header(path: &std::path::Path, n: usize) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut header = vec![0u8; n];
    let bytes_read = file.read(&mut header)?;
    header.truncate(bytes_read);
    Ok(header)
}
