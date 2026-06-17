## Why

When running `tree` command on a page with multiple top-level frames (screens), only the first frame is shown. The other frames are silently dropped. This is a critical bug because users expect to see ALL screens in the tree output.

**Root cause**: `cmd_tree` calls `get_canvas(data)` which assumes `data` is the full document. But when processing pages individually (with `-o` flag), `data` is already a page (canvas). `get_canvas(page)` returns the page's first child instead of the page itself.

## What Changes

- Fix `cmd_tree` to use the page directly as root when no `-l` layer is specified
- Ensure all top-level frames (screens) in a page are shown in tree output

## Capabilities

### New Capabilities

(none — bug fix)

### Modified Capabilities

(none — no spec changes, just behavior fix)

## Impact

- **Code**: `src/main.rs` — `cmd_tree` function logic
- **Behavior**: Tree command now shows all screens in a page, not just the first one
- **Backward compatibility**: Fix is backward-compatible; existing single-screen pages work identically
