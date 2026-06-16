use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::Path;
use std::time::Instant;
use unicode_width::UnicodeWidthStr;

use hexcore::{
    ByteBuffer, Config, Cursor, EditCommand, FileIo, JumpList, Searcher, SelectionMode, UndoManager,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Replace,
    ReplaceOnce,
    VisualChar,
    VisualLine,
    VisualBlock,
    Command,
    Search,
}

pub struct TabInfo {
    pub name: String,
    #[allow(dead_code)]
    pub path: Option<std::path::PathBuf>,
    pub modified: bool,
}

pub struct App {
    pub buffer: ByteBuffer,
    pub cursor: Cursor,
    pub undo: UndoManager,
    pub config: Config,
    pub mode: Mode,
    pub search_results: Vec<u64>,
    pub search_index: usize,
    pub search_reverse: bool,
    pub command_line: String,
    pub clipboard: Vec<u8>,
    pub tabs: Vec<TabInfo>,
    pub active_tab: usize,
    pub quit_requested: bool,
    pub status_message: String,
    pub show_help: bool,
    pub help_scroll: u64,
    pub help_lines: Vec<String>,
    pub nibble_mode: bool,
    pub jump_list: JumpList,
    last_key: (KeyCode, Instant),
}

impl App {
    pub fn new() -> Self {
        let config = Config::load();
        App {
            buffer: ByteBuffer::new(b""),
            cursor: Cursor::new(config.bytes_per_row),
            undo: UndoManager::new(config.max_undo_depth),
            config,
            mode: Mode::Normal,
            search_results: Vec::new(),
            search_index: 0,
            search_reverse: false,
            command_line: String::new(),
            clipboard: Vec::new(),
            tabs: Vec::new(),
            active_tab: 0,
            quit_requested: false,
            status_message: String::new(),
            show_help: false,
            help_scroll: 0,
            help_lines: Vec::new(),
            nibble_mode: false,
            jump_list: JumpList::new(100),
            last_key: (KeyCode::Null, Instant::now()),
        }
    }

