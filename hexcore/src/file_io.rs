use std::path::Path;
use crate::buffer::ByteBuffer;

pub struct FileIo;

impl FileIo {
    pub fn detect_encoding(data: &[u8]) -> &'static str {
        if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xFE {
            "UTF-16LE"
        } else if std::str::from_utf8(data).is_ok() {
            "UTF-8"
        } else {
            "Binary"
        }
    }

    pub fn open(path: &Path) -> Result<(ByteBuffer, &'static str), String> {
        let buf = ByteBuffer::open(path)?;
        let header = buf.read(0, 4.min(buf.len() as usize)).unwrap_or(b"");
        let encoding = Self::detect_encoding(header);
        Ok((buf, encoding))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_utf8() {
        assert_eq!(FileIo::detect_encoding(b"hello"), "UTF-8");
    }

    #[test]
    fn test_detect_utf16le() {
        assert_eq!(
            FileIo::detect_encoding(&[0xFF, 0xFE, 0x48, 0x00]),
            "UTF-16LE"
        );
    }

    #[test]
    fn test_detect_binary() {
        assert_eq!(FileIo::detect_encoding(&[0x00, 0xFF, 0xFE]), "Binary");
    }

    #[test]
    fn test_open_utf8_file() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut f, b"hello").unwrap();
        let (buf, enc) = FileIo::open(f.path()).unwrap();
        assert_eq!(enc, "UTF-8");
        assert_eq!(buf.len(), 5);
    }

    #[test]
    fn test_open_utf16le_file() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut f, &[0xFF, 0xFE, 0x48, 0x00]).unwrap();
        let (buf, enc) = FileIo::open(f.path()).unwrap();
        assert_eq!(enc, "UTF-16LE");
        assert_eq!(buf.len(), 4);
    }

    #[test]
    fn test_open_binary_file() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut f, &[0x00, 0xFF, 0xFE]).unwrap();
        let (buf, enc) = FileIo::open(f.path()).unwrap();
        assert_eq!(enc, "Binary");
        assert_eq!(buf.len(), 3);
    }
}
