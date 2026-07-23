use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct FormatInfo {
    pub mime: String,
    pub extensions: Vec<String>,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversionRequest {
    pub input_path: String,
    pub output_path: String,
    pub target_format: String,
    pub quality: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRequest {
    pub input_paths: Vec<String>,
    pub output_dir: String,
    pub target_format: String,
    pub quality: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversionResult {
    pub success: bool,
    pub bytes_written: Option<u64>,
    pub checksum: Option<String>,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueStatus {
    pub total: usize,
    pub pending: usize,
    pub converting: usize,
    pub completed: usize,
    pub failed: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub input_path: String,
    pub output_path: String,
    pub source_format: String,
    pub target_format: String,
    pub success: bool,
    pub bytes_written: Option<u64>,
    pub duration_ms: Option<u64>,
    pub timestamp: String,
    pub error: Option<String>,
}

#[tauri::command]
pub fn detect_format(path: String) -> Result<FormatInfo, String> {
    let path = PathBuf::from(&path);
    let detector = ufc_core::FormatDetector::new();
    let header = read_header(&path, 16).map_err(|e| e.to_string())?;
    let format = detector.detect(&path, &header).map_err(|e| e.to_string())?;
    Ok(FormatInfo {
        mime: format.mime,
        extensions: format.extensions,
        display_name: format.display_name,
    })
}

#[tauri::command]
pub async fn convert_file(request: ConversionRequest) -> Result<ConversionResult, String> {
    let input = PathBuf::from(&request.input_path);
    let output = PathBuf::from(&request.output_path);
    let target_mime = format_to_mime(&request.target_format);

    let config = ufc_core::AppConfig::default();
    let mut orchestrator = ufc_core::Orchestrator::new(config).map_err(|e| e.to_string())?;
    register_plugins(&mut orchestrator);

    let start = std::time::Instant::now();
    let id = orchestrator.enqueue(input, output, &target_mime).map_err(|e| e.to_string())?;
    let stats = orchestrator.process_queue().await.map_err(|e| e.to_string())?;
    let duration = start.elapsed().as_millis() as u64;

    let item = orchestrator.queue().get(id).unwrap();
    if stats.completed > 0 {
        Ok(ConversionResult {
            success: true,
            bytes_written: item.bytes_written,
            checksum: item.output_checksum.clone(),
            error: None,
            duration_ms: Some(duration),
        })
    } else {
        Ok(ConversionResult {
            success: false,
            bytes_written: None,
            checksum: None,
            error: item.error.clone(),
            duration_ms: Some(duration),
        })
    }
}

#[tauri::command]
pub async fn batch_convert(request: BatchRequest) -> Result<Vec<ConversionResult>, String> {
    let inputs: Vec<PathBuf> = request.input_paths.iter().map(PathBuf::from).collect();
    let output_dir = PathBuf::from(&request.output_dir);
    let target_mime = format_to_mime(&request.target_format);

    let config = ufc_core::AppConfig::default();
    let mut orchestrator = ufc_core::Orchestrator::new(config).map_err(|e| e.to_string())?;
    register_plugins(&mut orchestrator);

    let start = std::time::Instant::now();
    let ids = orchestrator.enqueue_batch(inputs, &output_dir, &request.target_format, &target_mime)
        .map_err(|e| e.to_string())?;
    let _stats = orchestrator.process_queue().await.map_err(|e| e.to_string())?;
    let duration = start.elapsed().as_millis() as u64;

    let results = ids.iter().map(|id| {
        let item = orchestrator.queue().get(*id).unwrap();
        ConversionResult {
            success: item.status == ufc_core::QueueItemStatus::Completed,
            bytes_written: item.bytes_written,
            checksum: item.output_checksum.clone(),
            error: item.error.clone(),
            duration_ms: Some(duration / ids.len() as u64),
        }
    }).collect();

    Ok(results)
}

#[tauri::command]
pub fn list_formats() -> Result<Vec<FormatInfo>, String> {
    let config = ufc_core::AppConfig::default();
    let mut orchestrator = ufc_core::Orchestrator::new(config).map_err(|e| e.to_string())?;
    register_plugins(&mut orchestrator);

    let conversions = orchestrator.supported_conversions();
    let mut formats: Vec<FormatInfo> = conversions.into_iter()
        .map(|(src, _)| FormatInfo {
            mime: src.mime,
            extensions: src.extensions,
            display_name: src.display_name,
        })
        .collect();
    formats.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    formats.dedup_by(|a, b| a.mime == b.mime);
    Ok(formats)
}

#[tauri::command]
pub fn get_queue_status() -> Result<QueueStatus, String> {
    Ok(QueueStatus { total: 0, pending: 0, converting: 0, completed: 0, failed: 0 })
}

#[tauri::command]
pub fn cancel_conversion(id: String) -> Result<bool, String> {
    Ok(true)
}

#[tauri::command]
pub fn get_history() -> Result<Vec<HistoryEntry>, String> {
    let state_path = ufc_core::state::default_state_path();
    let state = ufc_core::StateManager::new(state_path);
    Ok(state.state().history.iter().map(|e| HistoryEntry {
        input_path: e.input_path.display().to_string(),
        output_path: e.output_path.display().to_string(),
        source_format: e.source_format.clone(),
        target_format: e.target_format.clone(),
        success: e.success,
        bytes_written: e.bytes_written,
        duration_ms: e.duration_ms,
        timestamp: e.timestamp.to_rfc3339(),
        error: e.error.clone(),
    }).collect())
}

#[tauri::command]
pub fn clear_history() -> Result<(), String> {
    let state_path = ufc_core::state::default_state_path();
    let mut state = ufc_core::StateManager::new(state_path);
    state.clear_history();
    state.save().map_err(|e| e.to_string())?;
    Ok(())
}

fn read_header(path: &std::path::Path, n: usize) -> Result<Vec<u8>, std::io::Error> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut header = vec![0u8; n];
    let bytes_read = file.read(&mut header)?;
    header.truncate(bytes_read);
    Ok(header)
}

fn format_to_mime(format: &str) -> String {
    match format.to_lowercase().as_str() {
        "png" => "image/png", "jpg" | "jpeg" => "image/jpeg", "webp" => "image/webp",
        "bmp" => "image/bmp", "tiff" | "tif" => "image/tiff", "gif" => "image/gif",
        "avif" => "image/avif", "ico" => "image/x-icon",
        "pdf" => "application/pdf", "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "html" | "htm" => "text/html", "md" | "markdown" => "text/markdown", "txt" => "text/plain",
        "wav" => "audio/wav", "flac" => "audio/flac", "mp3" => "audio/mpeg",
        "aac" | "m4a" => "audio/aac", "ogg" => "audio/ogg", "opus" => "audio/opus",
        "mp4" => "video/mp4", "webm" => "video/webm", "mkv" => "video/x-matroska",
        "zip" => "application/zip", "tar" => "application/x-tar", "gz" | "tgz" => "application/gzip",
        "csv" => "text/csv", "json" => "application/json", "xml" => "application/xml",
        "yaml" | "yml" => "application/x-yaml",
        _ => "application/octet-stream",
    }.to_string()
}

fn register_plugins(orchestrator: &mut ufc_core::Orchestrator) {
    use std::sync::Arc;
    orchestrator.register_plugin(Arc::new(core_image_png::PngPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_image_jpeg::JpegPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_image_webp::WebPPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_image_bmp::BmpPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_image_tiff::TiffPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_image_gif::GifPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_image_avif::AvifPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_image_ico::IcoPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_doc_pdf::PdfPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_doc_docx::DocxPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_doc_html::HtmlPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_doc_markdown::MarkdownPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_doc_rtf::RtfPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_audio_wav::WavPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_audio_flac::FlacPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_audio_mp3::Mp3Plugin::new()));
    orchestrator.register_plugin(Arc::new(core_audio_aac::AacPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_audio_vorbis::VorbisPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_audio_opus::OpusPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_archive_zip::ZipPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_archive_tar::TarPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_archive_7z::SevenZPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_struct_csv::CsvPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_struct_json::JsonPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_struct_xml::XmlPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_struct_yaml::YamlPlugin::new()));
    orchestrator.register_plugin(Arc::new(core_video_ffmpeg::FFmpegPlugin::new()));
}
