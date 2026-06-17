## 1. CLI Restructure

- [~] 1.1 Create `Command` enum with `Convert` (default) and `Doctor` variants using `clap::Subcommand` (skipped)
- [x] 1.2 Move existing convert logic into a `run_convert` function
- [x] 1.3 Update `main()` to dispatch based on subcommand (convert as default, doctor)
- [x] 1.4 Verify existing CLI usage still works (no subcommand = convert mode)

## 2. Doctor Module

- [x] 2.1 Create `src/doctor.rs` module with public `run_doctor` function
- [x] 2.2 Implement `collect_file_metadata` — reads file size, magic header, version, chunk count
- [x] 2.3 Implement `collect_schema_info` — decodes Kiwi schema, returns definition count and root type
- [x] 2.4 Implement `collect_node_changes_summary` — counts total nodes and type distribution
- [x] 2.5 Implement `collect_page_breakdown` — builds tree, lists pages with child counts, counts orphans
- [x] 2.6 Implement `collect_blob_summary` — counts blobs and their types
- [x] 2.7 Implement `collect_tree_stats` — computes max depth, total reachable nodes, nodes per page
- [x] 2.8 Implement `format_doctor_output` — assembles all sections into human-readable text

## 3. Output Handling

- [x] 3.1 Implement writing output to `doctor.log` in current working directory
- [x] 3.2 Support `--verbose` flag for additional detail (first 10 GUIDs, first 5 blobs)
- [x] 3.3 Print summary to stderr (e.g., "Doctor log written to doctor.log")

## 4. Integration & Testing

- [x] 4.1 Wire `doctor.rs` into `main.rs` dispatch
- [x] 4.2 Add unit tests for each `collect_*` function in `doctor.rs`
- [x] 4.3 Add integration test: run doctor on a minimal `.fig` file, verify `doctor.log` output
- [x] 4.4 Test verbose vs non-verbose output
- [x] 4.5 Test error cases: missing file, non-existent file, corrupt file
- [x] 4.6 Run `cargo clippy` and `cargo test` to verify no regressions
