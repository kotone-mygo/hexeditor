# hexeditor — Agent Guide

## Overview

Terminal hex editor in Rust. Two-crate workspace: `hexcore` (library, no terminal deps) + `hexview` (binary, ratatui + crossterm).

## Build & Test

```bash
cargo build              # debug
cargo build --release    # release
cargo test               # all tests (hexcore + hexview)
cargo run -- <file>      # run the editor
```

## Architecture

### crate `hexcore` (`hexcore/src/`)

Pure data/logic, no terminal dependencies.

| Module | Purpose |
|--------|---------|
| `buffer.rs` | `ByteBuffer` — owns raw bytes, read/write/insert/delete, nibble operations, `is_empty()` |
| `cursor.rs` | `Cursor` — offset, sub_offset, selection state, movement helpers, `current_row_end()` for `$` key |
| `commands.rs` | `EditCommand` enum (`Overwrite`, `Insert`, `Delete`) — command pattern |
| `undo.rs` | `UndoManager` — two-stack undo/redo, returns offset after undo/redo for cursor jump |
| `jump_list.rs` | `JumpList` — position history for `Ctrl-O`/`Tab` navigation |
| `search.rs` | `Searcher` — text and hex search with `??` nibble wildcard; `parse_hex_pattern()` and `hex_to_bytes()` helpers |
| `file_io.rs` | `FileIo` — open/save with encoding detection (UTF-8, UTF-16, binary) |
| `config.rs` | `Config` — serialized settings (bytes_per_row, max_undo_depth, mmap_threshold_mb, show_ascii, use_overwrite_mode) |
| `lib.rs` | Re-exports public API |

### crate `hexview` (`hexview/src/`)

Binary name: `hedit` (declared in `Cargo.toml` `[[bin]]`).

Terminal UI (depends on `hexcore`, `ratatui`, `crossterm`, `unicode-width`).

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point, event loop |
| `app.rs` | `App` struct (`nibble_mode`, `jump_list`, `show_help`, `help_scroll`, `help_lines`, `status_message`, `show_config`, `config_selection`, `config_lines`), `TabInfo` struct, `Mode` enum (`Normal`, `Insert`, `Replace`, `ReplaceOnce`, `VisualChar`, `VisualLine`, `VisualBlock`, `Command`, `Search`), key handlers, command parsing, search, help builder, config panel handler |
| `ui/mod.rs` | Layout orchestrator |
| `ui/hex_view.rs` | Hex/ASCII rendering |
| `ui/config_view.rs` | Config panel overlay — adjust settings at runtime |
| `ui/status_bar.rs` | Bottom status line (mode, offset, endian preview) |
| `ui/command_bar.rs` | Command/search input line — always rendered, acts as spacer in normal mode, shows persistent search result counter |
| `ui/help_view.rs` | Help overlay |
| `ui/tabs.rs` | Tab bar |

## Key Patterns

### Command pattern (undo/redo)

`EditCommand` stores offset + old/new bytes. `UndoManager` maintains two stacks. `undo()`/`redo()` return the offset for cursor repositioning.

```rust
pub enum EditCommand {
    Overwrite { offset: u64, old_bytes: Vec<u8>, new_bytes: Vec<u8> },
    Insert { offset: u64, bytes: Vec<u8> },
    Delete { offset: u64, bytes: Vec<u8> },
}

impl EditCommand {
    pub fn offset(&self) -> u64 { ... }
}
```

### Jump list

`JumpList` stores a vector of `u64` offsets. `index` always points past the last entry (`entries.len()`). `back()` decrements then returns, `forward()` returns the entry at the incremented index (requires an entry ahead). `push()` truncates forward history, avoids consecutive duplicates, enforces max_size by removing from front. `saved_target: Option<u64>` enables round-trip navigation — `back()` saves the current position so `forward()` can return to it.

### Cursor

Public fields on `Cursor` (`offset`, `sub_offset`, `bytes_per_row`, `selection_anchor`, `selection_sub_anchor: Option<u8>`, `selection_mode`). Nibble mode adds a sub-byte offset (0 = high nybble, 1 = low nybble). Movement methods clamp to `[0, file_size-1]`.

Selection helpers are mode-aware:
- `selection_start()`/`selection_end()` — `Char`: linear min/max; `Line`: aligned to full row boundaries; `Block`: top-left/bottom-right corners
- `in_selection(offset, file_size, nibble_mode, sub_offset)` — `Block` checks row+column rectangle, `Char`/`Line` checks linear span
- `block_bounds(nibble_mode)` — returns `(top, bottom, left, right)` rectangle for Block mode
- `row(offset)` — returns row number for a byte offset

### Error handling

Key handlers return `Result<(), String>`. Errors propagate via `?` and are displayed in the status bar.

## Conventions

- Rust edition 2021
- 4-space indentation
- `#[cfg(test)] mod tests` inline in each module
- Public fields on structs (no getters/setters)
- serde for config serialization (JSON)
- `use` statements grouped: std → external → crate

## Keybinding Updates

Whenever adding or changing keybindings, the in-app help menu (`build_help_lines()` in `app.rs`) must be updated to match. The help view is the primary reference for users and commonly drifts out of sync with the actual key handlers.
