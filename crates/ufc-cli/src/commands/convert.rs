use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use colored::*;

#[derive(Args)]
pub struct ConvertArgs {
    /// Input file path
    pub input: PathBuf,
    /// Output file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Target format (e.g., "png", "jpg", "pdf", "mp3")
    #[arg(short, long)]
    pub format: String,
    /// Quality preset (lossless, high, medium, low)
    #[arg(short, long, default_value = "high")]
    pub quality: String,
    /// Overwrite existing output file
    #[arg(long)]
    pub overwrite: bool,
    /// Skip integrity verification
    #[arg(long)]
    pub no_verify: bool,
}

pub async fn run(args: ConvertArgs) -> Result<()> {
    let input = &args.input;
    if !input.exists() {
        anyhow::bail!("Input file not found: {}", input.display());
    }

    // Determine output path
    let output = match &args.output {
        Some(o) => o.clone(),
        None => {
            let stem = input.file_stem().unwrap_or_default().to_string_lossy();
            let ext = &args.format;
            input.parent().unwrap_or(std::path::Path::new(".")).join(format!("{}.{}", stem, ext))
        }
    };

    if output.exists() && !args.overwrite {
        anyhow::bail!("Output file already exists: {} (use --overwrite to force)", output.display());
    }

    println!("{} {} → {}", "Converting".green().bold(), input.display(), output.display());
    println!("  Target format: {}", args.format.yellow());
    println!("  Quality: {}", args.quality.yellow());

    // Detect source format
    let detector = ufc_core::FormatDetector::new();
    let header = read_header(input, 16)?;
    let source_format = detector.detect(input, &header)?;
    println!("  Detected: {} ({})", source_format.display_name.cyan(), source_format.mime);

    // Map target format to MIME type
    let target_mime = format_to_mime(&args.format);

    // Create orchestrator and run conversion
    let config = ufc_core::AppConfig::default();
    let mut orchestrator = ufc_core::Orchestrator::new(config)?;

    // Register built-in plugins
    register_plugins(&mut orchestrator);

    // Enqueue and process
    let id = orchestrator.enqueue(input.clone(), output.clone(), &target_mime)?;
    let stats = orchestrator.process_queue().await?;

    if stats.completed > 0 {
        let item = orchestrator.queue().get(id).unwrap();
        println!("{} Conversion complete!", "✓".green().bold());
        if let Some(bytes) = item.bytes_written {
            println!("  Output size: {}", format_size(bytes));
        }
        if let Some(ref checksum) = item.output_checksum {
            println!("  Checksum: {}", checksum.dimmed());
        }
    } else {
        let item = orchestrator.queue().get(id).unwrap();
        let error = item.error.as_deref().unwrap_or("Unknown error");
        println!("{} Conversion failed: {}", "✗".red().bold(), error);
        std::process::exit(1);
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

fn format_to_mime(format: &str) -> String {
    match format.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",
        "gif" => "image/gif",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "html" | "htm" => "text/html",
        "md" | "markdown" => "text/markdown",
        "txt" => "text/plain",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "mp3" => "audio/mpeg",
        "aac" | "m4a" => "audio/aac",
        "ogg" => "audio/ogg",
        "opus" => "audio/opus",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" | "tgz" => "application/gzip",
        "csv" => "text/csv",
        "json" => "application/json",
        "xml" => "application/xml",
        "yaml" | "yml" => "application/x-yaml",
        _ => "application/octet-stream",
    }.to_string()
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 { format!("{} B", bytes) }
    else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else if bytes < 1024 * 1024 * 1024 { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
    else { format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) }
}

fn register_plugins(orchestrator: &mut ufc_core::Orchestrator) {
    use std::sync::Arc;
    // Register all built-in plugins
    orchestrator.register_plugin(Arc::new(plugins_core_image_png::PngPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_image_jpeg::JpegPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_image_webp::WebPPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_image_bmp::BmpPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_image_tiff::TiffPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_image_gif::GifPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_image_avif::AvifPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_image_ico::IcoPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_doc_pdf::PdfPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_doc_docx::DocxPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_doc_html::HtmlPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_doc_markdown::MarkdownPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_doc_rtf::RtfPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_audio_wav::WavPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_audio_flac::FlacPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_audio_mp3::Mp3Plugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_audio_aac::AacPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_audio_vorbis::VorbisPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_audio_opus::OpusPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_archive_zip::ZipPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_archive_tar::TarPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_archive_7z::SevenZPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_struct_csv::CsvPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_struct_json::JsonPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_struct_xml::XmlPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_struct_yaml::YamlPlugin::new()));
    orchestrator.register_plugin(Arc::new(plugins_core_video_ffmpeg::FFmpegPlugin::new()));
}