    pub fn open_file(&mut self, path: &Path) -> Result<(), String> {
        let (buf, encoding) = FileIo::open(path)?;
        self.buffer = buf;
        self.cursor = Cursor::new(self.config.bytes_per_row);
        self.undo = UndoManager::new(self.config.max_undo_depth);
        self.mode = Mode::Normal;
        self.nibble_mode = false;
        self.jump_list.clear();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        self.status_message = format!("Opened {} ({})", name, encoding);
        let tab = TabInfo {
            name: name.clone(),
            path: Some(path.to_path_buf()),
            modified: false,
        };
        if self.active_tab < self.tabs.len() {
            self.tabs[self.active_tab] = tab;
        } else {
            self.tabs.push(tab);
        }
        Ok(())
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match self.mode {
            Mode::Normal => self.handle_normal(key),
            Mode::Insert => self.handle_insert(key),
            Mode::Replace => self.handle_replace(key),
            Mode::ReplaceOnce => self.handle_replace_once(key),
            Mode::VisualChar | Mode::VisualLine | Mode::VisualBlock => self.handle_visual(key),
            Mode::Command => self.handle_command(key),
            Mode::Search => self.handle_search_mode(key),
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> Result<(), String> {
        // When help is open, only j/k scroll help; : enters command mode to dismiss
        if self.show_help {
            return match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    let max_scroll = self.help_lines.len().saturating_sub(1) as u64;
                    if self.help_scroll < max_scroll {
                        self.help_scroll += 1;
                    }
                    Ok(())
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if self.help_scroll > 0 {
                        self.help_scroll -= 1;
                    }
                    Ok(())
                }
                KeyCode::Char(':') => {
                    self.mode = Mode::Command;
                    self.command_line.clear();
                    Ok(())
                }
                _ => Ok(()),
            };
        }

        match key.code {
            KeyCode::Char('h') | KeyCode::Left => {
                if self.nibble_mode {
                    if self.cursor.sub_offset == 1 {
                        self.cursor.sub_offset = 0;
                    } else {
                        self.cursor.move_by(-1, self.buffer.len());
                        self.cursor.sub_offset = 1;
                    }
                } else {
                    self.cursor.move_by(-1, self.buffer.len());
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.cursor.move_down(self.buffer.len());
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.cursor.move_up(self.buffer.len());
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.nibble_mode {
                    if self.cursor.sub_offset == 0 {
                        self.cursor.sub_offset = 1;
                    } else {
                        self.cursor.move_by(1, self.buffer.len());
                        self.cursor.sub_offset = 0;
                    }
                } else {
                    self.cursor.move_by(1, self.buffer.len());
                }
            }

            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let page = 8 * self.cursor.bytes_per_row as i64;
                self.cursor.move_by(page, self.buffer.len());
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let page = -(8 * self.cursor.bytes_per_row as i64);
                self.cursor.move_by(page, self.buffer.len());
            }

            KeyCode::Char('g') => {
                let now = Instant::now();
                if self.last_key.0 == KeyCode::Char('g')
                    && now.duration_since(self.last_key.1).as_millis() < 500
                {
                    self.jump_list.push(self.cursor.offset);
                    self.cursor.offset = 0;
                    self.cursor.sub_offset = 0;
                    self.cursor.clear_selection();
                } else {
                    self.status_message = "*".to_string();
                }
                self.last_key = (KeyCode::Char('g'), now);
            }
            KeyCode::Char('G') => {
                self.jump_list.push(self.cursor.offset);
                let max = self.buffer.len().saturating_sub(1);
                self.cursor.offset = max;
                self.cursor.sub_offset = 0;
                self.cursor.clear_selection();
            }

            KeyCode::Char('x') => {
                if self.buffer.len() > 0 {
                    if self.nibble_mode {
                        let offset = self.cursor.offset;
                        let nibble = self.cursor.sub_offset;
                        let remaining = (self.buffer.len() - offset) as usize;
                        let old_tail = self.buffer.read(offset, remaining).map(|s| s.to_vec()).unwrap_or_default();
                        self.buffer.delete_nibble(offset, nibble)?;
                        let new_tail = self.buffer.read(offset, remaining).map(|s| s.to_vec()).unwrap_or_default();
                        let cmd = EditCommand::Overwrite {
                            offset,
                            old_bytes: old_tail,
                            new_bytes: new_tail,
                        };
                        self.undo.push(cmd);
                        if self.cursor.offset >= self.buffer.len() && self.buffer.len() > 0 {
                            self.cursor.offset = self.buffer.len() - 1;
                        }
                    } else {
                        let offset = self.cursor.offset;
                        let old = self
                            .buffer
                            .read(offset, 1)
                            .map(|s| s.to_vec())
                            .unwrap_or_default();
                        let cmd = EditCommand::Delete {
                            offset,
                            bytes: old,
                        };
                        cmd.apply(&mut self.buffer)?;
                        self.undo.push(cmd);
                    }
                }
            }
            KeyCode::Char('d') => {
                let now = Instant::now();
                if self.last_key.0 == KeyCode::Char('d')
                    && now.duration_since(self.last_key.1).as_millis() < 500
                {
                    let row_start = self.cursor.current_row_start();
                    let row_end =
                        (row_start + self.cursor.bytes_per_row).min(self.buffer.len());
                    let len = (row_end - row_start) as usize;
                    if len > 0 {
                        let bytes = self
                            .buffer
                            .read(row_start, len)
                            .map(|s| s.to_vec())
                            .unwrap_or_default();
                        let cmd = EditCommand::Delete {
                            offset: row_start,
                            bytes,
                        };
                        cmd.apply(&mut self.buffer)?;
                        self.undo.push(cmd);
                        if self.cursor.offset >= self.buffer.len() && self.buffer.len() > 0 {
                            self.cursor.offset = self.buffer.len() - 1;
                        }
                    }
                }
                self.last_key = (KeyCode::Char('d'), now);
            }
            KeyCode::Char('a') => {
                if self.nibble_mode {
                    if self.cursor.sub_offset == 0 {
                        self.cursor.sub_offset = 1;
                    } else {
                        self.cursor.offset += 1;
                        self.cursor.sub_offset = 0;
                    }
                } else if self.cursor.offset < self.buffer.len() {
                    self.cursor.offset += 1;
                }
                self.mode = Mode::Insert;
            }
            KeyCode::Char('i') => {
                self.mode = Mode::Insert;
            }

            KeyCode::Char('z') => {
                self.nibble_mode = !self.nibble_mode;
                self.cursor.sub_offset = 0;
                self.status_message = if self.nibble_mode {
                    "Nibble mode on".to_string()
                } else {
                    "Nibble mode off".to_string()
                };
            }

            KeyCode::Char('0') => {
                self.cursor.offset = self.cursor.current_row_start();
                self.cursor.sub_offset = 0;
                self.cursor.clear_selection();
            }

            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(pos) = self.jump_list.back(self.cursor.offset) {
                    self.cursor.offset = pos;
                    self.cursor.sub_offset = 0;
                    self.cursor.clear_selection();
                }
            }
            KeyCode::Tab => {
                if let Some(pos) = self.jump_list.forward() {
                    self.cursor.offset = pos;
                    self.cursor.sub_offset = 0;
                    self.cursor.clear_selection();
                }
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let pre = self.cursor.offset;
                let offset = self.undo.redo(&mut self.buffer)?;
                self.cursor.offset = offset;
                self.cursor.sub_offset = 0;
                self.jump_list.push(pre);
            }
            KeyCode::Char('r') => {
                self.mode = Mode::ReplaceOnce;
            }
            KeyCode::Char('R') => {
                self.mode = Mode::Replace;
            }

            KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.mode = Mode::VisualBlock;
                self.cursor.start_selection(SelectionMode::Block);
            }
            KeyCode::Char('v') => {
                self.mode = Mode::VisualChar;
                self.cursor.start_selection(SelectionMode::Char);
            }
            KeyCode::Char('V') => {
                self.mode = Mode::VisualLine;
                self.cursor.start_selection(SelectionMode::Line);
            }

            KeyCode::Char('y') => {
                if let (Some(start), Some(end)) =
                    (self.cursor.selection_start(), self.cursor.selection_end())
                {
                    let len = (end - start + 1) as usize;
                    if let Ok(data) = self.buffer.read(start, len) {
                        self.clipboard = data.to_vec();
                        self.status_message =
                            format!("Yanked {} bytes", self.clipboard.len());
                    }
                    self.cursor.clear_selection();
                }
            }
            KeyCode::Char('p') => {
                if !self.clipboard.is_empty() {
                    let offset = self.cursor.offset + 1;
                    let bytes = self.clipboard.clone();
                    let cmd = EditCommand::Insert { offset, bytes };
                    cmd.apply(&mut self.buffer)?;
                    self.undo.push(cmd);
                }
            }
            KeyCode::Char('P') => {
                if !self.clipboard.is_empty() {
                    let offset = self.cursor.offset;
                    let bytes = self.clipboard.clone();
                    let cmd = EditCommand::Insert { offset, bytes };
                    cmd.apply(&mut self.buffer)?;
                    self.undo.push(cmd);
                }
            }

