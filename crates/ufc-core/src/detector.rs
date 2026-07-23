use std::collections::HashMap;
use std::path::Path;
use ufc_plugin_api::{FormatId, ProbeResult};
use crate::error::CoreError;

/// Format detection using magic bytes, file extensions, and MIME types.
pub struct FormatDetector {
    /// Magic byte signatures: (offset, bytes, format_id)
    magic_signatures: Vec<(usize, Vec<u8>, FormatId)>,
    /// Extension → format mapping
    extension_map: HashMap<String, FormatId>,
}

impl FormatDetector {
    pub fn new() -> Self {
        let mut detector = Self {
            magic_signatures: Vec::new(),
            extension_map: HashMap::new(),
        };
        detector.register_built_in_formats();
        detector
    }

    /// Detect the format of a file by reading its magic bytes and extension.
    pub fn detect(&self, path: &Path, header: &[u8]) -> Result<FormatId, CoreError> {
        // 1. Try magic bytes first (highest confidence)
        if let Some(format) = self.detect_by_magic(header) {
            return Ok(format);
        }

        // 2. Fall back to file extension
        if let Some(format) = self.detect_by_extension(path) {
            return Ok(format);
        }

        Err(CoreError::DetectionFailed {
            reason: format!(
                "Could not detect format for: {}",
                path.display()
            ),
        })
    }

    /// Detect format by magic bytes.
    pub fn detect_by_magic(&self, header: &[u8]) -> Option<FormatId> {
        for (offset, signature, format) in &self.magic_signatures {
            if header.len() >= offset + signature.len()
                && &header[*offset..*offset + signature.len()] == signature.as_slice()
            {
                return Some(format.clone());
            }
        }
        None
    }

