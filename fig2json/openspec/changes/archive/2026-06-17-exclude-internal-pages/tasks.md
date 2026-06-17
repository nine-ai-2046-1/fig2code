## Tasks

- [x] 1. Filter pages after `get_all_canvases()` call at line 1137 in `src/main.rs`
  - Add: `let pages: Vec<&Value> = pages.into_iter().filter(|p| !node_name(p).contains("Internal Only")).collect();`

- [x] 2. Add test case for internal page filtering
  - Add `test_exclude_internal_pages` test with multi-page document containing "Internal Only Canvas"
