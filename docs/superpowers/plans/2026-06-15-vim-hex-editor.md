# vim-hex Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a full-featured vim-inspired terminal hex editor in Rust.

**Architecture:** Two-crate workspace — `hexcore` (pure library with no terminal deps) and `hexview` (ratatui/crossterm binary). Core logic testable via simple `#[test]` functions.

**Tech Stack:** Rust, ratatui, crossterm, memmap2, serde

---

### Task 1: Workspace & Crate Scaffolding

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `hexcore/Cargo.toml`
- Create: `hexview/Cargo.toml`
- Create: `.gitignore`

- [ ] **Step 1: Write workspace Cargo.toml**

```toml
[workspace]
members = ["hexcore", "hexview"]
resolver = "2"
```

- [ ] **Step 2: Write hexcore/Cargo.toml**

```toml
[package]
name = "hexcore"
version = "0.1.0"
edition = "2021"

[dependencies]
memmap2 = "0.9"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Write hexview/Cargo.toml**

```toml
[package]
name = "hexview"
version = "0.1.0"
edition = "2021"

[dependencies]
hexcore = { path = "../hexcore" }
ratatui = "0.29"
crossterm = "0.28"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 4: Write .gitignore**

```
target/
*.swp
*.swo
```

- [ ] **Step 5: Create placeholder lib.rs and main.rs**

```bash
mkdir -p hexcore/src hexview/src
echo "// hexcore library" > hexcore/src/lib.rs
echo "fn main() {}" > hexview/src/main.rs
```

- [ ] **Step 6: Verify build**

Run: `cargo build`
Expected: clean compile

---

### Task 2: hexcore — ByteBuffer

**Files:**
- Create: `hexcore/src/buffer.rs`

The `ByteBuffer` owns the raw bytes and handles large file strategy automatically.

- [ ] **Step 1: Write tests and ByteBuffer**

```rust
// hexcore/src/buffer.rs
use std::collections::BTreeMap;
use std::path::Path;

pub struct ByteBuffer {
    /// In-memory data for files under threshold
    data: Vec<u8>,
    /// Total file size
    size: u64,
    /// File path
    path: Option<std::path::PathBuf>,
    /// Dirty flag
    modified: bool,
}

impl ByteBuffer {
    pub fn open(path: &Path) -> Result<Self, String>;
    pub fn save(&self) -> Result<(), String>;
    pub fn save_as(&mut self, path: &Path) -> Result<(), String>;
    pub fn len(&self) -> u64;
    pub fn read(&self, offset: u64, len: usize) -> Result<&[u8], String>;
    pub fn write(&mut self, offset: u64, bytes: &[u8]) -> Result<(), String>;
    pub fn insert(&mut self, offset: u64, bytes: &[u8]) -> Result<(), String>;
    pub fn delete(&mut self, offset: u64, len: usize) -> Result<(), String>;
    pub fn is_modified(&self) -> bool;
    pub fn path(&self) -> Option<&Path>;
}
```

- [ ] **Step 2: Write tests (TDD)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_open_and_read_small_file() {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(b"Hello World!").unwrap();
        let buf = ByteBuffer::open(tmp.path()).unwrap();
        assert_eq!(buf.len(), 12);
        let contents = buf.read(0, 12).unwrap();
        assert_eq!(contents, b"Hello World!");
    }

    #[test]
    fn test_write_marks_modified() {
        let mut buf = ByteBuffer::new(b"test");
        assert!(!buf.is_modified());
        buf.write(0, b"X").unwrap();
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
}
```

---

### Task 3: hexcore — Cursor

**Files:**
- Create: `hexcore/src/cursor.rs`

Handles cursor position and selection state across three visual modes.

- [ ] **Step 1: Write Cursor struct**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMode {
    None,
    Char,
    Line,
    Block,
}

pub struct Cursor {
    pub offset: u64,
    pub selection_start: Option<u64>,
    pub selection_mode: SelectionMode,
    pub bytes_per_row: u64,
}

impl Cursor {
    pub fn new(bytes_per_row: u64) -> Self;
    pub fn move_by(&mut self, delta: i64, file_size: u64);
    pub fn move_down(&mut self, file_size: u64);
    pub fn move_up(&mut self, file_size: u64);
    pub fn next_row(&self, offset: u64, file_size: u64) -> u64;
    pub fn prev_row(&self, offset: u64) -> u64;
    pub fn selection_start(&self) -> Option<u64>;
    pub fn selection_end(&self) -> Option<u64>;
    pub fn in_selection(&self, offset: u64) -> bool;
    pub fn start_selection(&mut self, mode: SelectionMode);
    pub fn clear_selection(&mut self);
    pub fn current_row_start(&self) -> u64;
}
```

- [ ] **Step 2: Write tests (TDD)**

