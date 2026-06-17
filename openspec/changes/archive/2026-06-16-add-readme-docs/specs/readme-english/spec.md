## ADDED Requirements

### Requirement: README.md contains project background
The README SHALL include a background section explaining what `parse-figma-canvas` does and why it exists.

#### Scenario: Background present
- **WHEN** user opens README.md
- **THEN** file contains a section describing the tool as an AI agent tool for querying Figma canvas.raw.json

### Requirement: README.md contains installation instructions
The README SHALL include instructions for building and installing the tool.

#### Scenario: Installation section present
- **WHEN** user reads README.md
- **THEN** file contains `cargo build` or `cargo install` instructions

### Requirement: README.md contains usage examples for all commands
The README SHALL include usage examples for every command: tree, node, texts, images, interactions, tokens, layers, raw, all.

#### Scenario: Each command has an example
- **WHEN** user reads README.md
- **THEN** each command has at least one usage example showing the command with realistic flags

### Requirement: README.md explains -o flag
The README SHALL explain the `-o` output directory flag with examples.

#### Scenario: -o flag documented
- **WHEN** user reads README.md
- **THEN** file explains `-o` saves output to files instead of stdout
- **AND** shows example: `parse-figma-canvas -o ./output tree`

### Requirement: README.md explains all command
The README SHALL explain the `all` command and its requirement for `-o`.

#### Scenario: all command documented
- **WHEN** user reads README.md
- **THEN** file explains `all` runs every command and saves outputs
- **AND** shows example: `parse-figma-canvas -o ./output all`
