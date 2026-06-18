# Visual Mode Differentiation — Design

## Problem

The hex editor defines three visual mode variants (`VisualChar`, `VisualLine`, `VisualBlock`) with distinct keybindings (`v`, `V`, `Ctrl-V`) and distinct status bar labels, but `in_selection()`, `selection_start()`, and `selection_end()` in the `Cursor` treat all three identically as a linear byte range. All three modes render with the same highlight and behave identically during yank/delete operations.

## Changes

### 1. `hexcore/src/cursor.rs` — Mode-aware selection helpers

**`selection_start()` / `selection_end()`** become `SelectionMode`-aware:

- **`SelectionMode::Char`** (unchanged): linear range between anchor and offset.
- **`SelectionMode::Line`**: align start/end to full row boundaries. Start = beginning of anchor's row (or cursor's row, whichever is earlier). End = last byte of the later row. The cursor offset (column position) stays unchanged — `V` does not snap cursor to row start.
- **`SelectionMode::Block`**: return the top-left and bottom-right corners of the rectangular block defined by anchor and cursor positions. Columns are byte-based in byte mode, nibble-based in nibble mode.

**`in_selection(offset, file_size, nibble_mode, sub_offset)`** — Mode-aware check:

- `Char`: unchanged (`offset >= start && offset <= end`).
- `Line`: `true` if offset falls on any row between anchor's row and cursor's row.
- `Block`: `true` if offset's row and column fit within the rectangle. In nibble mode, evaluates column at the nibble level (sub_offset determines which nibble of the byte).

**New: `selection_block_bounds()`** returns `(top_row, bottom_row, left_col, right_col)` for Block mode, where column granularity depends on nibble mode.

**New: `selection_anchor_row()` / `selection_cursor_row()`** helpers for row-based calculations.

### 2. `hexview/src/app.rs` — Mode-aware yank/delete

**VisualLine yank/delete**: Use the expanded `selection_start()`/`selection_end()` (whole-row bounds) with the same contiguous range logic.

**VisualBlock yank/delete**: Iterate over each row in the block rectangle, collect/operate on bytes (or nibbles) within the column range. Yank produces a flat array row-by-row, left-to-right within each row. Nibble-mode block yank/delete operates on nibble positions.

### 3. `hexview/src/ui/hex_view.rs` — Mode-aware rendering

- **VisualBlock**: Pass nibble_mode + sub_offset to `in_selection()` so only the rectangular region highlights. In nibble mode, highlight individual nibbles.
- **VisualLine**: Already renders correctly since `in_selection()` returns whole rows — just needs the updated `in_selection()`.
- **VisualChar** (unchanged).

### 4. `hexview/src/ui/status_bar.rs` — Block selection count

Update the "Sel:" count for Block mode to count only bytes within the block rectangle, not the entire linear span.

## Out of scope

- VisualBlock paste with columnar alignment (Vim's `p` on block yank). Clipboard is a flat byte array regardless of mode.
- VisualBlock insert mode.
- Multiple cursors.
