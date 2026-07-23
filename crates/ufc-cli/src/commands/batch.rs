use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use colored::*;

#[derive(Args)]
pub struct BatchArgs {
    /// Input files or glob pattern (e.g., "*.png", "photos/*.jpg")
    pub inputs: Vec<String>,
    /// Output directory
    #[arg(short, long)]
    pub output_dir: PathBuf,
    /// Target format
    #[arg(short, long)]
    pub format: String,
    /// Quality preset
    #[arg(short, long, default_value = "high")]
    pub quality: String,
    /// Overwrite existing files
    #[arg(long)]
    pub overwrite: bool,
    /// Number of concurrent conversions
    #[arg(short, long, default_value = "4")]
    pub concurrent: usize,
}

pub async fn run(args: BatchArgs) -> Result<()> {
    // Expand glob patterns
    let mut input_files = Vec::new();
    for pattern in &args.inputs {
        if pattern.contains('*') || pattern.contains('?') {
            for entry in glob::glob(pattern)? {
                match entry {
                    Ok(path) if path.is_file() => input_files.push(path),
                    Ok(_) => {},
                    Err(e) => eprintln!("{}: {}", "Glob error".red(), e),
                }
            }
        } else {
            let path = PathBuf::from(pattern);
            if path.is_file() {
                input_files.push(path);
            } else if path.is_dir() {
                for entry in walkdir::WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() {
                        input_files.push(entry.path().to_path_buf());
                    }
                }
            }
        }
    }

    if input_files.is_empty() {
        anyhow::bail!("No input files found");
    }

    // Create output directory
    std::fs::create_dir_all(&args.output_dir)?;

    println!("{} {} files → {}", "Batch converting".green().bold(), input_files.len(), args.output_dir.display());
    println!("  Target format: {}", args.format.yellow());
    println!("  Concurrency: {}", args.concurrent.to_string().yellow());

    let target_mime = super::convert::format_to_mime(&args.format);
    let config = ufc_core::AppConfig {
        max_concurrent: args.concurrent,
        overwrite_existing: args.overwrite,
        ..Default::default()
    };
    let mut orchestrator = ufc_core::Orchestrator::new(config)?;
    super::convert::register_plugins(&mut orchestrator);

    let ids = orchestrator.enqueue_batch(input_files, &args.output_dir, &args.format, &target_mime)?;
    let stats = orchestrator.process_queue().await?;

    println!("\n{}", "Results:".bold());
    println!("  {} completed", stats.completed.to_string().green());
    println!("  {} failed", stats.failed.to_string().red());
    println!("  {} skipped", stats.skipped.to_string().yellow());

    Ok(())
}
