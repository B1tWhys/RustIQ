use num_complex::Complex;
use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::Path;

/// Abstraction for IQ sample sources (hardware SDR, files, mock data)
#[allow(dead_code)]
pub trait IqSource {
    /// Read IQ samples into the provided buffer.
    /// Returns the number of samples read, or an error.
    fn read_samples(&mut self, buf: &mut [Complex<f32>]) -> io::Result<usize>;
}

/// File-based IQ source that reads binary IQ samples from disk.
/// Expects interleaved I/Q samples as little-endian f32 pairs.
#[derive(Debug)]
#[allow(dead_code)]
pub struct FileIqSource {
    file: File,
    loop_on_eof: bool,
}

#[allow(dead_code)]
impl FileIqSource {
    /// Create a new FileIqSource from a path.
    /// If loop_on_eof is true, the file will restart from the beginning on EOF.
    pub fn new<P: AsRef<Path>>(path: P, loop_on_eof: bool) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self { file, loop_on_eof })
    }
}

impl IqSource for FileIqSource {
    fn read_samples(&mut self, buf: &mut [Complex<f32>]) -> io::Result<usize> {
        // Each complex sample is 2 f32 values (I and Q)
        let bytes_needed = buf.len() * 2 * std::mem::size_of::<f32>();
        let mut byte_buf = vec![0u8; bytes_needed];

        let mut total_read = 0;

        loop {
            match self.file.read(&mut byte_buf[total_read..]) {
                Ok(0) => {
                    // EOF
                    if self.loop_on_eof && total_read == 0 {
                        // Restart from beginning
                        self.file.seek(std::io::SeekFrom::Start(0))?;
                        continue;
                    } else {
                        // Convert bytes read so far to samples
                        break;
                    }
                }
                Ok(n) => {
                    total_read += n;
                    if total_read >= bytes_needed {
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }

        // Convert bytes to Complex<f32> samples
        let samples_read = total_read / (2 * std::mem::size_of::<f32>());

        for i in 0..samples_read {
            let byte_offset = i * 2 * std::mem::size_of::<f32>();
            let i_bytes = &byte_buf[byte_offset..byte_offset + 4];
            let q_bytes = &byte_buf[byte_offset + 4..byte_offset + 8];

            let i_val = f32::from_le_bytes([i_bytes[0], i_bytes[1], i_bytes[2], i_bytes[3]]);
            let q_val = f32::from_le_bytes([q_bytes[0], q_bytes[1], q_bytes[2], q_bytes[3]]);

            buf[i] = Complex::new(i_val, q_val);
        }

        Ok(samples_read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper function to create a temp file with IQ samples
    fn create_iq_file(samples: &[f32]) -> NamedTempFile {
        let mut temp_file = NamedTempFile::new().unwrap();
        for &val in samples {
            temp_file.write_all(&val.to_le_bytes()).unwrap();
        }
        temp_file.flush().unwrap();
        temp_file
    }

    #[test]
    fn test_file_iq_source_reads_samples() {
        // 4 complex samples (I,Q pairs as f32)
        let temp_file = create_iq_file(&[
            1.0, 0.0,   // Sample 1: 1+0j
            0.0, 1.0,   // Sample 2: 0+1j
            -1.0, 0.0,  // Sample 3: -1+0j
            0.0, -1.0,  // Sample 4: 0-1j
        ]);

        let mut source = FileIqSource::new(temp_file.path(), false).unwrap();
        let mut buf = vec![Complex::new(0.0, 0.0); 4];

        let n = source.read_samples(&mut buf).unwrap();

        assert_eq!(n, 4);
        assert_eq!(buf, vec![
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 1.0),
            Complex::new(-1.0, 0.0),
            Complex::new(0.0, -1.0),
        ]);
    }

    #[test]
    fn test_file_iq_source_eof_no_loop() {
        let temp_file = create_iq_file(&[1.0, 0.0, 0.0, 1.0]);

        let mut source = FileIqSource::new(temp_file.path(), false).unwrap();
        let mut buf = vec![Complex::new(0.0, 0.0); 2];

        // First read should succeed
        let n = source.read_samples(&mut buf).unwrap();
        assert_eq!(n, 2);
        assert_eq!(buf, vec![
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 1.0),
        ]);

        // Second read should return 0 (EOF)
        let n = source.read_samples(&mut buf).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn test_file_iq_source_eof_with_loop() {
        let temp_file = create_iq_file(&[1.0, 2.0]);

        let mut source = FileIqSource::new(temp_file.path(), true).unwrap();
        let mut buf = vec![Complex::new(0.0, 0.0); 1];

        // First read
        let n = source.read_samples(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf, vec![Complex::new(1.0, 2.0)]);

        // Second read should loop back to start
        let n = source.read_samples(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf, vec![Complex::new(1.0, 2.0)]);
    }

    #[test]
    fn test_file_iq_source_file_not_found() {
        let result = FileIqSource::new("/nonexistent/path/to/file.iq", false);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_file_iq_source_partial_read() {
        // Create file with 3 samples, but try to read 5
        let temp_file = create_iq_file(&[
            1.0, 0.0,   // Sample 1
            2.0, 0.0,   // Sample 2
            3.0, 0.0,   // Sample 3
        ]);

        let mut source = FileIqSource::new(temp_file.path(), false).unwrap();
        let mut buf = vec![Complex::new(0.0, 0.0); 5];

        let n = source.read_samples(&mut buf).unwrap();

        // Should only read 3 samples
        assert_eq!(n, 3);
        assert_eq!(buf[0], Complex::new(1.0, 0.0));
        assert_eq!(buf[1], Complex::new(2.0, 0.0));
        assert_eq!(buf[2], Complex::new(3.0, 0.0));
        // Remaining buffer elements should be unchanged
        assert_eq!(buf[3], Complex::new(0.0, 0.0));
        assert_eq!(buf[4], Complex::new(0.0, 0.0));
    }
}
