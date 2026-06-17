# Insert Mode: Always Insert, Never Overwrite

## Problem

Insert mode (`i`) overwrites existing bytes instead of inserting/shifting them right.
Root cause: `handle_insert()` in `app.rs` checks `config.use_overwrite_mode` (default `true`),
and uses `EditCommand::Overwrite` when the flag is set.

## Changes

### `hexcore/src/config.rs`
- Change `use_overwrite_mode` default from `true` to `false`
- Update test assertion accordingly

### `hexview/src/app.rs`
- Remove the `use_overwrite_mode` conditional branch from `handle_insert()`
  so Insert mode always uses `EditCommand::Insert` (shifts bytes right)
- Remove `app.config.use_overwrite_mode = false;` setup in `test_insert_mode`
  (the test already asserts correct insert behavior; the setup becomes redundant)

## Rationale

Insert mode should always insert. The `use_overwrite_mode` config field remains
in the codebase for potential use by Replace/Overwrite mode later, but it no
longer affects Insert mode behavior.
