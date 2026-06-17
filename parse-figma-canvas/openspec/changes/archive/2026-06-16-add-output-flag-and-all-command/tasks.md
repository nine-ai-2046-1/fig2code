## 1. CLI Struct Changes

- [x] 1.1 Add `output: Option<PathBuf>` field to `Cli` struct with `-o` short flag and `long` attribute
- [x] 1.2 Add `All` variant to `Commands` enum (no fields)

## 2. Refactor Output Functions

- [x] 2.1 Refactor `cmd_layers` to accept `out: &mut dyn Write` parameter, replace all `println!` with `writeln!`
- [x] 2.2 Refactor `print_tree` and `cmd_tree` to accept `out: &mut dyn Write`, replace all `println!` with `writeln!`
- [x] 2.3 Refactor `print_fills` and `cmd_node` to accept `out: &mut dyn Write`, replace all `println!` and `print!` with `writeln!` and `write!`
- [x] 2.4 Refactor `cmd_texts` and its inner `walk_texts` to accept `out: &mut dyn Write`
- [x] 2.5 Refactor `cmd_images` and its inner `walk_images` to accept `out: &mut dyn Write`
- [x] 2.6 Refactor `cmd_interactions` and its inner `walk_interactions` to accept `out: &mut dyn Write`
- [x] 2.7 Refactor `cmd_tokens` and its inner `walk_tokens` to accept `out: &mut dyn Write`
- [x] 2.8 Refactor `cmd_raw` to accept `out: &mut dyn Write`

## 3. Output Routing Helpers

- [x] 3.1 Create `fn cmd_filename(cmd: &Commands) -> &'static str` to map command names to filenames
- [x] 3.2 Create `fn run_cmd(cmd: &Commands, data: &Value, out: &mut dyn Write)` dispatcher that calls appropriate cmd function
- [x] 3.3 Create `fn execute_to_file(cmd: &Commands, data: &Value, output_dir: &Path)` that opens file and calls `run_cmd`

## 4. All Command Implementation

- [x] 4.1 Create `fn run_all(data: &Value, output_dir: &Path)` that iterates over all commands (tree, texts, images, interactions, tokens, layers)
- [x] 4.2 Add error handling in `run_all` — catch failures, log to stderr, continue with remaining commands
- [x] 4.3 Add validation: `all` command requires `-o` flag, error if missing

## 5. Main Function Integration

- [x] 5.1 Update `main()` to create output directory if `-o` is specified
- [x] 5.2 Route single commands through output logic: if `-o` present, save to file; else print to stdout
- [x] 5.3 Route `all` command to `run_all` function
- [x] 5.4 Update `match` block to pass `out` parameter to all `cmd_xxx` calls

## 6. Verification

- [x] 6.1 Run `cargo build` — ensure no compilation errors
- [x] 6.2 Run `cargo test` — ensure no test failures
- [x] 6.3 Manual test: `parse-figma-canvas -o /tmp/test tree` — verify file created
- [x] 6.4 Manual test: `parse-figma-canvas tree` — verify stdout output unchanged
- [x] 6.5 Manual test: `parse-figma-canvas -o /tmp/test all` — verify all files created
- [x] 6.6 Manual test: `parse-figma-canvas all` — verify error about missing `-o`
