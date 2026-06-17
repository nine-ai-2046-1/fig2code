## Context

The `cmd_tree` function is called in two contexts:
1. **Full document mode** (no `-o`): `data` is the full document → `get_canvas(data)` returns first page
2. **Page mode** (with `-o`): `data` is a page → `get_canvas(data)` incorrectly returns first child

The bug is that context 2 is broken.

## Goals / Non-Goals

**Goals:**
- Fix `cmd_tree` to work correctly in both contexts
- Show all top-level frames when processing a page directly

**Non-Goals:**
- Changing behavior for full document mode (already works)
- Adding new flags or options

## Decisions

### D1: Check if data is a page or full document

**Choice**: Detect whether `data` is a page (has `type: CANVAS`) or a full document, and use appropriate root.

**Why**: Simple, minimal change. The page has a `type` field with value `CANVAS`. If `data.type == CANVAS`, use `data` directly as root. Otherwise, call `get_canvas(data)`.

### D2: Alternative — always use data directly

**Choice**: Since `cmd_tree` now always receives a page (canvas) in the `-o` code path, and the full document code path uses `get_canvas` before calling `cmd_tree`, we could just use `data` directly.

**Why**: Simpler, but need to verify the full document code path doesn't call `cmd_tree` directly.

## Risks / Trade-offs

- **[Minimal change]** → Fix is small and focused, low risk of regression
- **[Test coverage]** → Should add test case for multi-screen pages
