use anyhow::Result;
use clap::Args;
use colored::*;

#[derive(Args)]
pub struct ListArgs {
    /// Show plugins only
    #[arg(long)]
    pub plugins: bool,
    /// Show formats only
    #[arg(long)]
    pub formats: bool,
}

pub async fn run(args: ListArgs) -> Result<()> {
    let config = ufc_core::AppConfig::default();
    let mut orchestrator = ufc_core::Orchestrator::new(config)?;
    super::convert::register_plugins(&mut orchestrator);

    if args.plugins {
        println!("{}", "Registered Plugins:".green().bold());
        for conversion in orchestrator.supported_conversions() {
            println!("  {} → {}", conversion.0.display_name, conversion.1.display_name);
        }
    } else if args.formats {
        let conversions = orchestrator.supported_conversions();
        println!("{} ({} conversions)", "Supported Conversions:".green().bold(), conversions.len());
        for (src, tgt) in &conversions {
            println!("  {} → {}", src.display_name.cyan(), tgt.display_name.yellow());
        }
    } else {
        println!("{}", "Universal File Converter".green().bold());
        println!();
        println!("{}", "Supported conversions:".bold());
        let conversions = orchestrator.supported_conversions();
        for (src, tgt) in conversions.iter().take(50) {
            println!("  {} → {}", src.display_name.cyan(), tgt.display_name.yellow());
        }
        if conversions.len() > 50 {
            println!("  ... and {} more", (conversions.len() - 50).to_string().dimmed());
        }
    }

    Ok(())
}
