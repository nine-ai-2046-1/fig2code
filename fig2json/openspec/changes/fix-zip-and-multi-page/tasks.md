## 1. Doctor ZIP Support

- [x] 1.1 Add ZIP extraction step at the top of `run_doctor()` in `src/doctor.rs` — extract `canvas.fig` from ZIP before calling `collect_*` functions
- [x] 1.2 Update `collect_file_metadata()` to show "Container type: ZIP" and extracted size when input is extracted from ZIP
- [x] 1.3 Add unit test for doctor on a mock ZIP-wrapped `.fig` file (create a small ZIP in-memory with `canvas.fig` entry)
- [x] 1.4 Add unit test for doctor on a ZIP with no `canvas.fig` (verify error message)

## 2. build_tree Diagnostic Logging

- [x] 2.1 Add `FIG2JSON_DEBUG` env var check to `build_tree()` in `src/schema/tree.rs`
- [x] 2.2 Add `eprintln!` for total node count, root existence, root children count
- [x] 2.3 Add orphan detection: nodes with no `parentIndex` that are not `"0:0"`
- [x] 2.4 Add unit test for debug output (set env var, capture stderr)

## 3. Multi-Page Export Fix

- [x] 3.1 Run `FIG2JSON_DEBUG=1 fig2json test/poc-verifier.fig -o /dev/null` to diagnose root cause
- [x] 3.2 Based on diagnosis, implement fix in `build_tree()` or upstream
- [x] 3.3 Add unit test for multi-page tree building (2 pages with parentIndex to root)
- [x] 3.4 Verify `test/poc-verifier.fig` exports both pages in `--raw` output

## 4. Verification

- [x] 4.1 Run `cargo clippy` — no warnings
- [x] 4.2 Run `cargo test` — all tests pass
- [x] 4.3 Manual test: `fig2json doctor test/poc-verifier.fig` shows page count ≥ 2
- [x] 4.4 Manual test: `fig2json test/poc-verifier.fig --raw -o /tmp/test.json` shows both pages in output
