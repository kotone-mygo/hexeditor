# hexeditor

[![CI](https://github.com/kotone-mygo/hexeditor/actions/workflows/ci.yml/badge.svg)](https://github.com/kotone-mygo/hexeditor/actions/workflows/ci.yml)

A terminal-based hex editor built with Rust, [ratatui](https://github.com/ratatui-org/ratatui), and [crossterm](https://github.com/crossterm-rs/crossterm).

## Features

- Hex and ASCII view with cursor navigation
- Byte-level and nibble-level (4-bit) editing
- Undo / redo with cursor position tracking
- Jump list (position history) — navigate with `Ctrl-O` / `Tab`
- Text and hex search with regex-like wildcards
- Persistent search result counter — always visible on the command line
- Search-and-replace (`:%s/find/replace`)
- Visual mode for selection, yank, and delete
- Clipboard (yank/paste)
- Multiple file tabs
- Nibble mode (edit high/low nybble independently)
- Configurable bytes-per-row and undo depth
- Endian value preview (u16 LE/BE) in status bar
- File encoding detection (UTF-8, UTF-16LE, Binary)
- Persistent configuration file (JSON)

## Installation

### Build from source

```bash
git clone https://github.com/kotone-mygo/hexeditor.git
cd hexeditor
cargo build --release
```

The binary is at `target/release/hedit`.

### Run

```bash
hedit <file>
```

Or with `cargo`:

```bash
cargo run -- <file>
```

## Key Bindings

### Normal mode

| Key | Action |
|-----|--------|
| `h`/`←` `l`/`→` `j`/`↓` `k`/`↑` | Move cursor |
| `0` / `$` | Go to row start / end |
| `gg` / `G` | Go to top / bottom |
| `Ctrl-D` / `Ctrl-U` | Page down / up |
| `i` / `a` | Insert at cursor / after cursor |
| `r` / `R` | Replace once / continuous replace |
| `x` / `dd` | Delete byte / delete row |
| `y` / `p` / `P` | Yank / paste after / paste before |
| `u` / `Ctrl-R` | Undo / redo |
| `Ctrl-O` / `Tab` | Jump back / forward in history |
| `/`  `?` | Search forward / backward |
| `n` / `N` | Next / previous search result |
| `:` | Enter command mode |
| `v` / `V` / `Ctrl-V` | Visual char / line / block |
| `z` | Toggle nibble mode (4-bit editing) |

### Insert mode

| Key | Action |
|-----|--------|
| `<char>` | Insert byte at cursor |
| `Esc` | Return to Normal |
| `Backspace` | Delete byte before cursor |

### Replace mode

| Key | Action |
|-----|--------|
| `<char>` | Overwrite byte, advance cursor |
| `Esc` | Return to Normal |

### Visual mode

| Key | Action |
|-----|--------|
| `h`/`j`/`k`/`l` | Extend selection |
| `d` / `x` / `y` | Delete / yank selection |
| `Esc` | Cancel selection |

### Command mode

| Command | Action |
|---------|--------|
| `:w` | Save file |
| `:w <path>` | Save as |
| `:e <path>` | Open file |
| `:q` / `:q!` | Quit |
| `:wq` | Save and quit |
| `:%s/find/replace` | Replace hex pattern |
| `:h` / `:help` | Open help view |
| `j` / `k` | Scroll help overlay |
| `:q` | Close help overlay |
| `Esc` | Cancel, return to Normal |

### Search mode

| Key | Action |
|-----|--------|
| `<query> Enter` | Search text or hex (`0x` prefix) |
| `Esc` | Cancel |

A hex search pattern (`0x`) may use `??` as a single-nibble wildcard.

## Architecture

The project is a Cargo workspace with two crates:

- **`hexcore`** — Library crate with no terminal dependencies. Contains data structures and logic: `ByteBuffer`, `Cursor`, `UndoManager`, `EditCommand`, `JumpList`, `Searcher`, `Config`, `FileIo`.
- **`hexview`** — Binary crate. Terminal UI with ratatui/crossterm. Contains the `App` state machine, key handlers, and UI rendering modules.

## Build

```bash
cargo build              # debug build
cargo build --release    # release build
cargo test               # run all tests
```

## Configuration

Settings are persisted to `~/.config/hexview/config.json`. Example:

```json
{
  "bytes_per_row": 16,
  "max_undo_depth": 500,
  "show_ascii": true,
  "use_overwrite_mode": false
}
```

Options:
- `bytes_per_row` — Number of bytes displayed per row (default: `16`)
- `max_undo_depth` — Maximum undo history entries (default: `5000`)
- `show_ascii` — Show ASCII panel alongside hex (default: `true`)
- `mmap_threshold_mb` — File size threshold in MB for memory-mapped I/O (default: `500`)
- `use_overwrite_mode` — Start in overwrite mode instead of insert (default: `false`)

## License

MIT
