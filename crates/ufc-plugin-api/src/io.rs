use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

/// Sandboxed file reader provided to plugins by the host.
///
/// Plugins never get direct filesystem access. Instead, the host provides
/// a `FileReader` that wraps the input file with resource tracking.
pub struct FileReader {
    inner: Box<dyn Read + Seek + Send>,
    path: PathBuf,
    size: u64,
    position: u64,
}

impl FileReader {
    /// Create a new FileReader (called by the host, not by plugins).
    pub fn new(
        reader: Box<dyn Read + Seek + Send>,
        path: PathBuf,
        size: u64,
    ) -> Self {
        Self {
            inner: reader,
            path,
            size,
            position: 0,
        }
    }

    /// Get the file path (for display purposes only).
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get the total file size in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get the current read position.
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Read bytes into the buffer. Returns the number of bytes read.
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.position += n as u64;
        Ok(n)
    }

    /// Read the entire file into a byte vector.
    pub fn read_all(&mut self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(self.size as usize);
        self.inner.read_to_end(&mut buf)?;
        self.position = self.size;
        Ok(buf)
    }

    /// Read exactly `n` bytes.
    pub fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.inner.read_exact(buf)?;
        self.position += buf.len() as u64;
        Ok(())
    }

    /// Seek to a position in the file.
    pub fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = self.inner.seek(pos)?;
        self.position = new_pos;
        Ok(new_pos)
    }

    /// Read a slice of bytes from the given offset and length.
    pub fn read_slice(&mut self, offset: u64, length: usize) -> io::Result<Vec<u8>> {
        self.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0u8; length];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }
}

/// Sandboxed file writer provided to plugins by the host.
///
/// Plugins never get direct filesystem access. Instead, the host provides
/// a `FileWriter` that wraps the output file with resource tracking.
pub struct FileWriter {
    inner: Box<dyn Write + Send>,
    path: PathBuf,
    bytes_written: u64,
}

impl FileWriter {
    /// Create a new FileWriter (called by the host, not by plugins).
    pub fn new(writer: Box<dyn Write + Send>, path: PathBuf) -> Self {
        Self {
            inner: writer,
            path,
            bytes_written: 0,
        }
    }

    /// Get the output file path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get the total bytes written so far.
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// Write bytes to the file. Returns the number of bytes written.
    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.bytes_written += n as u64;
        Ok(n)
    }

    /// Write all bytes to the file.
    pub fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.write_all(buf)?;
        self.bytes_written += buf.len() as u64;
        Ok(())
    }

    /// Flush the writer.
    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
