use crate::buffer::ByteBuffer;
use crate::commands::EditCommand;

pub struct UndoManager {
    undo_stack: Vec<EditCommand>,
    redo_stack: Vec<EditCommand>,
    max_depth: usize,
}

impl UndoManager {
    pub fn new(max_depth: usize) -> Self {
        UndoManager {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
        }
    }

    pub fn push(&mut self, cmd: EditCommand) {
        self.redo_stack.clear();
        self.undo_stack.push(cmd);
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self, buf: &mut ByteBuffer) -> Result<u64, String> {
        let cmd = self.undo_stack.pop().ok_or("Nothing to undo")?;
        let offset = cmd.offset();
        cmd.undo(buf)?;
        self.redo_stack.push(cmd);
        Ok(offset)
    }

    pub fn redo(&mut self, buf: &mut ByteBuffer) -> Result<u64, String> {
        let cmd = self.redo_stack.pop().ok_or("Nothing to redo")?;
        let offset = cmd.offset();
        cmd.apply(buf)?;
        self.undo_stack.push(cmd);
        Ok(offset)
    }

    pub fn set_max_depth(&mut self, max_depth: usize) {
        self.max_depth = max_depth;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_redo_cycle() {
        let mut um = UndoManager::new(100);
        let mut buf = ByteBuffer::new(b"abcd");
        let cmd = EditCommand::Insert { offset: 0, bytes: b"XY".to_vec() };
        cmd.apply(&mut buf).unwrap();
        um.push(cmd);
        assert_eq!(buf.len(), 6);
        assert_eq!(um.undo(&mut buf).unwrap(), 0);
        assert_eq!(buf.read(0, 4).unwrap(), b"abcd");
        assert_eq!(um.redo(&mut buf).unwrap(), 0);
        assert_eq!(buf.read(0, 6).unwrap(), b"XYabcd");
    }

    #[test]
    fn test_new_action_clears_redo() {
        let mut um = UndoManager::new(100);
        let mut buf = ByteBuffer::new(b"test");
        let cmd = EditCommand::Overwrite { offset: 0, old_bytes: b"t".to_vec(), new_bytes: b"T".to_vec() };
        cmd.apply(&mut buf).unwrap();
        um.push(cmd);
        um.undo(&mut buf).unwrap();
        assert!(um.can_redo());
        let cmd2 = EditCommand::Overwrite { offset: 1, old_bytes: b"e".to_vec(), new_bytes: b"E".to_vec() };
        cmd2.apply(&mut buf).unwrap();
        um.push(cmd2);
        assert!(!um.can_redo());
    }
}