    /// Detect format by file extension.
    pub fn detect_by_extension(&self, path: &Path) -> Option<FormatId> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        self.extension_map.get(&ext).cloned()
    }

    /// Register a format's magic bytes.
    pub fn register_magic(&mut self, offset: usize, signature: Vec<u8>, format: FormatId) {
        self.magic_signatures.push((offset, signature, format));
    }

    /// Register a format's file extensions.
    pub fn register_extensions(&mut self, format: &FormatId) {
        for ext in &format.extensions {
            self.extension_map.insert(ext.to_lowercase(), format.clone());
        }
    }

    /// Register all built-in format signatures.
    fn register_built_in_formats(&mut self) {
        // ── Images ──
        let png = FormatId::new("image/png", &["png"], "PNG");
        self.register_magic(0, vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A], png.clone());
        self.register_extensions(&png);

        let jpeg = FormatId::new("image/jpeg", &["jpg", "jpeg"], "JPEG");
        self.register_magic(0, vec![0xFF, 0xD8, 0xFF], jpeg.clone());
        self.register_extensions(&jpeg);

        let gif87 = FormatId::new("image/gif", &["gif"], "GIF");
        self.register_magic(0, b"GIF87a".to_vec(), gif87.clone());
        self.register_magic(0, b"GIF89a".to_vec(), gif87.clone());
        self.register_extensions(&gif87);

        let bmp = FormatId::new("image/bmp", &["bmp"], "BMP");
        self.register_magic(0, b"BM".to_vec(), bmp.clone());
        self.register_extensions(&bmp);

        let webp = FormatId::new("image/webp", &["webp"], "WebP");
        self.register_magic(0, b"RIFF".to_vec(), webp.clone()); // need additional check for WEBP at offset 8
        self.register_extensions(&webp);

        let tiff_le = FormatId::new("image/tiff", &["tiff", "tif"], "TIFF");
        self.register_magic(0, b"II\x2A\x00".to_vec(), tiff_le.clone());
        self.register_magic(0, b"MM\x00\x2A".to_vec(), tiff_le.clone());
        self.register_extensions(&tiff_le);

        let ico = FormatId::new("image/x-icon", &["ico"], "ICO");
        self.register_magic(0, vec![0x00, 0x00, 0x01, 0x00], ico.clone());
        self.register_extensions(&ico);

        let avif = FormatId::new("image/avif", &["avif"], "AVIF");
        self.register_extensions(&avif); // no simple magic, detect by extension

        let qoi = FormatId::new("image/qoi", &["qoi"], "QOI");
        self.register_magic(0, b"qoif".to_vec(), qoi.clone());
        self.register_extensions(&qoi);

        // ── Audio ──
        let wav = FormatId::new("audio/wav", &["wav"], "WAV");
        self.register_magic(0, b"RIFF".to_vec(), wav.clone());
        self.register_extensions(&wav);

        let flac = FormatId::new("audio/flac", &["flac"], "FLAC");
        self.register_magic(0, b"fLaC".to_vec(), flac.clone());
        self.register_extensions(&flac);

        let mp3 = FormatId::new("audio/mpeg", &["mp3"], "MP3");
        self.register_magic(0, vec![0xFF, 0xFB], mp3.clone());
        self.register_magic(0, vec![0xFF, 0xF3], mp3.clone());
        self.register_magic(0, vec![0xFF, 0xF2], mp3.clone());
        self.register_magic(0, b"ID3".to_vec(), mp3.clone());
        self.register_extensions(&mp3);

        let ogg = FormatId::new("audio/ogg", &["ogg", "oga", "opus"], "OGG");
        self.register_magic(0, b"OggS".to_vec(), ogg.clone());
        self.register_extensions(&ogg);

        let aac = FormatId::new("audio/aac", &["aac", "m4a"], "AAC");
        self.register_extensions(&aac);

        let aiff = FormatId::new("audio/aiff", &["aiff", "aif"], "AIFF");
        self.register_magic(0, b"FORM".to_vec(), aiff.clone());
        self.register_extensions(&aiff);

        // ── Video ──
        let mp4 = FormatId::new("video/mp4", &["mp4", "m4v"], "MP4");
        self.register_extensions(&mp4);

        let mkv = FormatId::new("video/x-matroska", &["mkv", "webm"], "Matroska/WebM");
        self.register_magic(0, vec![0x1A, 0x45, 0xDF, 0xA3], mkv.clone());
        self.register_extensions(&mkv);

        let avi = FormatId::new("video/x-msvideo", &["avi"], "AVI");
        self.register_magic(0, b"RIFF".to_vec(), avi.clone());
        self.register_extensions(&avi);

        // ── Documents ──
        let pdf = FormatId::new("application/pdf", &["pdf"], "PDF");
        self.register_magic(0, b"%PDF".to_vec(), pdf.clone());
        self.register_extensions(&pdf);

        let docx = FormatId::new("application/vnd.openxmlformats-officedocument.wordprocessingml.document", &["docx"], "DOCX");
        self.register_extensions(&docx);

        let html = FormatId::new("text/html", &["html", "htm"], "HTML");
        self.register_extensions(&html);

        let markdown = FormatId::new("text/markdown", &["md", "markdown"], "Markdown");
        self.register_extensions(&markdown);

        let rtf = FormatId::new("application/rtf", &["rtf"], "RTF");
        self.register_magic(0, b"{\\rtf".to_vec(), rtf.clone());
        self.register_extensions(&rtf);

        let txt = FormatId::new("text/plain", &["txt"], "Plain Text");
        self.register_extensions(&txt);

        // ── Archives ──
        let zip = FormatId::new("application/zip", &["zip", "jar", "docx", "xlsx", "pptx"], "ZIP");
        self.register_magic(0, b"PK\x03\x04".to_vec(), zip.clone());
        self.register_extensions(&zip);

        let gz = FormatId::new("application/gzip", &["gz", "tgz"], "Gzip");
        self.register_magic(0, vec![0x1F, 0x8B], gz.clone());
        self.register_extensions(&gz);

        let tar = FormatId::new("application/x-tar", &["tar"], "TAR");
        self.register_extensions(&tar);

        let sevenz = FormatId::new("application/x-7z-compressed", &["7z"], "7-Zip");
        self.register_magic(0, b"7z\xBC\xAF\x27\x1C".to_vec(), sevenz.clone());
        self.register_extensions(&sevenz);

        // ── Structured data ──
        let csv = FormatId::new("text/csv", &["csv"], "CSV");
        self.register_extensions(&csv);

        let json = FormatId::new("application/json", &["json"], "JSON");
        self.register_extensions(&json);

        let xml = FormatId::new("application/xml", &["xml"], "XML");
        self.register_extensions(&xml);

        let yaml = FormatId::new("application/x-yaml", &["yaml", "yml"], "YAML");
        self.register_extensions(&yaml);

        // ── Fonts ──
        let ttf = FormatId::new("font/ttf", &["ttf"], "TrueType");
        self.register_magic(0, vec![0x00, 0x01, 0x00, 0x00, 0x00], ttf.clone());
        self.register_extensions(&ttf);

        let otf = FormatId::new("font/otf", &["otf"], "OpenType");
        self.register_magic(0, b"OTTO".to_vec(), otf.clone());
        self.register_extensions(&otf);

        let woff = FormatId::new("font/woff", &["woff"], "WOFF");
        self.register_magic(0, b"wOFF".to_vec(), woff.clone());
        self.register_extensions(&woff);

        let woff2 = FormatId::new("font/woff2", &["woff2"], "WOFF2");
        self.register_magic(0, b"wOF2".to_vec(), woff2.clone());
        self.register_extensions(&woff2);
    }
}

impl Default for FormatDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_png() {
        let detector = FormatDetector::new();
        let header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let format = detector.detect_by_magic(&header).unwrap();
        assert_eq!(format.mime, "image/png");
    }

    #[test]
    fn test_detect_jpeg() {
        let detector = FormatDetector::new();
        let header = vec![0xFF, 0xD8, 0xFF, 0xE0];
        let format = detector.detect_by_magic(&header).unwrap();
        assert_eq!(format.mime, "image/jpeg");
    }

    #[test]
    fn test_detect_pdf() {
        let detector = FormatDetector::new();
        let header = b"%PDF-1.4".to_vec();
        let format = detector.detect_by_magic(&header).unwrap();
        assert_eq!(format.mime, "application/pdf");
    }

    #[test]
    fn test_detect_by_extension() {
        let detector = FormatDetector::new();
        let path = PathBuf::from("photo.jpg");
        let format = detector.detect_by_extension(&path).unwrap();
        assert_eq!(format.mime, "image/jpeg");
    }

    #[test]
    fn test_detect_zip() {
        let detector = FormatDetector::new();
        let header = b"PK\x03\x04".to_vec();
        let format = detector.detect_by_magic(&header).unwrap();
        assert_eq!(format.mime, "application/zip");
    }
}
