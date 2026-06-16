use crate::buffer::ByteBuffer;

pub enum EditCommand {
    Overwrite { offset: u64, old_bytes: Vec<u8>, new_bytes: Vec<u8> },
    Insert { offset: u64, bytes: Vec<u8> },
    Delete { offset: u64, bytes: Vec<u8> },
}

impl EditCommand {
    pub fn offset(&self) -> u64 {
        match self {
            EditCommand::Overwrite { offset, .. }
            | EditCommand::Insert { offset, .. }
            | EditCommand::Delete { offset, .. } => *offset,
        }
    }

    pub fn apply(&self, buf: &mut ByteBuffer) -> Result<(), String> {
        match self {
            EditCommand::Overwrite { offset, old_bytes: _, new_bytes } => {
                buf.write(*offset, new_bytes)
            }
            EditCommand::Insert { offset, bytes } => {
                buf.insert(*offset, bytes)
            }
            EditCommand::Delete { offset, bytes } => {
                buf.delete(*offset, bytes.len())
            }
        }
    }

    pub fn undo(&self, buf: &mut ByteBuffer) -> Result<(), String> {
        match self {
            EditCommand::Overwrite { offset, old_bytes, new_bytes: _ } => {
                buf.write(*offset, old_bytes)
            }
            EditCommand::Insert { offset, bytes } => {
                buf.delete(*offset, bytes.len())
            }
            EditCommand::Delete { offset, bytes } => {
                buf.insert(*offset, bytes)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overwrite_undo_restores_original() {
        let cmd = EditCommand::Overwrite {
            offset: 0,
            old_bytes: b"H".to_vec(),
            new_bytes: b"X".to_vec(),
        };
        let mut buf = ByteBuffer::new(b"Hello");
        cmd.apply(&mut buf).unwrap();
        assert_eq!(buf.read(0, 1).unwrap(), b"X");
        cmd.undo(&mut buf).unwrap();
        assert_eq!(buf.read(0, 1).unwrap(), b"H");
    }

    #[test]
    fn test_insert_undo_removes_bytes() {
        let cmd = EditCommand::Insert { offset: 1, bytes: b"XY".to_vec() };
        let mut buf = ByteBuffer::new(b"ab");
        cmd.apply(&mut buf).unwrap();
        assert_eq!(buf.len(), 4);
        cmd.undo(&mut buf).unwrap();
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.read(0, 2).unwrap(), b"ab");
    }

    #[test]
    fn test_delete_undo_restores_bytes() {
        let cmd = EditCommand::Delete { offset: 1, bytes: b"bc".to_vec() };
        let mut buf = ByteBuffer::new(b"abcd");
        cmd.apply(&mut buf).unwrap();
        assert_eq!(buf.len(), 2);
        cmd.undo(&mut buf).unwrap();
        assert_eq!(buf.len(), 4);
        assert_eq!(buf.read(0, 4).unwrap(), b"abcd");
    }
}
