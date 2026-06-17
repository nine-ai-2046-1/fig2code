## Context

The `fig2json` tool has two issues with multi-page Figma files:

1. **Doctor ZIP gap**: The `doctor` command (`src/doctor.rs`) reads raw bytes and returns empty results for ZIP-wrapped `.fig` files. Every `collect_*` function has `if parser::is_zip_container(bytes) { return empty; }`. The convert path handles this via `parser::extract_from_zip()` but doctor does not.

2. **Missing pages**: `build_tree()` in `src/schema/tree.rs:25` starts from hardcoded root `"0:0"` and recursively builds children. When a `.fig` file has 2+ pages, only 1 appears in output. The pipeline is identical for `convert` and `convert_raw`, so the bug is in tree building or upstream decoding.

Current convert flow: `bytes → extract_from_zip → extract_chunks → decompress → decode_fig_to_json → build_tree(node_changes)`
Current doctor flow: `bytes → (bail if ZIP) → extract_chunks → ...`

## Goals / Non-Goals

**Goals:**
- Fix doctor to extract `canvas.fig` from ZIP before running diagnostics
- Add diagnostic logging to `build_tree` to identify why pages are missing
- Fix the root cause so all pages appear in output
- Maintain backward compatibility (existing CLI behavior preserved)

**Non-Goals:**
- Changing the Kiwi schema decoding
- Restructuring the parser module
- Adding new CLI flags beyond what's needed for diagnostics

## Decisions

### 1. Doctor ZIP handling: extract bytes upfront

**Decision**: Add a single ZIP extraction step at the top of `run_doctor()` before calling any `collect_*` functions. This avoids duplicating ZIP logic across 6+ functions.

```rust
// In run_doctor():
let bytes = if parser::is_zip_container(&bytes) {
    parser::extract_from_zip(&bytes)?
} else {
    bytes
};
```

**Rationale**: Minimal change, one extraction point, all `collect_*` functions receive raw `.fig` bytes. Alternative considered: adding ZIP handling to each `collect_*` function — rejected as repetitive.

### 2. Page diagnosis: add debug eprintln to build_tree

**Decision**: Add `eprintln!` statements to `build_tree()` to log:
- Total node count
- Root `"0:0"` existence and its children count
- All nodes without parentIndex (orphans)
- All unique parentIndex values

**Rationale**: Quick diagnostic, no CLI flag needed, outputs to stderr so stdout stays clean.

### 3. Multi-page fix: TBD based on diagnosis

**Three possible root causes identified**:

| Hypothesis | Symptom | Fix |
|-----------|---------|-----|
| Page 2 not in nodeChanges | Kiwi decode skips it | Fix decoder or schema |
| Page 2 has no parentIndex | Not attached to tree | Add root detection for parentless nodes |
| Page 2 parentIndex ≠ "0:0" | Attached to wrong parent | Adjust root detection |

The diagnosis step will determine which hypothesis is correct, then the fix applies.

## Risks / Trade-offs

- **ZIP extraction failure**: If ZIP contains no `canvas.fig`, doctor will error. Mitigation: provide clear error message with ZIP contents listed.
- **Diagnostic noise**: `eprintln` in `build_tree` will print on every conversion. Mitigation: Only print when `FIG2JSON_DEBUG` env var is set.
- **Root cause uncertainty**: The page fix depends on diagnosis. If the issue is in Kiwi decoding (upstream), we may not be able to fix it in this change.

## Migration Plan

- No migration needed — all changes are internal
- Doctor command will now work on ZIP files (previously returned empty)
- `build_tree` behavior unchanged except debug output when env var set
