## Context

The fig2json CLI currently has a single implicit mode: convert a `.fig` file to JSON. There is no diagnostic or inspection capability. When users encounter issues (missing pages, incorrect tree structure, blob failures), they must add ad-hoc logging or use external tools to debug.

The CLI uses `clap` with a flat argument structure (`struct Cli` with `#[derive(Parser)]`). Adding a `doctor` command requires restructuring to support subcommands while preserving backward compatibility.

## Goals / Non-Goals

**Goals:**
- Add a `doctor` subcommand that outputs diagnostic data to `doctor.log`
- Restructure CLI to support subcommands via `clap`'s `#[command(subcommand)]` pattern
- Keep the existing convert mode as the default behavior (no subcommand = convert)
- Diagnostic output covers: file metadata, Kiwi schema, nodeChanges, pages, blobs, tree stats

**Non-Goals:**
- Real-time streaming output (doctor runs once, writes file)
- JSON-formatted doctor output (human-readable text only for now)
- Doctor analyzing ZIP archives (only single `.fig` files)
- Modifying the existing convert pipeline (read-only analysis)

## Decisions

### 1. CLI Structure: Clap Subcommands

**Decision**: Use `clap`'s `#[derive(Subcommand)]` enum with `Convert` (default) and `Doctor` variants.

**Why**: Clap natively supports subcommands with `#[command(subcommand)]`. This is the idiomatic approach and preserves backward compatibility via `#[command(default)]` or by making convert the implicit mode.

**Alternative considered**: Keep flat args and add `--doctor` flag. Rejected because it conflates two distinct modes and makes future subcommands harder.

### 2. Doctor Module: Separate `src/doctor.rs`

**Decision**: Create a new `src/doctor.rs` module for all diagnostic logic.

**Why**: Keeps diagnostic concerns isolated from conversion logic. The `convert` and `doctor` paths share the same parsing pipeline (chunks, decompression, Kiwi decode) but diverge after that. A separate module makes this clear.

### 3. Output Format: Plain Text with Sections

**Decision**: Write human-readable text with section headers (e.g., `=== File Metadata ===`) to `doctor.log`.

**Why**: Plain text is easy to read in any editor, grep for specific values, and doesn't require additional dependencies. Sections make it scannable.

**Alternative considered**: JSON output. Rejected for now because doctor output is for human debugging, not programmatic consumption.

### 4. Diagnostic Scope: Post-Decode Analysis

**Decision**: Run the full parsing pipeline (extract chunks → decompress → Kiwi decode → build tree) and capture intermediate results at each stage.

**Why**: The most common issues happen at specific stages. Capturing data at each stage lets users pinpoint where things go wrong without re-running with custom logging.

**Trade-off**: This means doctor re-runs the full parse. For very large files this could be slow, but it's acceptable for a diagnostic tool.

### 5. File Location: Current Working Directory

**Decision**: Write `doctor.log` to the directory where the CLI is executed (not alongside the `.fig` file).

**Why**: Matches user expectation - the log appears where they ran the command. Avoids permission issues with read-only directories.

## Risks / Trade-offs

- **[Risk]** Large `.fig` files may produce very large `doctor.log` → **Mitigation**: Cap output per section (e.g., max 100 nodes printed in detail, summarize the rest)
- **[Risk]** Restructuring CLI may break existing invocations → **Mitigation**: Use clap's `#[command(flatten)]` or default subcommand pattern; test existing usage
- **[Risk]** Doctor output format may not suit all debugging needs → **Mitigation**: Start with text; can add `--json` flag later if needed
