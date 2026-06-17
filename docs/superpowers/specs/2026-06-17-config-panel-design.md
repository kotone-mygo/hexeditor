# Config Panel (`=` key)

## Problem

Users need to temporarily adjust hex editor configuration at runtime without editing JSON config files and restarting.

## Design

Press `=` in Normal mode to open a config panel overlay (replaces hex view area, same pattern as help view). Config changes are in-memory only — reverted on restart.

### Config items (all 5 from `Config` struct)

| Field | Type | Default | Step | Range |
|-------|------|---------|------|-------|
| `bytes_per_row` | u64 | 16 | 1 | [1, 64] |
| `show_ascii` | bool | true | toggle | - |
| `max_undo_depth` | usize | 5000 | 100 | [1, ∞) |
| `mmap_threshold_mb` | u64 | 500 | 100 | [1, ∞) |
| `use_overwrite_mode` | bool | false | toggle | - |

### Key bindings (config panel open)

| Key | Action |
|-----|--------|
| `j`/`k` or `↓`/`↑` | Navigate items |
| `Enter` | Toggle boolean fields |
| `+` | Increment numeric field |
| `-` | Decrement numeric field |
| `Esc` | Close panel |

### Side effects

- Changing `bytes_per_row` also updates `app.cursor.bytes_per_row`
- Changing `max_undo_depth` updates `app.undo.max_depth`
- No config is persisted to disk

### Files changed

- `hexcore/src/undo.rs` — add `set_max_depth()` method
- `hexview/src/app.rs` — add `show_config`, `config_selection` fields, `handle_config()` key routing, `build_config_lines()`, `=` key in normal mode
- `hexview/src/ui/config_view.rs` — new renderer
- `hexview/src/ui/mod.rs` — layout routing for config panel
