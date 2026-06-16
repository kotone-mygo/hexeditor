#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMode {
    None,
    Char,
    Line,
    Block,
}

pub struct Cursor {
    pub offset: u64,
    pub sub_offset: u8,
    pub selection_anchor: Option<u64>,
    pub selection_sub_anchor: Option<u8>,
    pub selection_mode: SelectionMode,
    pub bytes_per_row: u64,
}

impl Cursor {
    pub fn new(bytes_per_row: u64) -> Self {
        Cursor {
            offset: 0,
            sub_offset: 0,
            selection_anchor: None,
            selection_sub_anchor: None,
            selection_mode: SelectionMode::None,
            bytes_per_row,
        }
    }

    pub fn move_by(&mut self, delta: i64, file_size: u64) {
        if file_size == 0 {
            return;
        }
        if delta >= 0 {
            let d = delta as u64;
            let max = file_size.saturating_sub(1);
            self.offset = self.offset.saturating_add(d).min(max);
        } else {
            let d = delta.unsigned_abs();
            self.offset = self.offset.saturating_sub(d);
        }
    }

    pub fn move_down(&mut self, file_size: u64) {
        let delta = self.bytes_per_row as i64;
        self.move_by(delta, file_size);
    }

    pub fn move_up(&mut self, file_size: u64) {
        let delta = self.bytes_per_row as i64;
        self.move_by(-delta, file_size);
    }

    pub fn selection_start(&self) -> Option<u64> {
        let anchor = self.selection_anchor?;
        Some(anchor.min(self.offset))
    }

    pub fn selection_end(&self) -> Option<u64> {
        let anchor = self.selection_anchor?;
        Some(anchor.max(self.offset))
    }

    pub fn in_selection(&self, offset: u64) -> bool {
        let Some(start) = self.selection_start() else {
            return false;
        };
        let Some(end) = self.selection_end() else {
            return false;
        };
        offset >= start && offset <= end
    }

    pub fn start_selection(&mut self, mode: SelectionMode) {
        self.selection_anchor = Some(self.offset);
        self.selection_sub_anchor = Some(self.sub_offset);
        self.selection_mode = mode;
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
        self.selection_sub_anchor = None;
        self.selection_mode = SelectionMode::None;
    }

    pub fn current_row_start(&self) -> u64 {
        if self.bytes_per_row == 0 {
            return self.offset;
        }
        (self.offset / self.bytes_per_row) * self.bytes_per_row
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_move_within_bounds() {
        let mut cursor = Cursor::new(16);
        cursor.move_by(10, 100);
        assert_eq!(cursor.offset, 10);
    }

    #[test]
    fn test_cursor_move_clamps_to_end() {
        let mut cursor = Cursor::new(16);
        cursor.offset = 95;
        cursor.move_by(10, 100);
        assert_eq!(cursor.offset, 99);
    }

    #[test]
    fn test_cursor_move_clamps_to_start() {
        let mut cursor = Cursor::new(16);
        cursor.offset = 0;
        cursor.move_by(-5, 100);
        assert_eq!(cursor.offset, 0);
    }

    #[test]
    fn test_selection_char_mode() {
        let mut cursor = Cursor::new(16);
        cursor.start_selection(SelectionMode::Char);
        assert_eq!(cursor.selection_anchor, Some(0));
        cursor.move_by(5, 100);
        assert_eq!(cursor.selection_anchor, Some(0));
        assert!(cursor.in_selection(3));
        assert!(cursor.in_selection(0));
        assert!(cursor.in_selection(5));
        assert!(!cursor.in_selection(10));
    }

    #[test]
    fn test_selection_clear() {
        let mut cursor = Cursor::new(16);
        cursor.start_selection(SelectionMode::Char);
        assert_eq!(cursor.selection_anchor, Some(0));
        cursor.clear_selection();
        assert_eq!(cursor.selection_anchor, None);
    }

    #[test]
    fn test_current_row_start() {
        let cursor = Cursor {
            offset: 20,
            bytes_per_row: 16,
            sub_offset: 0,
            selection_anchor: None,
            selection_sub_anchor: None,
            selection_mode: SelectionMode::None,
        };
        assert_eq!(cursor.current_row_start(), 16);
    }

    #[test]
    fn test_move_down_up() {
        let mut cursor = Cursor::new(16);
        cursor.move_down(100);
        assert_eq!(cursor.offset, 16);
        cursor.move_up(100);
        assert_eq!(cursor.offset, 0);
    }
}
