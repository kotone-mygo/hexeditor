use std::io::Write;
use std::path::{Path, PathBuf};

pub struct ByteBuffer {
    data: Vec<u8>,
    path: Option<PathBuf>,
    modified: bool,
}

impl ByteBuffer {
    pub fn new(data: &[u8]) -> Self {
        let data = data.to_vec();
        ByteBuffer {
            data,
            path: None,
            modified: false,
        }
    }

    pub fn open(path: &Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| format!("Failed to open file: {}", e))?;
        Ok(ByteBuffer {
            data,
            path: Some(path.to_path_buf()),
            modified: false,
        })
    }

    pub fn save(&mut self) -> Result<(), String> {
        let path = self.path.as_ref().ok_or("No path set")?;
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let mut tmp = tempfile::NamedTempFile::new_in(dir)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        tmp.write_all(&self.data)
            .map_err(|e| format!("Failed to write: {}", e))?;
        tmp.persist(path)
            .map_err(|e| format!("Failed to persist: {}", e))?;
        self.modified = false;
        Ok(())
    }

    pub fn save_as(&mut self, path: &Path) -> Result<(), String> {
        self.path = Some(path.to_path_buf());
        self.save()
    }

    pub fn len(&self) -> u64 {
        self.data.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn read(&self, offset: u64, len: usize) -> Result<&[u8], String> {
        let size = self.data.len() as u64;
        if offset >= size {
            return Err("Offset out of bounds".to_string());
        }
        let end = offset
            .checked_add(len as u64)
            .ok_or("Overflow")?;
        if end > size {
            return Err("Read out of bounds".to_string());
        }
        Ok(&self.data[offset as usize..end as usize])
    }

    pub fn write(&mut self, offset: u64, bytes: &[u8]) -> Result<(), String> {
        let size = self.data.len() as u64;
        let end = offset
            .checked_add(bytes.len() as u64)
            .ok_or("Overflow")?;
        if end > size {
            return Err("Write out of bounds".to_string());
        }
        self.data[offset as usize..end as usize].copy_from_slice(bytes);
        self.modified = true;
        Ok(())
    }

    pub fn insert(&mut self, offset: u64, bytes: &[u8]) -> Result<(), String> {
        let size = self.data.len() as u64;
        if offset > size {
            return Err("Insert offset out of bounds".to_string());
        }
        let offset = offset as usize;
        let mut new_data = Vec::with_capacity(self.data.len() + bytes.len());
        new_data.extend_from_slice(&self.data[..offset]);
        new_data.extend_from_slice(bytes);
        new_data.extend_from_slice(&self.data[offset..]);
        self.data = new_data;
        self.modified = true;
        Ok(())
    }

    pub fn delete(&mut self, offset: u64, len: usize) -> Result<(), String> {
        let size = self.data.len() as u64;
        let end = offset
            .checked_add(len as u64)
            .ok_or("Overflow")?;
        if end > size {
            return Err("Delete out of bounds".to_string());
        }
        let offset = offset as usize;
        self.data.drain(offset..offset + len);
        self.modified = true;
        Ok(())
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn get_nibble(&self, byte_offset: u64, nibble: u8) -> Result<u8, String> {
        let bytes = self.read(byte_offset, 1).map_err(|e| e.to_string())?;
        let byte = bytes[0];
        Ok(match nibble {
            0 => byte >> 4,
            1 => byte & 0x0F,
            _ => return Err("nibble must be 0 or 1".to_string()),
        })
    }

    pub fn set_nibble(&mut self, byte_offset: u64, nibble: u8, digit: u8) -> Result<(), String> {
        let digit = digit & 0x0F;
        let byte = self.data[byte_offset as usize];
        self.data[byte_offset as usize] = match nibble {
            0 => (digit << 4) | (byte & 0x0F),
            1 => (byte & 0xF0) | digit,
            _ => return Err("nibble must be 0 or 1".to_string()),
        };
        self.modified = true;
        Ok(())
    }

    pub fn insert_nibble(&mut self, byte_offset: u64, nibble: u8, digit: u8) -> Result<(), String> {
        let digit = digit & 0x0F;
        let size = self.data.len();
        if byte_offset as usize > size {
            return Err("Insert offset out of bounds".to_string());
        }
        if byte_offset as usize == size {
            self.data.push(digit << 4);
            self.modified = true;
            return Ok(());
        }
        let off = byte_offset as usize;
        let orig_low = self.data[off] & 0x0F;
        if nibble == 0 {
            self.data[off] = (digit << 4) | (self.data[off] >> 4);
        } else {
            self.data[off] = (self.data[off] & 0xF0) | digit;
        }
        let mut carry = orig_low;
        for i in (off + 1)..self.data.len() {
            let b = self.data[i];
            self.data[i] = (carry << 4) | (b >> 4);
            carry = b & 0x0F;
        }
        self.modified = true;
        Ok(())
    }

    pub fn delete_nibble(&mut self, byte_offset: u64, nibble: u8) -> Result<(), String> {
        let size = self.data.len();
        if byte_offset as usize >= size {
            return Err("Delete offset out of bounds".to_string());
        }
        let off = byte_offset as usize;
        if nibble == 0 {
            for i in off..(size - 1) {
                let low = self.data[i] & 0x0F;
                let next_high = self.data[i + 1] >> 4;
                self.data[i] = (low << 4) | next_high;
            }
            self.data[size - 1] = (self.data[size - 1] & 0x0F) << 4;
        } else {
            if off + 1 < size {
                let next_high = self.data[off + 1] >> 4;
                self.data[off] = (self.data[off] & 0xF0) | next_high;
                for i in (off + 1)..(size - 1) {
                    let low = self.data[i] & 0x0F;
                    let next_high = self.data[i + 1] >> 4;
                    self.data[i] = (low << 4) | next_high;
                }
                self.data[size - 1] = (self.data[size - 1] & 0x0F) << 4;
            } else {
                self.data[off] &= 0xF0;
            }
        }
        self.modified = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_open_and_read_small_file() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(b"hello hex").unwrap();
        let buf = ByteBuffer::open(f.path()).unwrap();
        assert_eq!(buf.len(), 9);
        assert_eq!(buf.read(0, 9).unwrap(), b"hello hex");
    }

    #[test]
    fn test_write_marks_modified() {
        let mut buf = ByteBuffer::new(b"hello");
        assert!(!buf.is_modified());
        buf.write(0, b"j").unwrap();
        assert!(buf.is_modified());
    }

    #[test]
    fn test_insert_shifts_bytes() {
        let mut buf = ByteBuffer::new(b"ac");
        buf.insert(1, b"b").unwrap();
        assert_eq!(buf.read(0, 3).unwrap(), b"abc");
    }

    #[test]
    fn test_delete_removes_bytes() {
        let mut buf = ByteBuffer::new(b"abcd");
        buf.delete(1, 2).unwrap();
        assert_eq!(buf.read(0, 2).unwrap(), b"ad");
    }

    #[test]
    fn test_read_out_of_bounds_error() {
        let buf = ByteBuffer::new(b"hi");
        assert!(buf.read(0, 3).is_err());
        assert!(buf.read(5, 1).is_err());
    }

    #[test]
    fn test_get_set_nibble() {
        let mut buf = ByteBuffer::new(b"\xAB\xCD");
        assert_eq!(buf.get_nibble(0, 0).unwrap(), 0xA);
        assert_eq!(buf.get_nibble(0, 1).unwrap(), 0xB);
        assert_eq!(buf.get_nibble(1, 0).unwrap(), 0xC);
        assert_eq!(buf.get_nibble(1, 1).unwrap(), 0xD);
        buf.set_nibble(0, 0, 0x3).unwrap();
        assert_eq!(buf.get_nibble(0, 0).unwrap(), 0x3);
        assert_eq!(buf.get_nibble(0, 1).unwrap(), 0xB);
        assert_eq!(buf.read(0, 1).unwrap(), b"\x3B");
        buf.set_nibble(0, 1, 0xE).unwrap();
        assert_eq!(buf.read(0, 1).unwrap(), b"\x3E");
        assert!(buf.is_modified());
    }

    #[test]
    fn test_insert_nibble_high() {
        let mut buf = ByteBuffer::new(b"\xAB\xCD");
        buf.insert_nibble(0, 0, 0x9).unwrap();
        assert_eq!(buf.read(0, 2).unwrap(), b"\x9A\xBC");
    }

    #[test]
    fn test_insert_nibble_low() {
        let mut buf = ByteBuffer::new(b"\xAB\xCD");
        buf.insert_nibble(0, 1, 0x9).unwrap();
        assert_eq!(buf.read(0, 2).unwrap(), b"\xA9\xBC");
    }

    #[test]
    fn test_insert_nibble_end() {
        let mut buf = ByteBuffer::new(b"\xAB");
        buf.insert_nibble(1, 0, 0x9).unwrap();
        assert_eq!(buf.read(0, 2).unwrap(), b"\xAB\x90");
    }

    #[test]
    fn test_insert_nibble_at_buffer_end() {
        let mut buf = ByteBuffer::new(b"\xAB");
        buf.insert_nibble(1, 0, 0xC).unwrap();
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.read(0, 2).unwrap(), b"\xAB\xC0");
    }

    #[test]
    fn test_delete_nibble_high() {
        let mut buf = ByteBuffer::new(b"\xAB\xCD");
        buf.delete_nibble(0, 0).unwrap();
        assert_eq!(buf.read(0, 2).unwrap(), b"\xBC\xD0");
    }

    #[test]
    fn test_delete_nibble_low() {
        let mut buf = ByteBuffer::new(b"\xAB\xCD");
        buf.delete_nibble(0, 1).unwrap();
        assert_eq!(buf.read(0, 2).unwrap(), b"\xAC\xD0");
    }

    #[test]
    fn test_delete_nibble_single_byte_high() {
        let mut buf = ByteBuffer::new(b"\xA0");
        buf.delete_nibble(0, 0).unwrap();
        assert_eq!(buf.read(0, 1).unwrap(), b"\x00");
        assert_eq!(buf.len(), 1);
    }

    #[test]
    fn test_delete_nibble_single_byte_low() {
        let mut buf = ByteBuffer::new(b"\xAB");
        buf.delete_nibble(0, 1).unwrap();
        assert_eq!(buf.read(0, 1).unwrap(), b"\xA0");
        assert_eq!(buf.len(), 1);
    }

    #[test]
    fn test_save_writes_to_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.bin");
        let mut buf = ByteBuffer::new(b"save test data");
        buf.save_as(&path).unwrap();
        let loaded = ByteBuffer::open(&path).unwrap();
        assert_eq!(loaded.len(), 14);
        assert_eq!(loaded.read(0, 14).unwrap(), b"save test data");
    }
}
