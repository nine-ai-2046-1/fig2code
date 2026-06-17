## Context

The `parse-figma-canvas` CLI currently writes all output to stdout. This works for interactive use but limits integration into automated pipelines, batch scripts, and file-based workflows. Users need to capture command outputs as files for downstream processing.

The codebase is a single-file Rust CLI (`src/main.rs`, ~967 lines) using `clap` for argument parsing. All `cmd_xxx` functions use `println!` for output.

## Goals / Non-Goals

**Goals:**
- Add `-o <output_dir>` global flag that routes command output to files
- Auto-create output directory if it doesn't exist
- Preserve 100% backward compatibility when `-o` is omitted
- Add `all` subcommand that runs all commands sequentially, saving each to a file
- Graceful error handling in `all` mode (continue on failure)

**Non-Goals:**
- Changing output format or content of existing commands
- Adding parallel execution in `all` mode
- Supporting individual file name overrides per command
- Supporting `raw` command in `all` mode (requires mandatory `name` argument)

## Decisions

### D1: Refactor cmd functions to accept `&mut dyn Write`

**Choice**: Change all `cmd_xxx` signatures from implicit stdout to explicit `out: &mut dyn Write` parameter.

**Why**: This is the most idiomatic Rust approach. Alternatives considered:
- `std::io::set_output_capture` — nightly-only, not stable
- `gag` crate — external dependency for a simple use case
- Process-level stdout redirect — complex, error-prone

**Trade-off**: Requires changing every `println!` to `writeln!(out, ...)` across ~200 lines, but results in clean, testable code.

### D2: Output file naming convention

**Choice**: Use `<command>.txt` as filename (e.g., `tree.txt`, `texts.txt`).

**Why**: Simple, predictable, matches command names. No ambiguity.

### D3: `all` command skips `raw`

**Choice**: The `raw` command requires a mandatory `name` argument, making it unsuitable for batch mode.

**Why**: No sensible default exists for "dump all raw nodes" — it would produce overwhelming output. Users can run `raw` individually with `-o`.

### D4: Error handling in `all` mode

**Choice**: Log errors to stderr, continue with remaining commands.

**Why**: Partial results are more useful than no results. Users can see which commands failed and retry individually.

### D5: `-o` required for `all`

**Choice**: The `all` command errors immediately if `-o` is not provided.

**Why**: Running all commands to stdout would produce interleaved, unusable output. File output is the whole point of `all`.

## Risks / Trade-offs

- **[Large diff]** → Refactoring ~200 `println!` calls is mechanical but creates a large changeset. Mitigation: Use `sed` or editor macros for bulk replacement, then manual review.
- **[Error handling in `all`]** → Some commands may fail silently if errors are only logged to stderr. Mitigation: Print clear "SKIPPED" or "FAILED" markers in output summary.
- **[File naming collisions]** → If two commands produce the same filename, the second overwrites the first. Mitigation: Use unique names per command (already guaranteed by design).
