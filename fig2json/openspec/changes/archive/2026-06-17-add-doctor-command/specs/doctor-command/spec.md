## ADDED Requirements

### Requirement: Doctor command accepts a .fig file path
The system SHALL accept a `.fig` file path as the sole argument when the `doctor` subcommand is used.

#### Scenario: Valid .fig file provided
- **WHEN** user runs `fig2json doctor path/to/file.fig`
- **THEN** the system reads the file at the given path

#### Scenario: Missing file argument
- **WHEN** user runs `fig2json doctor` without a file path
- **THEN** the system displays an error message indicating a file path is required

#### Scenario: Non-existent file
- **WHEN** user runs `fig2json doctor path/to/missing.fig`
- **THEN** the system displays an error message indicating the file was not found

### Requirement: Doctor command writes output to doctor.log
The system SHALL write all diagnostic output to a file named `doctor.log` in the current working directory (the directory from which the CLI is executed).

#### Scenario: Successful diagnostic run
- **WHEN** the doctor command completes analysis of a valid `.fig` file
- **THEN** a `doctor.log` file is created in the current working directory containing the diagnostic output

#### Scenario: Overwrite existing doctor.log
- **WHEN** a `doctor.log` file already exists in the current working directory
- **THEN** the system overwrites it with the new diagnostic output

### Requirement: Doctor output includes file metadata
The system SHALL include file metadata in the diagnostic output.

#### Scenario: File metadata section
- **WHEN** the doctor command analyzes a `.fig` file
- **THEN** the output contains a section titled `=== File Metadata ===` with:
  - File size in bytes
  - Magic header (e.g., `fig-kiwi` or `fig-jam.`)
  - File format version number
  - Number of chunks detected

### Requirement: Doctor output includes Kiwi schema info
The system SHALL include Kiwi schema information in the diagnostic output.

#### Scenario: Schema info section
- **WHEN** the doctor command successfully decodes the Kiwi schema
- **THEN** the output contains a section titled `=== Kiwi Schema ===` with:
  - Total definition count
  - Root message type name (e.g., `Message`)
  - Names of all definition types

### Requirement: Doctor output includes nodeChanges summary
The system SHALL include a summary of the decoded `nodeChanges` array.

#### Scenario: NodeChanges summary section
- **WHEN** the doctor command successfully decodes nodeChanges
- **THEN** the output contains a section titled `=== NodeChanges ===` with:
  - Total node count
  - Distribution of node types (count per type, e.g., `FRAME: 15`, `TEXT: 42`)

### Requirement: Doctor output includes page-level breakdown
The system SHALL include a breakdown of pages and their children.

#### Scenario: Page breakdown section
- **WHEN** the doctor command builds the document tree
- **THEN** the output contains a section titled `=== Pages ===` with:
  - Number of pages found
  - For each page: name, GUID, and direct child count
  - Count of orphaned nodes (nodes not reachable from root `"0:0"`)

### Requirement: Doctor output includes blob summary
The system SHALL include a summary of the blobs array.

#### Scenario: Blob summary section
- **WHEN** the doctor command processes the blobs array
- **THEN** the output contains a section titled `=== Blobs ===` with:
  - Total blob count
  - Distribution of blob types (if type field exists)

### Requirement: Doctor output includes tree structure stats
The system SHALL include statistics about the built document tree.

#### Scenario: Tree stats section
- **WHEN** the doctor command builds the document tree
- **THEN** the output contains a section titled `=== Tree Stats ===` with:
  - Maximum tree depth
  - Total nodes in the tree (reachable from root)
  - Nodes per page (for each page, count of descendants)

### Requirement: Doctor uses verbose flag for detailed output
The system SHALL support the `-v` / `--verbose` flag to show additional detail in the doctor output.

#### Scenario: Verbose mode enabled
- **WHEN** user runs `fig2json doctor path/to/file.fig -v`
- **THEN** the output includes additional details such as:
  - First 10 node GUIDs in the nodeChanges array
  - First 5 blob entries in detail

#### Scenario: Verbose mode not enabled (default)
- **WHEN** user runs `fig2json doctor path/to/file.fig` without `-v`
- **THEN** the output shows only summary statistics without detailed listings
