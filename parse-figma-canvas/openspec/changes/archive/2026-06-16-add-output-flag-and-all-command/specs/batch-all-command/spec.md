## ADDED Requirements

### Requirement: All command runs all other commands
The CLI SHALL provide an `all` subcommand that runs every supported command sequentially.

#### Scenario: All command with valid output directory
- **WHEN** user runs `parse-figma-canvas -o ./output all`
- **THEN** the following commands are executed in order: `tree`, `texts`, `images`, `interactions`, `tokens`, `layers`
- **AND** each command's output is saved to `./output/<command>.txt`

#### Scenario: All command output files
- **WHEN** `all` command completes successfully
- **THEN** the output directory contains: `tree.txt`, `texts.txt`, `images.txt`, `interactions.txt`, `tokens.txt`, `layers.txt`

### Requirement: All command requires output directory
The `all` command SHALL require the `-o` flag to be specified.

#### Scenario: All command without -o flag
- **WHEN** user runs `parse-figma-canvas all`
- **AND** no `-o` flag is provided
- **THEN** the CLI prints an error message to stderr
- **AND** exits with non-zero status code

### Requirement: All command skips raw command
The `all` command SHALL NOT execute the `raw` command.

#### Scenario: Raw is excluded from all
- **WHEN** `all` command runs
- **THEN** the `raw` command is not executed
- **AND** no `raw.txt` file is created in the output directory

### Requirement: All command error handling
The `all` command SHALL continue executing remaining commands if an individual command fails.

#### Scenario: One command fails
- **WHEN** `all` command runs
- **AND** one command (e.g., `images`) fails with an error
- **THEN** the error is printed to stderr
- **AND** the remaining commands continue executing
- **AND** successfully completed commands have their output saved

#### Scenario: Multiple commands fail
- **WHEN** `all` command runs
- **AND** multiple commands fail
- **THEN** all errors are printed to stderr
- **AND** all successfully completed commands have their output saved

### Requirement: All command skips commands needing mandatory args
The `all` command SHALL skip commands that require mandatory arguments not provided in batch mode.

#### Scenario: Raw command skipped
- **WHEN** `all` command runs
- **THEN** `raw` is skipped (requires mandatory `name` argument)
- **AND** no error is raised for skipping
