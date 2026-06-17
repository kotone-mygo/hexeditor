# GitHub Actions CI/CD Workflow Design

## Overview

Single GitHub Actions workflow (`.github/workflows/ci.yml`) for the hexeditor Rust project. Cross-platform builds on Linux, macOS, and Windows. Version tag pushes trigger a release.

## Trigger

- `push` on any branch and tags matching `v*`
- `pull_request` on any branch

## Jobs

### `build` (always runs)

Runs on every push/PR across 3 OS (`ubuntu-latest`, `macos-latest`, `windows-latest`).

Steps:
1. `actions/checkout@v4`
2. `actions-rust-lang/setup-rust-toolchain@v1` with `clippy` component
3. `cargo clippy --all-targets -- -D warnings`
4. `cargo test`
5. `cargo build --release`

If the push is a version tag (`startsWith(github.ref, 'refs/tags/v')`):
6. Rename the release binary to include the OS name
7. Upload as a workflow artifact (`actions/upload-artifact@v4`)

### `release` (tag pushes only)

Runs only on version tag pushes. Depends on `build`. Single job on `ubuntu-latest`.

Steps:
1. `actions/download-artifact@v4` — downloads all artifacts from `build` job
2. `softprops/action-gh-release@v2` — creates a GitHub Release and attaches all artifact files

## Artifact naming

- `hedit-Linux` — Ubuntu build binary
- `hedit-macOS` — macOS build binary
- `hedit-Windows.exe` — Windows build binary

## Release output

When a tag like `v0.1.0` is pushed, the workflow creates a GitHub Release with:
- Release notes auto-generated from commits
- Three binary attachments (one per platform)
