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

    fn row(&self, offset: u64) -> u64 {
        if self.bytes_per_row == 0 { return 0; }
        offset / self.bytes_per_row
    }

    pub fn selection_start(&self) -> Option<u64> {
        match self.selection_mode {
            SelectionMode::Line => {
                let anchor = self.selection_anchor?;
                let start_row = self.row(anchor).min(self.row(self.offset));
                Some(start_row * self.bytes_per_row)
            }
            _ => {
                let anchor = self.selection_anchor?;
                Some(anchor.min(self.offset))
            }
        }
    }

    pub fn selection_end(&self, file_size: u64) -> Option<u64> {
        match self.selection_mode {
            SelectionMode::Line => {
                let anchor = self.selection_anchor?;
                let end_row = self.row(anchor).max(self.row(self.offset));
                let end = end_row * self.bytes_per_row + self.bytes_per_row - 1;
                Some(end.min(file_size.saturating_sub(1)))
            }
            _ => {
                let anchor = self.selection_anchor?;
                Some(anchor.max(self.offset))
            }
        }
    }

    pub fn block_bounds(&self, nibble_mode: bool) -> (u64, u64, u64, u64) {
        let anchor = self.selection_anchor.unwrap_or(self.offset);
        let anchor_col = if nibble_mode {
            (anchor % self.bytes_per_row) * 2 + self.selection_sub_anchor.unwrap_or(0) as u64
        } else {
            anchor % self.bytes_per_row
        };
        let cursor_col = if nibble_mode {
            (self.offset % self.bytes_per_row) * 2 + self.sub_offset as u64
        } else {
            self.offset % self.bytes_per_row
        };
        let top = self.row(anchor).min(self.row(self.offset));
        let bottom = self.row(anchor).max(self.row(self.offset));
        let left = anchor_col.min(cursor_col);
        let right = anchor_col.max(cursor_col);
        (top, bottom, left, right)
    }

    pub fn in_selection(&self, offset: u64, file_size: u64, nibble_mode: bool, sub_offset: u8) -> bool {
        match self.selection_mode {
            SelectionMode::Block => {
                let (top, bottom, left, right) = self.block_bounds(nibble_mode);
                let off_row = self.row(offset);
                if off_row < top || off_row > bottom { return false; }
                if nibble_mode {
                    let off_col = (offset % self.bytes_per_row) * 2 + sub_offset as u64;
                    off_col >= left && off_col <= right
                } else {
                    let off_col = offset % self.bytes_per_row;
                    off_col >= left && off_col <= right
                }
            }
            SelectionMode::None => false,
            _ => {
                let Some(start) = self.selection_start() else { return false; };
                let Some(end) = self.selection_end(file_size) else { return false; };
                offset >= start && offset <= end
            }
        }
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

    pub fn current_row_end(&self, file_size: u64) -> u64 {
        if self.bytes_per_row == 0 || file_size == 0 {
            return self.offset;
        }
        let row_end = self.current_row_start() + self.bytes_per_row - 1;
        row_end.min(file_size - 1)
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
        assert!(cursor.in_selection(3, 100, false, 0));
        assert!(cursor.in_selection(0, 100, false, 0));
        assert!(cursor.in_selection(5, 100, false, 0));
        assert!(!cursor.in_selection(10, 100, false, 0));
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

    #[test]
    fn test_current_row_end_mid_row() {
        let cursor = Cursor {
            offset: 20,
            bytes_per_row: 16,
            sub_offset: 0,
            selection_anchor: None,
            selection_sub_anchor: None,
            selection_mode: SelectionMode::None,
        };
        assert_eq!(cursor.current_row_end(100), 31);
    }

    #[test]
    fn test_current_row_end_clamps_to_eof() {
        let cursor = Cursor {
            offset: 20,
            bytes_per_row: 16,
            sub_offset: 0,
            selection_anchor: None,
            selection_sub_anchor: None,
            selection_mode: SelectionMode::None,
        };
        // row would end at 31 but file only has 22 bytes
        assert_eq!(cursor.current_row_end(22), 21);
    }

    #[test]
    fn test_current_row_end_empty_file() {
        let cursor = Cursor {
            offset: 0,
            bytes_per_row: 16,
            sub_offset: 0,
            selection_anchor: None,
            selection_sub_anchor: None,
            selection_mode: SelectionMode::None,
        };
        assert_eq!(cursor.current_row_end(0), 0);
    }

    #[test]
    fn test_selection_line_mode() {
        let mut cursor = Cursor::new(16);
        cursor.start_selection(SelectionMode::Line);
        cursor.move_down(100);
        let s = cursor.selection_start().unwrap();
        let e = cursor.selection_end(100).unwrap();
        // Selection spans from row 0 start to row 1 end
        assert_eq!(s, 0);
        assert_eq!(e, 31);
        // All offsets in rows 0-1 should be in selection
        assert!(cursor.in_selection(0, 100, false, 0));
        assert!(cursor.in_selection(15, 100, false, 0));
        assert!(cursor.in_selection(16, 100, false, 0));
        assert!(cursor.in_selection(31, 100, false, 0));
        assert!(!cursor.in_selection(32, 100, false, 0));
    }

    #[test]
    fn test_selection_block_mode() {
        let mut cursor = Cursor::new(16);
        cursor.offset = 18;  // row 1, col 2
        cursor.start_selection(SelectionMode::Block);
        cursor.move_up(100);
        // Anchor at row 1 col 2, cursor now at row 0 col 2
        // Block spans rows 0-1, cols 2-2
        assert!(cursor.in_selection(2, 100, false, 0));   // row 0 col 2
        assert!(cursor.in_selection(18, 100, false, 0));  // row 1 col 2
        assert!(!cursor.in_selection(0, 100, false, 0));  // row 0 col 0
        assert!(!cursor.in_selection(1, 100, false, 0));  // row 0 col 1
        assert!(!cursor.in_selection(17, 100, false, 0)); // row 1 col 1
        assert!(!cursor.in_selection(19, 100, false, 0)); // row 1 col 3
    }

    #[test]
    fn test_selection_block_nibble_mode() {
        let mut cursor = Cursor::new(16);
        cursor.sub_offset = 1;  // low nibble
        cursor.offset = 18;
        cursor.start_selection(SelectionMode::Block);
        cursor.move_up(100);
        // Block spans rows 0-1, nibble cols 4-5 (since byte 2 has nibbles 4,5)
        assert!(cursor.in_selection(2, 100, true, 1));
        assert!(cursor.in_selection(18, 100, true, 1));
        assert!(!cursor.in_selection(2, 100, true, 0));
        assert!(!cursor.in_selection(18, 100, true, 0));
    }

    #[test]
    fn test_selection_line_mode_anchor_after_cursor() {
        let mut cursor = Cursor::new(16);
        cursor.offset = 20;  // row 1, col 4
        cursor.start_selection(SelectionMode::Line);
        cursor.move_up(100); // now row 0, col 4
        // Anchor at row 1, cursor at row 0. Line selection spans rows 0-1
        let s = cursor.selection_start().unwrap();
        let e = cursor.selection_end(100).unwrap();
        assert_eq!(s, 0);
        assert_eq!(e, 31);
    }

    #[test]
    fn test_selection_line_end_clamps_to_file_size() {
        let mut cursor = Cursor::new(16);
        cursor.offset = 18;  // row 1, col 2, file of 20 bytes (row 1 has only 4 bytes: 16-19)
        cursor.start_selection(SelectionMode::Line);
        // Single row selection on a partial row
        let s = cursor.selection_start().unwrap();
        let e = cursor.selection_end(20).unwrap();
        assert_eq!(s, 16);
        assert_eq!(e, 19);  // clamped to file_size - 1, not 31
        assert_eq!(e - s + 1, 4);  // only 4 yankable bytes

        // in_selection should work correctly too
        assert!(cursor.in_selection(16, 20, false, 0));
        assert!(cursor.in_selection(19, 20, false, 0));
        assert!(!cursor.in_selection(20, 20, false, 0));
        assert!(!cursor.in_selection(15, 20, false, 0));
    }

    #[test]
    fn test_selection_line_yank_bounds() {
        let mut cursor = Cursor::new(16);
        cursor.offset = 5;  // row 0, col 5
        cursor.start_selection(SelectionMode::Line);
        cursor.move_down(100); // now row 1, col 5
        // Yank should capture rows 0-1 = 32 bytes (offsets 0-31)
        let s = cursor.selection_start().unwrap();
        let e = cursor.selection_end(100).unwrap();
        assert_eq!(s, 0);
        assert_eq!(e, 31);
        assert_eq!(e - s + 1, 32);
    }
}
