# vim-hex: A Vim-inspired Terminal Hex Editor

**Date:** 2026-06-15
**Language:** Rust
**Stack:** ratatui + crossterm + memmap2 + serde

## Architecture

Two-crate workspace — `hexcore` (pure library, no terminal deps) + `hexview` (TUI binary).

## Features

- Open/save files, hex + ASCII view, vim modal keybindings
- Edit (overwrite, insert, delete), undo/redo (depth 5000)
- Search: text (case-sensitive/insensitive), hex patterns (`??` wildcard)
- Go-to-offset, copy/paste yanked bytes
- Multiple files via tabs
- Endianness display modes (u16/u32/u64 LE/BE preview in info line)
- Large file support via mmap + sparse overlay (threshold: 500 MB)
- Command bar (`:` for commands, `/` for search, `?` for reverse search)

## Core Data Model (hexcore)

| Component | Responsibility |
|---|---|
| `ByteBuffer` | Owns raw bytes. Under 500MB loads into `Vec<u8>`. Over 500MB uses memmap2 with copy-on-write overlay. |
| `Cursor` | Tracks 64-bit offset, selection start, selection direction, selection mode (None/Char/Line/Block). |
| `EditCommand` | Enum of every edit with inverse data for undo. Variants: `Overwrite`, `Insert`, `Delete`, `Fill`. |
| `UndoManager` | Two stacks (undo/redo) of `EditCommand`. Bounded configurable depth (default 5000). |
| `Searcher` | Case-sensitive and insensitive text search; hex pattern search with `??` wildcard. Returns `Vec<u64>`. |
| `FileIo` | Open, save, save-as helpers. |
| `Config` | Serialized settings: undo depth, bytes per row, mmap threshold, etc. |

## TUI Frontend Layout (hexview)

Four visual regions:
1. **Status bar** — filename, mode indicator, dirty flag, cursor offset
2. **Hex view** — offset column (8 hex digits) | byte column (grouped every 8) | ASCII column
3. **Command/search bar** — appears on `:`, `/`, `?`
4. **Info line** — mode, offset, selection size, encoding, endianness preview

## Keyboard Model

Modes: Normal, Insert, Visual (char/line/block), Command-line, Search

### Normal Mode
| Key | Action |
|---|---|
| `h`/`j`/`k`/`l` | Move cursor (1 byte / 1 row / -1 row / 1 byte) |
| `Ctrl-D`/`Ctrl-U` | Page down/up (half screen) |
| `gg` / `G` | Jump to start / end |
| `:N` or `N G` | Go to offset N |
| `x` | Delete byte under cursor |
| `dd` | Delete 16-byte row under cursor |
| `i` | Enter Insert mode |
| `v` | Enter Visual mode (charwise selection) |
| `V` | Visual Line mode |
| `Ctrl-V` | Visual Block mode |
| `y` | Yank selected bytes |
| `p` / `P` | Paste after / before cursor |
| `u` / `Ctrl-R` | Undo / Redo |
| `/` / `?` | Forward / reverse search |
| `n` / `N` | Next / previous match |
| `:` | Enter Command-line mode |

### Insert Mode
| Key | Action |
|---|---|
| Typing | Overwrites or inserts bytes (configurable) |
| `Esc` | Back to Normal mode |

### Visual Char/Line/Block Mode
| Key | Action |
|---|---|
| `h`/`j`/`k`/`l` | Extend selection |
| `d` | Delete selection |
| `x` | Cut selection |
| `y` | Yank |
| `Esc` | Exit to Normal mode |

### Command-line Mode
| Command | Action |
|---|---|
| `:w` | Save |
| `:w <path>` | Save as |
| `:q` | Quit (or `:q!` to force) |
| `:wq` | Save and quit |
| `:e <path>` | Open file |
| `:%s/<hex>/<replacement>` | Search and replace (hex patterns) |

## Large File Strategy

Files under 500 MB are fully loaded into memory. For larger files, use memmap2 for read-only access with a sparse overlay of changed bytes. On save, write original + overlay to temp file and atomically rename.

## Project Structure

```
hexeditor/
├── Cargo.toml (workspace)
├── .gitignore
├── hexcore/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── buffer.rs
│       ├── cursor.rs
│       ├── commands.rs
│       ├── undo.rs
│       ├── search.rs
│       ├── file_io.rs
│       └── config.rs
└── hexview/
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── app.rs
        ├── ui.rs
        ├── input.rs
        ├── status_bar.rs
        ├── hex_view.rs
        ├── command_bar.rs
        └── tabs.rs
```
