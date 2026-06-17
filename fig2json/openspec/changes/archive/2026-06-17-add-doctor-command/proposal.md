## Why

When debugging issues with `.fig` file parsing (e.g., missing pages, incorrect tree structure, blob parsing failures), there is no built-in way to inspect the intermediate data at each conversion stage. Developers must add ad-hoc logging or use external tools. A `doctor` command provides a structured diagnostic output, making it easy to diagnose parsing problems without modifying code.

## What Changes

- Add a new `doctor` subcommand to the CLI (alongside the existing implicit convert mode)
- The `doctor` command takes a `.fig` file path and outputs diagnostic data to `doctor.log` in the current working directory
- Diagnostic data includes:
  - File metadata (size, magic header, version, chunk count)
  - Kiwi schema info (definition count, root message type)
  - `nodeChanges` summary (total count, node types distribution, GUID space)
  - Page-level breakdown (pages found, children per page, orphaned nodes)
  - Blob summary (total blobs, types, sizes)
  - Tree structure stats (depth, total nodes, nodes per page)
- Output format is human-readable text with clear section headers

## Capabilities

### New Capabilities
- `doctor-command`: CLI subcommand that runs diagnostic analysis on a `.fig` file and writes structured output to `doctor.log`

### Modified Capabilities
<!-- No existing capabilities are modified -->

## Impact

- **CLI**: `main.rs` will be restructured to support subcommands (currently uses flat args via `clap::Parser`)
- **New module**: `src/doctor.rs` for diagnostic logic (analysis functions)
- **Dependencies**: No new dependencies; uses existing `clap` subcommand support
- **Output**: Creates `doctor.log` file in the directory where the CLI is executed
- **Backward compatibility**: Existing convert mode must remain the default behavior when no subcommand is specified
