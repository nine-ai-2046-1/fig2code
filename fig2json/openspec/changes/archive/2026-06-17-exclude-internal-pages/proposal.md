## Why

When parsing `canvas.raw.json`, the tool processes all pages including "Internal Only Canvas". This page contains internal design artifacts (brushes, etc.) that are not relevant to the output. Users expect only meaningful pages (like "demo-1" with screens) to be processed.

## What Changes

- Filter out pages where name contains "Internal Only" before processing
- Apply filter to all commands (tree, texts, images, etc.) and the `all` command

## Capabilities

### New Capabilities

(none — filtering behavior)

### Modified Capabilities

(none)

## Impact

- **Code**: `src/main.rs` — filter `pages` vector after `get_all_canvases()`
- **Behavior**: "Internal Only Canvas" page excluded from all output
