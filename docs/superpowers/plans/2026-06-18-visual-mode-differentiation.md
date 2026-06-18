# Visual Mode Differentiation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Differentiate VisualLine and VisualBlock selection behavior from VisualChar in the hex editor.

**Architecture:** Extend `Cursor` with mode-aware `in_selection()` and block bounds helpers. Update `handle_visual()` yank/delete for Block mode. Update hex_view.rs rendering to use mode-aware selection checks.

**Tech Stack:** Rust, hexcore (no-std), hexview (ratatui + crossterm)

## Global Constraints

- All existing tests must continue to pass
- `selection_start()`/`selection_end()` still return `Option<u64>` for Char/Line compatibility
- `in_selection()` signature changes — update all callers
- VisualBlock yank/delete iterates rows, produces flat byte array

---

### Task 1: Mode-aware selection helpers in Cursor

**Files:**
- Modify: `hexcore/src/cursor.rs:54-72`
- Test: `hexcore/src/cursor.rs` (inline tests)

**Interfaces:**
- Consumes: existing `Cursor` fields (`offset`, `sub_offset`, `selection_anchor`, `selection_sub_anchor`, `selection_mode`, `bytes_per_row`)
- Produces: `Cursor::row()`, `Cursor::block_bounds()`, updated `selection_start()`, `selection_end()`, `in_selection()`

- [ ] **Step 1: Write failing tests for Line and Block selection**

Add tests for:
- Line mode: selection spans full rows, `in_selection` works for all offsets in selected rows
- Block mode: `in_selection` returns true only within the rectangle, respects nibble mode

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p hexcore`
Expected: new tests fail (old tests still pass)

- [ ] **Step 3: Add helper methods to Cursor**

Add:
```rust
fn row(&self, offset: u64) -> u64 {
    if self.bytes_per_row == 0 { return 0; }
    offset / self.bytes_per_row
}

fn anchor_row(&self) -> u64 {
    self.selection_anchor.map_or(0, |a| a / self.bytes_per_row)
}

fn cursor_row(&self) -> u64 {
    self.offset / self.bytes_per_row
}
```

- [ ] **Step 4: Update `selection_start()` / `selection_end()` for Line mode**

```rust
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