```rust
#[test]
fn test_cursor_move_within_bounds() {
    let mut c = Cursor::new(16);
    c.move_by(10, 100);
    assert_eq!(c.offset, 10);
}
#[test]
fn test_cursor_stays_in_bounds() {
    let mut c = Cursor::new(16);
    c.offset = 95;
    c.move_by(10, 100);
    assert_eq!(c.offset, 99);
}
#[test]
fn test_selection_char_mode() {
    let mut c = Cursor::new(16);
    c.start_selection(SelectionMode::Char);
    c.move_by(5, 100);
    assert_eq!(c.selection_start, Some(0));
    assert!(c.in_selection(3));
    assert!(!c.in_selection(10));
}
```

---

### Task 4: hexcore — EditCommand & UndoManager

**Files:**
- Create: `hexcore/src/commands.rs`
- Create: `hexcore/src/undo.rs`

- [ ] **Step 1: Write EditCommand**

```rust
pub enum EditCommand {
    Overwrite { offset: u64, old_bytes: Vec<u8>, new_bytes: Vec<u8> },
    Insert { offset: u64, bytes: Vec<u8> },
    Delete { offset: u64, bytes: Vec<u8> },
}

impl EditCommand {
    pub fn apply(&self, buf: &mut ByteBuffer) -> Result<(), String>;
    pub fn undo(&self, buf: &mut ByteBuffer) -> Result<(), String>;
}
```

- [ ] **Step 2: Write UndoManager**

```rust
pub struct UndoManager {
    undo_stack: Vec<EditCommand>,
    redo_stack: Vec<EditCommand>,
    max_depth: usize,
}

impl UndoManager {
    pub fn new(max_depth: usize) -> Self;
    pub fn push(&mut self, cmd: EditCommand);
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
    pub fn undo(&mut self, buf: &mut ByteBuffer) -> Result<(), String>;
    pub fn redo(&mut self, buf: &mut ByteBuffer) -> Result<(), String>;
}
```

- [ ] **Step 3: Write tests (TDD)**

```rust
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
fn test_undo_redo_cycle() {
    let mut um = UndoManager::new(100);
    let mut buf = ByteBuffer::new(b"abcd");
    let cmd = EditCommand::Insert { offset: 0, bytes: b"XY".to_vec() };
    um.push(cmd);
    um.undo(&mut buf).unwrap();
    assert_eq!(buf.read(0, 2).unwrap(), b"ab");
    um.redo(&mut buf).unwrap();
    assert_eq!(buf.read(0, 4).unwrap(), b"XYab");
}
```

---

### Task 5: hexcore — Searcher

**Files:**
- Create: `hexcore/src/search.rs`

- [ ] **Step 1: Write Searcher**

```rust
pub struct Searcher;

impl Searcher {
    pub fn find_text(data: &[u8], pattern: &str, case_sensitive: bool) -> Vec<u64>;
    pub fn find_hex(data: &[u8], pattern: &str) -> Vec<u64>;
}
```

- [ ] **Step 2: Write tests (TDD)**

```rust
#[test]
fn test_find_text_basic() {
    let data = b"Hello World Hello";
    let results = Searcher::find_text(data, "Hello", true);
    assert_eq!(results, vec![0, 12]);
}
#[test]
fn test_find_hex_with_wildcard() {
    let data = &[0x48, 0x65, 0x6C, 0x6C, 0x6F];
    let results = Searcher::find_hex(data, "48 65 ?? 6C");
    assert_eq!(results, vec![0]);
}
#[test]
fn test_find_case_insensitive() {
    let data = b"hello HELLO Hello";
    let results = Searcher::find_text(data, "hello", false);
    assert_eq!(results, vec![0, 6, 12]);
}
```

---

### Task 6: hexcore — FileIo & Config

**Files:**
- Create: `hexcore/src/file_io.rs`
- Create: `hexcore/src/config.rs`

- [ ] **Step 1: Write FileIo**

```rust
pub struct FileIo;
impl FileIo {
    pub fn detect_encoding(data: &[u8]) -> &'static str;
    pub fn open(path: &Path) -> Result<(ByteBuffer, String), String>;
}
```

- [ ] **Step 2: Write Config**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub max_undo_depth: usize,
    pub bytes_per_row: u64,
    pub mmap_threshold_mb: u64,
    pub show_ascii: bool,
    pub use_overwrite_mode: bool,
}
impl Default for Config { ... }
impl Config { pub fn load() -> Self; pub fn save(&self); }
```

---

### Task 7: hexcore — lib.rs Re-exports

- [ ] **Step 1: Write lib.rs**

```rust
mod buffer;
mod cursor;
mod commands;
mod undo;
mod search;
mod file_io;
mod config;

