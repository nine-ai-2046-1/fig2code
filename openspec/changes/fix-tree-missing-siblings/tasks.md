## 1. Fix cmd_tree

- [ ] 1.1 Modify `cmd_tree` to detect if `data` is a page (canvas) and use it directly as root
- [ ] 1.2 Add test case for multi-screen page tree output

## 2. Verification

- [ ] 2.1 Run `cargo build` — ensure no compilation errors
- [ ] 2.2 Run `cargo test` — ensure no test failures
- [ ] 2.3 Manual test: run tree on demo-1 data, verify both screens appear
