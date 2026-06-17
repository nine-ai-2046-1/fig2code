## ADDED Requirements

### Requirement: Global output directory flag
The CLI SHALL accept a global `-o <output_dir>` option that specifies where to save command output.

#### Scenario: Flag provided with valid path
- **WHEN** user runs `parse-figma-canvas -o ./output tree`
- **THEN** the output directory is created if it doesn't exist
- **AND** the command output is saved to `./output/tree.txt`

#### Scenario: Flag provided with existing directory
- **WHEN** user runs `parse-figma-canvas -o ./existing-dir texts`
- **AND** `./existing-dir` already exists
- **THEN** the command output is saved to `./existing-dir/texts.txt`
- **AND** no error is raised about the directory existing

#### Scenario: Flag omitted
- **WHEN** user runs `parse-figma-canvas tree`
- **AND** no `-o` flag is provided
- **THEN** output is written to stdout (current behavior)

### Requirement: Auto-create output directory
The CLI SHALL automatically create the output directory and any parent directories when `-o` is specified.

#### Scenario: Directory does not exist
- **WHEN** user runs `parse-figma-canvas -o /tmp/new/deep/path layers`
- **THEN** the directory `/tmp/new/deep/path` is created recursively
- **AND** the output is saved to `/tmp/new/deep/path/layers.txt`

### Requirement: Output file naming
The CLI SHALL save command output to files named `<command_name>.txt` within the output directory.

#### Scenario: Tree command output
- **WHEN** user runs `parse-figma-canvas -o ./out tree`
- **THEN** output is saved to `./out/tree.txt`

#### Scenario: Texts command output
- **WHEN** user runs `parse-figma-canvas -o ./out texts`
- **THEN** output is saved to `./out/texts.txt`

#### Scenario: Node command output
- **WHEN** user runs `parse-figma-canvas -o ./out node "Button"`
- **THEN** output is saved to `./out/node.txt`
