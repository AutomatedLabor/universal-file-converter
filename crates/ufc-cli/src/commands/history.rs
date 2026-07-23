use anyhow::Result;
use clap::Args;
use colored::*;

#[derive(Args)]
pub struct HistoryArgs {
    /// Search query (filename, format)
    pub query: Option<String>,
    /// Show last N entries
    #[arg(short, long, default_value = "20")]
    pub limit: usize,
    /// Clear history
    #[arg(long)]
    pub clear: bool,
}

pub async fn run(args: HistoryArgs) -> Result<()> {
    let state_path = ufc_core::state::default_state_path();
    let mut state = ufc_core::StateManager::new(state_path);

    if args.clear {
        state.clear_history();
        state.save()?;
        println!("{}", "History cleared.".green().bold());
        return Ok(());
    }

    let entries = match &args.query {
        Some(q) => state.search_history(q),
        None => {
            let history = &state.state().history;
            history.iter().collect()
        }
    };

    if entries.is_empty() {
        println!("{}", "No conversion history found.".dimmed());
        return Ok(());
    }

    println!("{} (showing last {})", "Conversion History:".green().bold(), args.limit);
    println!();

    for entry in entries.iter().rev().take(args.limit) {
        let status = if entry.success { "✓".green() } else { "✗".red() };
        let time = entry.timestamp.format("%Y-%m-%d %H:%M:%S");
        println!("  {} {} → {}", status, entry.input_path.display().to_string().cyan(), entry.output_path.display().to_string().yellow());
        println!("    {} → {} | {}", entry.source_format, entry.target_format, time.to_string().dimmed());
        if let Some(ref err) = entry.error {
            println!("    Error: {}", err.red());
        }
        if let Some(bytes) = entry.bytes_written {
            println!("    Size: {}", format_size(bytes));
        }
        if let Some(ms) = entry.duration_ms {
            println!("    Duration: {}", format_duration(ms));
        }
        println!();
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{} B", bytes) }
    else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else if bytes < 1024 * 1024 * 1024 { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
    else { format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) }
}

fn format_duration(ms: u64) -> String {
    if ms < 1000 { format!("{}ms", ms) }
    else if ms < 60_000 { format!("{:.1}s", ms as f64 / 1000.0) }
    else { format!("{}m {}s", ms / 60_000, (ms % 60_000) / 1000) }
}
