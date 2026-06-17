## ADDED Requirements

### Requirement: Doctor command handles ZIP-wrapped .fig files

The `doctor` command SHALL correctly analyze `.fig` files that are ZIP containers by extracting `canvas.fig` from the ZIP archive before running diagnostic analysis.

#### Scenario: Doctor analyzes a ZIP-wrapped .fig file
- **WHEN** the user runs `fig2json doctor input.fig` where `input.fig` is a ZIP container
- **THEN** the doctor command extracts `canvas.fig` from the ZIP and produces diagnostic output showing node counts, page counts, and blob counts matching the actual file contents

#### Scenario: Doctor analyzes a raw .fig file
- **WHEN** the user runs `fig2json doctor input.fig` where `input.fig` is a raw .fig binary
- **THEN** the doctor command produces diagnostic output as before (no regression)

#### Scenario: ZIP contains no canvas.fig
- **WHEN** the user runs `fig2json doctor input.fig` where the ZIP has no `canvas.fig` entry
- **THEN** the doctor command exits with an error message indicating `canvas.fig` was not found

### Requirement: Diagnostic output includes ZIP metadata

When analyzing a ZIP-wrapped file, the doctor output SHALL include a "Container" section showing the ZIP file size and extracted `canvas.fig` size.

#### Scenario: ZIP container info displayed
- **WHEN** the doctor analyzes a ZIP-wrapped .fig file
- **THEN** the output includes "Container type: ZIP" and the extracted canvas.fig byte count
