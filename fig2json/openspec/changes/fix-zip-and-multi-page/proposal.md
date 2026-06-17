## Why

Two bugs prevent proper diagnosis and output of multi-page Figma files:

1. **Doctor command ignores ZIP files**: Figma's "Save local copy" produces ZIP-wrapped `.fig` files (magic header `PK`). The `doctor` command reads raw bytes without extracting `canvas.fig` from the ZIP, resulting in empty diagnostic output (0 nodes, 0 pages, 0 blobs).

2. **`--raw` (and regular convert) only exports the first page**: When a `.fig` file contains multiple pages, only the first page appears in the output. The root cause is unknown — the `build_tree` function starts from hardcoded root `"0:0"` and should recursively include all pages, but something in the pipeline (Kiwi decoding, parentIndex mapping, or tree building) drops subsequent pages.

## What Changes

- **Fix doctor ZIP handling**: Extract `canvas.fig` from ZIP containers before running diagnostic analysis, matching the behavior of the convert path.
- **Add diagnostic logging to `build_tree`**: Emit debug output showing total node count, root children count, and orphan count to identify where pages are lost.
- **Fix multi-page export**: Once diagnosed, fix the root cause so all pages appear in both `convert` and `convert_raw` output.

## Capabilities

### New Capabilities
- `doctor-zip-support`: Doctor command correctly handles ZIP-wrapped `.fig` files by extracting `canvas.fig` before analysis

### Modified Capabilities
- `doctor-command`: Add ZIP extraction step before diagnostic analysis (delta to existing spec)

## Impact

- **Files affected**: `src/doctor.rs`, `src/schema/tree.rs`, `src/lib.rs`
- **No new dependencies**: Uses existing `parser::extract_from_zip()`
- **Backward compatible**: Existing convert behavior unchanged; doctor now works on ZIP files
- **Diagnostic output**: `build_tree` will emit debug info to stderr when verbose flag is active