            KeyCode::Char('u') => {
                let pre = self.cursor.offset;
                let offset = self.undo.undo(&mut self.buffer)?;
                self.cursor.offset = offset;
                self.cursor.sub_offset = 0;
                self.jump_list.push(pre);
            }
            KeyCode::Char('/') => {
                self.jump_list.push(self.cursor.offset);
                self.mode = Mode::Search;
                self.command_line.clear();
                self.search_reverse = false;
                self.search_index = 0;
            }
            KeyCode::Char('?') => {
                self.jump_list.push(self.cursor.offset);
                self.mode = Mode::Search;
                self.command_line.clear();
                self.search_reverse = true;
                self.search_index = 0;
            }
            KeyCode::Char('n') => {
                self.jump_list.push(self.cursor.offset);
                self.next_search_result();
                self.cursor.sub_offset = 0;
            }
            KeyCode::Char('N') => {
                self.jump_list.push(self.cursor.offset);
                self.prev_search_result();
                self.cursor.sub_offset = 0;
            }

            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.command_line.clear();
            }

            _ => {}
        }
        Ok(())
    }

    fn handle_insert(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char(c) => {
                if self.nibble_mode {
                    let digit = c.to_digit(16).unwrap_or(u32::MAX) as u8;
                    if digit <= 0x0F {
                        let offset = self.cursor.offset;
                        let nibble = self.cursor.sub_offset;
                        if offset >= self.buffer.len() {
                            let cmd = EditCommand::Insert {
                                offset,
                                bytes: vec![digit << 4],
                            };
                            cmd.apply(&mut self.buffer)?;
                            self.undo.push(cmd);
                        } else {
                            let remaining = (self.buffer.len() - offset) as usize;
                            let old_tail = self.buffer.read(offset, remaining).map(|s| s.to_vec()).unwrap_or_default();
                            self.buffer.insert_nibble(offset, nibble, digit)?;
                            let new_tail = self.buffer.read(offset, remaining).map(|s| s.to_vec()).unwrap_or_default();
                            let cmd = EditCommand::Overwrite {
                                offset,
                                old_bytes: old_tail,
                                new_bytes: new_tail,
                            };
                            self.undo.push(cmd);
                        }
                        if self.cursor.sub_offset == 0 {
                            self.cursor.sub_offset = 1;
                        } else {
                            self.cursor.offset += 1;
                            self.cursor.sub_offset = 0;
                        }
                    }
                } else {
                    let offset = self.cursor.offset;
                    let old = if offset < self.buffer.len() {
                        self.buffer
                            .read(offset, 1)
                            .map(|s| s.to_vec())
                            .unwrap_or_default()
                    } else {
                        vec![]
                    };
                    if self.config.use_overwrite_mode && offset < self.buffer.len() {
                        let cmd = EditCommand::Overwrite {
                            offset,
                            old_bytes: old,
                            new_bytes: vec![c as u8],
                        };
                        cmd.apply(&mut self.buffer)?;
                        self.undo.push(cmd);
                    } else {
                        let cmd = EditCommand::Insert {
                            offset,
                            bytes: vec![c as u8],
                        };
                        cmd.apply(&mut self.buffer)?;
                        self.undo.push(cmd);
                    }
                    if self.cursor.offset < self.buffer.len() {
                        self.cursor.offset += 1;
                    }
                }
            }
            KeyCode::Backspace => {
                if self.nibble_mode {
                    if self.cursor.offset > 0 || self.cursor.sub_offset > 0 {
                        if self.cursor.sub_offset == 0 {
                            self.cursor.offset -= 1;
                            self.cursor.sub_offset = 1;
                        } else {
                            self.cursor.sub_offset = 0;
                        }
                        let offset = self.cursor.offset;
                        let nibble = self.cursor.sub_offset;
                        let remaining = (self.buffer.len() - offset) as usize;
                        let old_tail = self.buffer.read(offset, remaining).map(|s| s.to_vec()).unwrap_or_default();
                        self.buffer.delete_nibble(offset, nibble)?;
                        let new_tail = self.buffer.read(offset, remaining).map(|s| s.to_vec()).unwrap_or_default();
                        let cmd = EditCommand::Overwrite {
                            offset,
                            old_bytes: old_tail,
                            new_bytes: new_tail,
                        };
                        self.undo.push(cmd);
                    }
                } else if self.cursor.offset > 0 {
                    let offset = self.cursor.offset - 1;
                    let bytes = self
                        .buffer
                        .read(offset, 1)
                        .map(|s| s.to_vec())
                        .unwrap_or_default();
                    let cmd = EditCommand::Delete { offset, bytes };
                    cmd.apply(&mut self.buffer)?;
                    self.undo.push(cmd);
                    self.cursor.offset = offset.min(self.buffer.len().saturating_sub(1));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_replace_once(&mut self, key: KeyEvent) -> Result<(), String> {
        if let KeyCode::Char(c) = key.code {
            let offset = self.cursor.offset;
            if offset < self.buffer.len() {
                if self.nibble_mode {
                    let digit = c.to_digit(16).unwrap_or(u32::MAX) as u8;
                    if digit <= 0x0F {
                        let old_byte = self.buffer.read(offset, 1).map(|s| s.to_vec()).unwrap_or_default();
                        self.buffer.set_nibble(offset, self.cursor.sub_offset, digit)?;
                        let new_byte = self.buffer.read(offset, 1).map(|s| s.to_vec()).unwrap_or_default();
                        let cmd = EditCommand::Overwrite {
                            offset,
                            old_bytes: old_byte,
                            new_bytes: new_byte,
                        };
                        self.undo.push(cmd);
                    }
                } else {
                    let old = self.buffer.read(offset, 1).map(|s| s.to_vec()).unwrap_or_default();
                    let cmd = EditCommand::Overwrite {
                        offset,
                        old_bytes: old,
                        new_bytes: vec![c as u8],
                    };
                    cmd.apply(&mut self.buffer)?;
                    self.undo.push(cmd);
                }
            }
        }
        self.mode = Mode::Normal;
        Ok(())
    }

    fn handle_replace(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char(c) => {
                let offset = self.cursor.offset;
                if self.nibble_mode {
                    if offset < self.buffer.len() {
                        let digit = c.to_digit(16).unwrap_or(u32::MAX) as u8;
                        if digit <= 0x0F {
                            let old_byte = self.buffer.read(offset, 1).map(|s| s.to_vec()).unwrap_or_default();
                            self.buffer.set_nibble(offset, self.cursor.sub_offset, digit)?;
                            let new_byte = self.buffer.read(offset, 1).map(|s| s.to_vec()).unwrap_or_default();
                            let cmd = EditCommand::Overwrite {
                                offset,
                                old_bytes: old_byte,
                                new_bytes: new_byte,
                            };
                            self.undo.push(cmd);
                            if self.cursor.sub_offset == 0 {
                                self.cursor.sub_offset = 1;
                            } else {
                                self.cursor.offset += 1;
                                self.cursor.sub_offset = 0;
                            }
                        }
                    }
                } else if offset < self.buffer.len() {
                    let old = self.buffer.read(offset, 1).map(|s| s.to_vec()).unwrap_or_default();
                    let cmd = EditCommand::Overwrite {
                        offset,
                        old_bytes: old,
                        new_bytes: vec![c as u8],
                    };
                    cmd.apply(&mut self.buffer)?;
                    self.undo.push(cmd);
                    self.cursor.offset += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_visual(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => {
                if self.nibble_mode {
                    if self.cursor.sub_offset == 1 {
                        self.cursor.sub_offset = 0;
                    } else {
                        self.cursor.move_by(-1, self.buffer.len());
                        self.cursor.sub_offset = 1;
                    }
                } else {
                    self.cursor.move_by(-1, self.buffer.len());
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.cursor.move_down(self.buffer.len());
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.cursor.move_up(self.buffer.len());
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.nibble_mode {
                    if self.cursor.sub_offset == 0 {
                        self.cursor.sub_offset = 1;
                    } else {
                        self.cursor.move_by(1, self.buffer.len());
                        self.cursor.sub_offset = 0;
                    }
                } else {
                    self.cursor.move_by(1, self.buffer.len());
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') => {
                if self.nibble_mode {
                    let anchor_off = self.cursor.selection_anchor.unwrap_or(self.cursor.offset);
                    let anchor_sub = self.cursor.selection_sub_anchor.unwrap_or(0);
                    let cur_off = self.cursor.offset;
                    let cur_sub = self.cursor.sub_offset;
                    let anchor_nib = anchor_off * 2 + anchor_sub as u64;
                    let cur_nib = cur_off * 2 + cur_sub as u64;
                    let start_nib = anchor_nib.min(cur_nib);
                    let end_nib = anchor_nib.max(cur_nib);
                    let start_byte = start_nib / 2;
                    let start_sub = (start_nib % 2) as u8;
                    let total = (end_nib - start_nib + 1) as usize;
                    let remaining = (self.buffer.len() - start_byte) as usize;
                    let old_tail = self.buffer.read(start_byte, remaining).map(|s| s.to_vec()).unwrap_or_default();
                    for _ in 0..total {
                        self.buffer.delete_nibble(start_byte, start_sub)?;
                    }
                    let new_tail = self.buffer.read(start_byte, remaining).map(|s| s.to_vec()).unwrap_or_default();
                    let cmd = EditCommand::Overwrite {
                        offset: start_byte,
                        old_bytes: old_tail,
                        new_bytes: new_tail,
                    };
                    self.undo.push(cmd);
                    self.cursor.offset = start_byte.min(self.buffer.len().saturating_sub(1));
                    self.cursor.sub_offset = 0;
                } else if let (Some(start), Some(end)) =
                    (self.cursor.selection_start(), self.cursor.selection_end())
                {
                    let len = (end - start + 1) as usize;
                    let bytes = self
                        .buffer
                        .read(start, len)
                        .map(|s| s.to_vec())
                        .unwrap_or_default();
                    let cmd = EditCommand::Delete {
                        offset: start,
                        bytes,
                    };
                    cmd.apply(&mut self.buffer)?;
                    self.undo.push(cmd);
                    self.cursor.offset = start.min(self.buffer.len().saturating_sub(1));
                }
                self.cursor.clear_selection();
                self.mode = Mode::Normal;
            }
            KeyCode::Char('y') => {
                if let (Some(start), Some(end)) =
                    (self.cursor.selection_start(), self.cursor.selection_end())
                {
                    let len = (end - start + 1) as usize;
                    if let Ok(data) = self.buffer.read(start, len) {
                        self.clipboard = data.to_vec();
                        self.status_message =
                            format!("Yanked {} bytes", self.clipboard.len());
                    }
                }
                self.cursor.clear_selection();
                self.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                self.cursor.clear_selection();
                self.mode = Mode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_command(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.command_line.clear();
            }
            KeyCode::Enter => {
                let cmd = self.command_line.clone();
                self.command_line.clear();
                self.mode = Mode::Normal;
                self.execute_command(&cmd)?;
            }
            KeyCode::Char(c) => {
                self.command_line.push(c);
            }
            KeyCode::Backspace => {
                self.command_line.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_search_mode(&mut self, key: KeyEvent) -> Result<(), String> {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.command_line.clear();
            }
            KeyCode::Enter => {
                let query = self.command_line.clone();
                self.command_line.clear();
                self.mode = Mode::Normal;
                let data = self
                    .buffer
                    .read(0, self.buffer.len() as usize)
                    .unwrap_or(b"")
                    .to_vec();
                let hex_results = Searcher::find_hex(&data, &query);
                if !hex_results.is_empty() {
                    self.search_results = hex_results;
                } else {
                    self.search_results = Searcher::find_text(&data, &query, true);
                    if self.search_results.is_empty() {
                        self.search_results = Searcher::find_text(&data, &query, false);
                    }
                }
                if !self.search_results.is_empty() {
                    self.search_index = 0;
                    if self.search_reverse {
                        self.search_index = self.search_results.len() - 1;
                    }
                    self.cursor.offset = self.search_results[self.search_index];
                    self.status_message = format!(
                        "{}/{} matches",
                        self.search_index + 1,
                        self.search_results.len()
                    );
                } else {
                    self.status_message = "No matches".to_string();
                }
            }
            KeyCode::Char(c) => {
                self.command_line.push(c);
            }
            KeyCode::Backspace => {
                self.command_line.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn next_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.search_index = (self.search_index + 1) % self.search_results.len();
        self.cursor.offset = self.search_results[self.search_index];
    }

    fn prev_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        if self.search_index == 0 {
            self.search_index = self.search_results.len() - 1;
        } else {
            self.search_index -= 1;
        }
        self.cursor.offset = self.search_results[self.search_index];
    }

    fn execute_command(&mut self, cmd: &str) -> Result<(), String> {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            return Ok(());
        }

        // If help is open, :q / :q! dismisses help instead of quitting
        if self.show_help && (cmd == "q" || cmd == "q!") {
            self.show_help = false;
            return Ok(());
        }

        match cmd {
            "q" | "q!" => {
                self.quit_requested = true;
            }
            "w" => {
                self.buffer.save()?;
                self.status_message = "Saved".to_string();
            }
            "wq" => {
                self.buffer.save()?;
                self.quit_requested = true;
            }
            _ => {
                if let Some(path) = cmd.strip_prefix("w ") {
                    self.buffer.save_as(Path::new(path))?;
                    self.status_message = format!("Saved to {}", path);
                } else if let Some(path) = cmd.strip_prefix("e ") {
                    self.open_file(Path::new(path))?;
                } else if let Some(replace) = cmd.strip_prefix("%s/") {
                    let parts: Vec<&str> = replace.splitn(2, '/').collect();
                    if parts.len() == 2 {
                        let find_pat = Searcher::parse_hex_pattern(parts[0]);
                        let replace_bytes = Searcher::hex_to_bytes(parts[1]);
                        if let (Some(find_pat), Some(replace_bytes)) =
                            (find_pat, replace_bytes)
                        {
                            let pat_len = find_pat.len();
                            if replace_bytes.len() != pat_len {
                                self.status_message = format!(
                                    "Replacement length ({}) must match pattern length ({})",
                                    replace_bytes.len(),
                                    pat_len
                                );
                            } else {
                                let data = self
                                    .buffer
                                    .read(0, self.buffer.len() as usize)
                                    .unwrap_or(b"")
                                    .to_vec();
                                let results =
                                    Searcher::find_hex(&data, parts[0]);
                                for &offset in results.iter().rev() {
                                    let old = self
                                        .buffer
                                        .read(offset, pat_len)
                                        .map(|s| s.to_vec())
                                        .unwrap_or_default();
                                    let cmd = EditCommand::Overwrite {
                                        offset,
                                        old_bytes: old,
                                        new_bytes: replace_bytes.clone(),
                                    };
                                    cmd.apply(&mut self.buffer)?;
                                    self.undo.push(cmd);
                                }
                                self.status_message = format!(
                                    "Replaced {} occurrences",
                                    results.len()
                                );
                            }
                        }
                    }
                } else if cmd == "h" || cmd == "help" {
                    self.show_help = true;
                    self.help_scroll = 0;
                    self.build_help_lines();
                } else {
                    self.status_message = format!("Unknown command: {}", cmd);
                }
            }
        }
        Ok(())
    }

    fn build_help_lines(&mut self) {
        let pad = |key: &str, desc: &str| -> String {
            let w = key.width();
            let padding = 20usize.saturating_sub(w);
            format!("{}{} {}", key, " ".repeat(padding), desc)
        };

        self.help_lines = vec![
            String::new(),
            "─── NORMAL MODE ───".to_string(),
            pad("h/← l/→ j/↓ k/↑", "Move cursor"),
            pad("0", "Go to row start"),
            pad("gg / G", "Go to top / bottom"),
            pad("Ctrl-D / Ctrl-U", "Page down / up"),
            pad("i / a", "Insert at cursor / after cursor"),
            pad("r / R", "Replace once / continuous replace"),
            pad("x / dd", "Delete byte / delete row"),
            pad("y / p / P", "Yank / paste after / paste before"),
            pad("u / Ctrl-R", "Undo / redo"),
            pad("Ctrl-O / Tab", "Jump back / forward in history"),
            pad("/  ?", "Search forward / backward"),
            pad("n / N", "Next / previous search result"),
            pad(":", "Enter command mode"),
            pad("v / V / Ctrl-V", "Visual char / line / block"),
            pad("z", "Toggle nibble mode (4-bit editing)"),
            String::new(),
            "─── INSERT MODE ───".to_string(),
            pad("<char>", "Insert byte at cursor"),
            pad("Esc", "Return to Normal"),
            pad("Backspace", "Delete byte before cursor"),
            String::new(),
            "─── REPLACE MODE ───".to_string(),
            pad("<char>", "Overwrite byte, advance cursor"),
            pad("Esc", "Return to Normal"),
            String::new(),
            "─── VISUAL MODE ───".to_string(),
            pad("h/j/k/l", "Extend selection"),
            pad("d / x / y", "Delete / yank selection"),
            pad("Esc", "Cancel selection"),
            String::new(),
            "─── COMMAND MODE ───".to_string(),
            pad(":w", "Save file"),
            pad(":w <path>", "Save as"),
            pad(":e <path>", "Open file"),
            pad(":q / :q!", "Quit"),
            pad(":wq", "Save and quit"),
            pad(":%s/find/replace", "Replace hex pattern"),
            pad(":h / :help", "This help view"),
            pad("Esc", "Cancel, return to Normal"),
            String::new(),
            "─── SEARCH MODE ───".to_string(),
            pad("<query> Enter", "Search text or hex (spaces optional, e.g. 48656C)"),
            pad("Esc", "Cancel"),
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn esc() -> KeyEvent {
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
    }

    fn enter() -> KeyEvent {
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
    }

    #[test]
    fn test_app_opens_file() {
        let mut app = App::new();
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.bin");
        std::fs::write(&path, b"Hello World").unwrap();
        app.open_file(&path).unwrap();
        assert_eq!(app.buffer.len(), 11);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_normal_mode_movement() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcdefghijklmnopqrstuvwxyz");
        app.cursor = Cursor::new(16);

        app.handle_key(key('l')).unwrap();
        assert_eq!(app.cursor.offset, 1);
        app.handle_key(key('h')).unwrap();
        assert_eq!(app.cursor.offset, 0);
        app.handle_key(key('j')).unwrap();
        assert_eq!(app.cursor.offset, 16);
        app.handle_key(key('k')).unwrap();
        assert_eq!(app.cursor.offset, 0);
    }

    #[test]
    fn test_delete_byte_and_undo() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcd");

        app.handle_key(key('x')).unwrap();
        assert_eq!(app.buffer.len(), 3);
        assert_eq!(app.buffer.read(0, 3).unwrap(), b"bcd");

        app.handle_key(key('u')).unwrap();
        assert_eq!(app.buffer.len(), 4);
        assert_eq!(app.buffer.read(0, 4).unwrap(), b"abcd");
        assert_eq!(app.cursor.offset, 0);
    }

    #[test]
    fn test_insert_mode() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"ac");
        app.config.use_overwrite_mode = false;

        // Move cursor to position 1, then insert
        app.handle_key(key('l')).unwrap();
        assert_eq!(app.cursor.offset, 1);

        app.handle_key(key('i')).unwrap();
        assert_eq!(app.mode, Mode::Insert);

        app.handle_key(key('b')).unwrap();
        assert_eq!(app.mode, Mode::Insert);
        assert_eq!(app.buffer.len(), 3);
        assert_eq!(app.buffer.read(0, 3).unwrap(), b"abc");

        app.handle_key(esc()).unwrap(); // Esc
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_visual_mode_selection() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcdefgh");

        app.handle_key(key('v')).unwrap(); // VisualChar
        assert_eq!(app.mode, Mode::VisualChar);
        assert!(app.cursor.selection_start().is_some());

        app.handle_key(key('l')).unwrap(); // extend right
        app.handle_key(key('l')).unwrap(); // extend right
        assert_eq!(app.cursor.offset, 2);

        // Yank
        app.handle_key(key('y')).unwrap();
        assert_eq!(app.clipboard, b"abc");
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_command_mode_quit() {
        let mut app = App::new();
        app.handle_key(key(':')).unwrap();
        assert_eq!(app.mode, Mode::Command);

        app.handle_key(key('q')).unwrap();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)).unwrap();
        assert!(app.quit_requested);
    }

    #[test]
    fn test_command_mode_save() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"test data");
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("out.bin");
        app.buffer.save_as(&path).unwrap();

        app.handle_key(key(':')).unwrap();
        for c in "w".chars() {
            app.handle_key(key(c)).unwrap();
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)).unwrap();

        let loaded = std::fs::read(&path).unwrap();
        assert_eq!(loaded, b"test data");
    }

    #[test]
    fn test_search_mode() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"Hello World Hello");

        app.handle_key(key('/')).unwrap();
        assert_eq!(app.mode, Mode::Search);

        for c in "Hello".chars() {
            app.handle_key(key(c)).unwrap();
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)).unwrap();
        assert_eq!(app.mode, Mode::Normal);
        assert!(!app.search_results.is_empty());

        app.handle_key(key('n')).unwrap();
        app.handle_key(key('N')).unwrap();
    }

    #[test]
    fn test_dd_delete_line() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcdefghijklmnopqrstuvwxyz");
        app.cursor = Cursor::new(16);

        // Simulate dd double-tap
        app.handle_key(key('d')).unwrap();
        app.handle_key(key('d')).unwrap();

        assert_eq!(app.buffer.len(), 10); // removed 16 bytes
    }

    #[test]
    fn test_gg_and_g() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcdefghijklmnopqrstuvwxyz");
        app.cursor.offset = 10;

        // gg should go to start
        app.handle_key(key('g')).unwrap();
        app.handle_key(key('g')).unwrap();
        assert_eq!(app.cursor.offset, 0);

        // G should go to end
        app.cursor.offset = 0;
        app.handle_key(key('G')).unwrap();
        assert_eq!(app.cursor.offset, 25);
    }

    #[test]
    fn test_paste_yanked() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcd");
        // Yank "bc"
        {
            let start = 1;
            let end = 2;
            let len = (end - start + 1) as usize;
            app.clipboard = app.buffer.read(start, len).unwrap().to_vec();
        }
        assert_eq!(app.clipboard, b"bc");

        // Paste at cursor (offset 0)
        app.handle_key(key('P')).unwrap(); // paste before
        assert_eq!(app.buffer.read(0, 6).unwrap(), b"bcabcd");
    }

    #[test]
    fn test_endianness_preview() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(&[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);

        // Check status bar includes endian values
        // (indirect test: buffer has data, cursor at 0, bytes readable)
        let bytes = app.buffer.read(0, 8).unwrap();
        let u16_le = u16::from_le_bytes([bytes[0], bytes[1]]);
        let u16_be = u16::from_be_bytes([bytes[0], bytes[1]]);
        assert_eq!(u16_le, 0x3412);
        assert_eq!(u16_be, 0x1234);
    }

    #[test]
    fn test_visual_block_mode() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcdefgh");
        app.handle_key(ctrl('v')).unwrap(); // Ctrl-V for Visual Block
        assert_eq!(app.mode, Mode::VisualBlock);
        assert_eq!(app.cursor.selection_mode, SelectionMode::Block);
    }

    #[test]
    fn test_visual_line_mode() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcdefghijklmnopqrstuvwxyz");
        app.handle_key(key('V')).unwrap();
        assert_eq!(app.mode, Mode::VisualLine);
        assert_eq!(app.cursor.selection_mode, SelectionMode::Line);

        // Extend selection
        app.handle_key(key('j')).unwrap();
        app.handle_key(key('y')).unwrap(); // yank
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_a_insert_after() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcd");
        app.cursor.offset = 0;

        // `a` should move cursor forward then enter insert
        app.handle_key(key('a')).unwrap();
        assert_eq!(app.mode, Mode::Insert);
        assert_eq!(app.cursor.offset, 1);
    }

    #[test]
    fn test_zero_goes_to_row_start() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcdefghijklmnopqrstuvwxyz");
        // Set cursor to something > row width
        app.cursor.offset = 10;

        app.handle_key(key('0')).unwrap();
        // Should jump to start of current row (row_start = offset / 16 * 16)
        assert_eq!(app.cursor.offset, 0);
    }

    #[test]
    fn test_r_replace_once() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcd");
        app.cursor.offset = 1;

        app.handle_key(key('r')).unwrap();
        assert_eq!(app.mode, Mode::ReplaceOnce);

        app.handle_key(key('X')).unwrap();
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.buffer.read(1, 1).unwrap(), b"X");
    }

    #[test]
    fn test_r_replace_mode() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcdefgh");
        app.cursor.offset = 2;

        app.handle_key(key('R')).unwrap();
        assert_eq!(app.mode, Mode::Replace);

        app.handle_key(key('X')).unwrap();
        assert_eq!(app.buffer.read(2, 1).unwrap(), b"X");
        assert_eq!(app.cursor.offset, 3);

        app.handle_key(key('Y')).unwrap();
        assert_eq!(app.buffer.read(3, 1).unwrap(), b"Y");
        assert_eq!(app.cursor.offset, 4);

        app.handle_key(esc()).unwrap();
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_help_opens() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"test");

        app.handle_key(key(':')).unwrap();
        app.handle_key(key('h')).unwrap();
        app.handle_key(enter()).unwrap();

        assert!(app.show_help);
        assert!(!app.help_lines.is_empty());
        assert_eq!(app.help_scroll, 0);
    }

    #[test]
    fn test_help_command() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"test");

        app.handle_key(key(':')).unwrap();
        for c in "help".chars() {
            app.handle_key(key(c)).unwrap();
        }
        app.handle_key(enter()).unwrap();

        assert!(app.show_help);
    }

    #[test]
    fn test_help_dismiss_with_q() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"test");

        // Open help
        app.handle_key(key(':')).unwrap();
        app.handle_key(key('h')).unwrap();
        app.handle_key(enter()).unwrap();
        assert!(app.show_help);

        // Dismiss with :q
        app.handle_key(key(':')).unwrap();
        app.handle_key(key('q')).unwrap();
        app.handle_key(enter()).unwrap();
        assert!(!app.show_help);
    }

    #[test]
    fn test_help_scroll() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"test");

        app.handle_key(key(':')).unwrap();
        app.handle_key(key('h')).unwrap();
        app.handle_key(enter()).unwrap();
        assert!(app.show_help);

        let initial = app.help_scroll;
        app.handle_key(key('j')).unwrap();
        assert_eq!(app.help_scroll, initial + 1);

        app.handle_key(key('k')).unwrap();
        assert_eq!(app.help_scroll, initial);
    }

    #[test]
    fn test_nibble_toggle() {
        let mut app = App::new();
        assert!(!app.nibble_mode);
        app.handle_key(key('z')).unwrap();
        assert!(app.nibble_mode);
        app.handle_key(key('z')).unwrap();
        assert!(!app.nibble_mode);
    }

    #[test]
    fn test_nibble_movement_h_l() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"\xAB\xCD");
        app.cursor = Cursor::new(16);
        app.handle_key(key('z')).unwrap();

        assert_eq!(app.cursor.offset, 0);
        assert_eq!(app.cursor.sub_offset, 0);

        app.handle_key(key('l')).unwrap();
        assert_eq!(app.cursor.offset, 0);
        assert_eq!(app.cursor.sub_offset, 1);

        app.handle_key(key('l')).unwrap();
        assert_eq!(app.cursor.offset, 1);
        assert_eq!(app.cursor.sub_offset, 0);

        app.handle_key(key('h')).unwrap();
        assert_eq!(app.cursor.offset, 0);
        assert_eq!(app.cursor.sub_offset, 1);

        app.handle_key(key('h')).unwrap();
        assert_eq!(app.cursor.offset, 0);
        assert_eq!(app.cursor.sub_offset, 0);
    }

    #[test]
    fn test_nibble_replace_r() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"\xAB\xCD");
        app.cursor = Cursor::new(16);
        app.handle_key(key('z')).unwrap();

        app.handle_key(key('r')).unwrap();
        app.handle_key(key('3')).unwrap();
        assert_eq!(app.buffer.read(0, 1).unwrap(), b"\x3B");
        assert_eq!(app.cursor.offset, 0);
        assert_eq!(app.cursor.sub_offset, 0);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_nibble_replace_mode() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"\xAB\xCD");
        app.cursor = Cursor::new(16);
        app.handle_key(key('z')).unwrap();

        app.handle_key(key('R')).unwrap();
        app.handle_key(key('3')).unwrap();
        assert_eq!(app.buffer.read(0, 1).unwrap(), b"\x3B");
        assert_eq!(app.cursor.offset, 0);
        assert_eq!(app.cursor.sub_offset, 1);

        app.handle_key(key('7')).unwrap();
        assert_eq!(app.buffer.read(0, 1).unwrap(), b"\x37");
        assert_eq!(app.cursor.offset, 1);
        assert_eq!(app.cursor.sub_offset, 0);

        app.handle_key(esc()).unwrap();
    }

    #[test]
    fn test_nibble_insert_i() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"\xAB\xCD");
        app.cursor = Cursor::new(16);
        app.handle_key(key('z')).unwrap();

        app.handle_key(key('i')).unwrap();
        app.handle_key(key('9')).unwrap();
        assert_eq!(app.buffer.read(0, 2).unwrap(), b"\x9A\xBC");
        assert_eq!(app.cursor.offset, 0);
        assert_eq!(app.cursor.sub_offset, 1);
        app.handle_key(esc()).unwrap();
    }

    #[test]
    fn test_nibble_delete_x() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"\xAB\xCD");
        app.cursor = Cursor::new(16);
        app.handle_key(key('z')).unwrap();

        app.handle_key(key('x')).unwrap();
        assert_eq!(app.buffer.read(0, 2).unwrap(), b"\xBC\xD0");
    }

    #[test]
    fn test_nibble_delete_low_nibble() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"\xAB\xCD");
        app.cursor = Cursor::new(16);
        app.handle_key(key('z')).unwrap();

        app.handle_key(key('l')).unwrap();
        app.handle_key(key('x')).unwrap();
        assert_eq!(app.buffer.read(0, 2).unwrap(), b"\xAC\xD0");
    }

    #[test]
    fn test_nibble_undo() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"\xAB\xCD");
        app.cursor = Cursor::new(16);
        app.handle_key(key('z')).unwrap();

        app.handle_key(key('x')).unwrap();
        assert_eq!(app.buffer.read(0, 2).unwrap(), b"\xBC\xD0");

        app.handle_key(key('u')).unwrap();
        assert_eq!(app.buffer.read(0, 2).unwrap(), b"\xAB\xCD");
        assert_eq!(app.cursor.offset, 0);
    }

    #[test]
    fn test_nibble_visual_delete_range() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"\xAB\xCD\xEF");
        app.cursor = Cursor::new(16);
        app.handle_key(key('z')).unwrap();

        app.handle_key(key('v')).unwrap();
        app.handle_key(key('l')).unwrap();
        app.handle_key(key('l')).unwrap();
        app.handle_key(key('l')).unwrap();
        app.handle_key(key('x')).unwrap();
        // Nibbles A B C D removed, leaving E F, padded to 3 bytes
        assert_eq!(app.buffer.read(0, 3).unwrap(), b"\xEF\x00\x00");
    }

    #[test]
    fn test_nibble_zero_goes_to_row_start() {
        let mut app = App::new();
        app.buffer = ByteBuffer::new(b"abcdefghijklmnopqrstuvwxyz");
        app.cursor = Cursor::new(16);
        app.cursor.offset = 20;
        app.cursor.sub_offset = 1;
        app.handle_key(key('z')).unwrap();

        app.handle_key(key('0')).unwrap();
        assert_eq!(app.cursor.offset, 16);
        assert_eq!(app.cursor.sub_offset, 0);
    }
}