pub fn selection_end(&self) -> Option<u64> {
    match self.selection_mode {
        SelectionMode::Line => {
            let anchor = self.selection_anchor?;
            let end_row = self.row(anchor).max(self.row(self.offset));
            Some(end_row * self.bytes_per_row + self.bytes_per_row - 1)
        }
        _ => {
            let anchor = self.selection_anchor?;
            Some(anchor.max(self.offset))
        }
    }
}
```

- [ ] **Step 5: Add `block_bounds()` for Block mode rectangle**

```rust
pub fn block_bounds(&self, nibble_mode: bool) -> (u64, u64, u64, u64) {
    let anchor = self.selection_anchor.unwrap_or(self.offset);
    let anchor_col = if nibble_mode {
        anchor * 2 + self.selection_sub_anchor.unwrap_or(0) as u64
    } else {
        anchor % self.bytes_per_row
    };
    let cursor_col = if nibble_mode {
        self.offset * 2 + self.sub_offset as u64
    } else {
        self.offset % self.bytes_per_row
    };
    let top = self.row(anchor).min(self.row(self.offset));
    let bottom = self.row(anchor).max(self.row(self.offset));
    let left = anchor_col.min(cursor_col);
    let right = anchor_col.max(cursor_col);
    (top, bottom, left, right)
}
```

- [ ] **Step 6: Update `in_selection()` for Block mode**

Signature changes to accept `file_size: u64, nibble_mode: bool, sub_offset: u8`:

```rust
pub fn in_selection(&self, offset: u64, file_size: u64, nibble_mode: bool, sub_offset: u8) -> bool {
    match self.selection_mode {
        SelectionMode::Block => {
            let (top, bottom, left, right) = self.block_bounds(nibble_mode);
            let off_row = self.row(offset);
            if off_row < top || off_row > bottom { return false; }
            if nibble_mode {
                let off_col = offset * 2 + sub_offset as u64;
                off_col >= left && off_col <= right
            } else {
                let off_col = offset % self.bytes_per_row;
                off_col >= left && off_col <= right
            }
        }
        SelectionMode::None => false,
        _ => {
            let Some(start) = self.selection_start() else { return false; };
            let Some(end) = self.selection_end() else { return false; };
            offset >= start && offset <= end
        }
    }
}
```

- [ ] **Step 7: Run tests to verify all pass**

Run: `cargo test -p hexcore`
Expected: 0 failures

- [ ] **Step 8: Commit**

```bash
git add hexcore/src/cursor.rs
git commit -m "feat(cursor): mode-aware selection helpers for Line and Block"
```

---

### Task 2: Update hex_view.rs rendering

**Files:**
- Modify: `hexview/src/ui/hex_view.rs:54,97`

**Interfaces:**
- Consumes: `Cursor::in_selection(offset, file_size, nibble_mode, sub_offset)`
- Produces: correct visual highlighting for all three visual modes

- [ ] **Step 1: Update `in_selection()` calls in hex_view.rs**

Line 54 and line 97 both call `app.cursor.in_selection(byte_off)`. Update to:
```rust
let in_sel = app.cursor.in_selection(byte_off, app.buffer.len(), app.nibble_mode, 0);
```

The `sub_offset` param is 0 for hex_view because we check byte-level offset; nibble granularity is only relevant for Block mode column comparison.

- [ ] **Step 2: Run tests to verify compilation**

Run: `cargo test`
Expected: compilation succeeds

- [ ] **Step 3: Commit**

```bash
git add hexview/src/ui/hex_view.rs
git commit -m "feat(hex_view): pass nibble_mode to in_selection for Block rendering"
```

---

### Task 3: Update status_bar.rs selection count

**Files:**
- Modify: `hexview/src/ui/status_bar.rs:31-36`

**Interfaces:**
- Consumes: `Cursor` methods for selection size computation
- Produces: correct byte count in status bar for all modes

- [ ] **Step 1: Update selection count for Block mode**

For Block mode, count is: `(bottom - top + 1) * (right - left + 1)` for byte mode, or nibble-adjusted for nibble mode.

```rust
let selection_str = if app.cursor.selection_mode == SelectionMode::Block {
    let (top, bottom, left, right) = app.cursor.block_bounds(app.nibble_mode);
    let rows = bottom - top + 1;
    let cols = right - left + 1;
    let count = if app.nibble_mode {
        // Nibble block: each column is a nibble, so count = rows * cols, but
        // the display shows "Sel:NNN" in byte-equivalent terms for simplicity
        rows * cols
    } else {
        rows * cols
    };
    format!(" Sel:{}", count)
} else if let (Some(s), Some(e)) = (app.cursor.selection_start(), app.cursor.selection_end()) {
    format!(" Sel:{}", e - s + 1)
} else {
    String::new()
};
```

- [ ] **Step 2: Run tests**

Run: `cargo test`
Expected: 0 failures

- [ ] **Step 3: Commit**

```bash
git add hexview/src/ui/status_bar.rs
git commit -m "feat(status_bar): block-mode selection byte count"
```

---

### Task 4: Block-mode yank/delete in handle_visual

**Files:**
- Modify: `hexview/src/app.rs:609-709`

**Interfaces:**
- Consumes: `Cursor::block_bounds(nibble_mode)`, `ByteBuffer::read()`, `EditCommand::Delete/Overwrite`
- Produces: correct yank/delete for VisualBlock selection

- [ ] **Step 1: Write failing tests for block yank/delete**

- VisualBlock yank: verify clipboard contains only block bytes
- VisualBlock delete: verify only block bytes are zeroed or removed
- VisualBlock in nibble mode: verify nibble-level block granularity

- [ ] **Step 2: Extract block iteration helper in handle_visual**

```rust
fn collect_block_bytes(&self) -> Vec<u8> {
    let nibble = self.nibble_mode;
    let (top, bottom, left, right) = self.cursor.block_bounds(nibble);
    let mut result = Vec::new();
    for row in top..=bottom {
        let row_offset = row * self.cursor.bytes_per_row;
        if nibble {
            for nib_col in left..=right {
                let byte_off = row_offset + nib_col / 2;
                if byte_off >= self.buffer.len() { continue; }
                let byte = self.buffer.read(byte_off, 1).map(|b| b[0]).unwrap_or(0);
                let nibble_val = if nib_col % 2 == 0 { byte >> 4 } else { byte & 0x0F };
                result.push(nibble_val);
            }
        } else {
            for col in left..=right {
                let byte_off = row_offset + col;
                if byte_off >= self.buffer.len() { continue; }
                let byte = self.buffer.read(byte_off, 1).map(|b| b[0]).unwrap_or(0);
                result.push(byte);
            }
        }
    }
    result
}
```

- [ ] **Step 3: Update yank (`KeyCode::Char('y')`) in handle_visual**

For VisualBlock mode, use `collect_block_bytes()` instead of linear `selection_start()`/`selection_end()`.

- [ ] **Step 4: Update delete (`KeyCode::Char('d') | KeyCode::Char('x')`) in handle_visual**

For VisualBlock mode (byte mode): iterate rows and columns, zero out bytes with Overwrite commands.
For VisualBlock in nibble mode: iterate nibble positions, use `delete_nibble()`.

- [ ] **Step 5: Run tests to verify block tests pass**

Run: `cargo test`
Expected: all tests pass including new block tests

- [ ] **Step 6: Commit**

```bash
git add hexview/src/app.rs
git commit -m "feat(visual): block-mode yank and delete"
```

---

### Task 5: Update existing tests for new in_selection signature

**Files:**
- Modify: `hexcore/src/cursor.rs:130-140` (existing test)
- Maybe: `hexview/src/app.rs:1108-1257` (existing visual tests)

- [ ] **Step 1: Update test_selection_char_mode in cursor.rs**

The existing test calls `cursor.in_selection(3)` which now needs `file_size`, `nibble_mode`, `sub_offset` params.

```rust
assert!(cursor.in_selection(3, 100, false, 0));
```

- [ ] **Step 2: Run tests**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 3: Add integration tests for VisualLine and VisualBlock**

In app.rs tests: test VisualLine yank selects full rows, VisualBlock yank selects rectangle.

- [ ] **Step 4: Commit**

```bash
git add hexcore/src/cursor.rs hexview/src/app.rs
git commit -m "test: update tests for mode-aware selection"
```