pub use buffer::ByteBuffer;
pub use cursor::{Cursor, SelectionMode};
pub use commands::EditCommand;
pub use undo::UndoManager;
pub use search::Searcher;
pub use file_io::FileIo;
pub use config::Config;
```

- [ ] **Step 2: Verify hexcore tests all pass**

Run: `cargo test -p hexcore`
Expected: all pass

---

### Task 8: hexview — App State Machine

**Files:**
- Create: `hexview/src/app.rs`

- [ ] **Step 1: Write App**

```rust
pub enum Mode {
    Normal,
    Insert,
    VisualChar,
    VisualLine,
    VisualBlock,
    Command,
    Search,
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
}

pub struct TabInfo {
    pub name: String,
    pub path: Option<PathBuf>,
    pub modified: bool,
}

impl App {
    pub fn new() -> Self;
    pub fn open_file(&mut self, path: &Path) -> Result<(), String>;
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<(), String>;
}
```

- [ ] **Step 2: Implement key dispatch for all modes**
  - handle_normal: movement (h/j/k/l, Ctrl-D/U, gg/G), edit (x, dd, i, v, V, Ctrl-V, y, p, P), undo/redo (u, Ctrl-R), search (/, ?, n, N), command (:)
  - handle_insert: char input → overwrite at cursor, advance cursor; Esc → Normal
  - handle_visual_char/line/block: extend selection on h/j/k/l, d/x/y delete/cut/yank, Esc → Normal
  - handle_command: backspace/char input/Enter to execute command
  - handle_search: backspace/char input/Enter to search

---

### Task 9: hexview — Input Handler

**Files:**
- Create: `hexview/src/input.rs`

```rust
pub enum Action {
    MoveLeft, MoveRight, MoveDown, MoveUp,
    PageDown, PageUp,
    DeleteByte, DeleteLine,
    InsertMode,
    VisualChar, VisualLine, VisualBlock,
    Yank, Paste, PasteBefore,
    Undo, Redo,
    Save, SaveAs, OpenFile,
    SearchForward, SearchReverse,
    NextResult, PrevResult,
    CommandMode,
    Enter, Escape,
    Char(char), Backspace,
    TabNext, TabPrev,
    GoToOffset,
    Noop,
}
```

---

### Task 10: hexview — Rendering Components

**Files:**
- Create: `hexview/src/hex_view.rs`
- Create: `hexview/src/status_bar.rs`
- Create: `hexview/src/command_bar.rs`
- Create: `hexview/src/tabs.rs`
- Create: `hexview/src/ui.rs`

- [ ] **Step 1: hex_view.rs** — Render offset | hex bytes (grouped every 8) | ASCII per row. Highlight cursor byte and selection range.
- [ ] **Step 2: status_bar.rs** — Filename, dirty flag, mode, offset, selection info. Include endianness preview (u16/u32/u64 LE/BE).
- [ ] **Step 3: command_bar.rs** — Prompt + input for `:`, `/`, `?` modes. Search results counter.
- [ ] **Step 4: tabs.rs** — Tab bar with active tab highlighting and modified indicator.
- [ ] **Step 5: ui.rs** — Vertical layout: tabs (if >1) | hex view | command bar | status line.

---

### Task 11: hexview — Event Loop & Entry Point

**Files:**
- Create: `hexview/src/main.rs`
- Modify: `hexview/src/app.rs` (finalize)

- [ ] **Step 1: Write main.rs** — Initialize terminal, parse CLI arg, run event loop, restore on exit.

```rust
fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = ratatui::init();
    let mut app = App::new();
    if let Some(path) = std::env::args_os().nth(1) {
        app.open_file(&PathBuf::from(path)).unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        });
    }
    while !app.quit_requested {
        terminal.draw(|frame| ui::render(&app, frame))?;
        if let Event::Key(key) = crossterm::event::read()? {
            app.handle_key(key).unwrap_or_else(|e| {
                app.status_message = format!("Error: {}", e);
            });
        }
    }
    ratatui::restore();
    Ok(())
}
```

- [ ] **Step 2: Verify full build**

Run: `cargo build`
Expected: clean compile

---

### Task 12: Integration Testing (Manual)

- [ ] **Step 1: Create test binary file**

```bash
printf '\x48\x65\x6C\x6C\x6F\x20\x57\x6F\x72\x6C\x64\x00\xFF\xFE\xFD\xFC' > /tmp/test.bin
```

- [ ] **Step 2: Run and verify**
  - Hex/ASCII view renders correctly
  - Arrow keys and h/j/k/l navigate
  - `x` deletes byte, `u` undoes, `Ctrl-R` redoes
  - `i` enters insert mode, typing overwrites, `Esc` returns
  - `/` search works for text and hex
  - `:w /tmp/out.bin` saves
  - `:q` quits
  - `V` visual line selects full rows
  - `Ctrl-V` visual block selects rectangular region
  - `dd` deletes current row
  - `y` / `p` yank and paste
