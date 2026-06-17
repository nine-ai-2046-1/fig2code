## Why

The CLI currently outputs everything to stdout, which makes it hard to integrate into automated pipelines or batch workflows. Users need a way to save command outputs to files for further processing. Additionally, running multiple commands individually is tedious — an `all` command would streamline batch extraction of Figma canvas data.

## What Changes

- Add global `-o <output_dir>` flag to CLI, auto-creates directory if not exists
- When `-o` is specified, command output is saved to `<output_dir>/<command>.txt` instead of stdout
- If `-o` is omitted, behavior remains identical to current (stdout)
- Add new `all` subcommand that runs every command and saves outputs to `-o` folder
- `all` command **requires** `-o` flag (errors if missing)
- `all` skips `raw` command (requires mandatory `name` argument)
- `all` continues on individual command errors, logs failures, completes remaining commands

## Capabilities

### New Capabilities

- `output-file-routing`: Route command output to files via `-o` flag with auto-directory creation
- `batch-all-command`: Run all commands in sequence, saving each output to a named file

### Modified Capabilities

(none — no existing specs)

## Impact

- **Code**: `src/main.rs` — CLI struct changes, all `cmd_xxx` functions refactored to write to `&mut dyn Write`, main() routing logic
- **Dependencies**: None new (uses existing `std::fs`, `std::io`)
- **APIs**: CLI interface changes (new `-o` flag, new `all` subcommand)
- **Backward compatibility**: Fully backward-compatible — `-o` is optional, no `-o` preserves current behavior