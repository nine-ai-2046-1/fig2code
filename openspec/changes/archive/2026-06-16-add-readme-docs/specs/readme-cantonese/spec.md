## ADDED Requirements

### Requirement: README-HK.md contains project background
The README-HK SHALL include a background section in Cantonese explaining what `parse-figma-canvas` does.

#### Scenario: Background present
- **WHEN** user opens README-HK.md
- **THEN** file contains 廣東話 description of the tool with emoji

### Requirement: README-HK.md contains installation instructions
The README-HK SHALL include build/install instructions in Cantonese.

#### Scenario: Installation section present
- **WHEN** user reads README-HK.md
- **THEN** file contains `cargo build` instructions written in 廣東話

### Requirement: README-HK.md contains usage examples for all commands
The README-HK SHALL include usage examples for every command: tree, node, texts, images, interactions, tokens, layers, raw, all.

#### Scenario: Each command has an example
- **WHEN** user reads README-HK.md
- **THEN** each command has at least one usage example with realistic flags and Cantonese descriptions

### Requirement: README-HK.md explains -o flag
The README-HK SHALL explain the `-o` flag in Cantonese with examples.

#### Scenario: -o flag documented
- **WHEN** user reads README-HK.md
- **THEN** file explains `-o` 嘅用法 using 廣東話
- **AND** shows example with emoji

### Requirement: README-HK.md explains all command
The README-HK SHALL explain the `all` command in Cantonese.

#### Scenario: all command documented
- **WHEN** user reads README-HK.md
- **THEN** file explains `all` 會跑晒所有 command 並存檔
- **AND** shows example with emoji
