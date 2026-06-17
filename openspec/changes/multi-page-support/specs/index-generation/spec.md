## ADDED Requirements

### Requirement: Generate index.md in output root

The system SHALL generate an index.md file in the output root directory describing all pages.

#### Scenario: Document with multiple pages
- **WHEN** a Figma document contains 3 pages
- **THEN** the system creates index.md in the output root with information about all 3 pages

#### Scenario: index.md content
- **WHEN** the output directory contains pages "Page 1" and "Page 2"
- **THEN** index.md contains:
  ```markdown
  # Pages
  
  | # | Page Name | Folder | Children |
  |---|-----------|--------|----------|
  | 1 | Page 1 | page_1 | 25 |
  | 2 | Page 2 | page_2 | 10 |
  ```

### Requirement: Include page metadata in index

The system SHALL include page name, folder name, and child count in index.md.

#### Scenario: Page with children
- **WHEN** a page has 25 child elements
- **THEN** index.md shows "25" in the Children column for that page

#### Scenario: Page with no children
- **WHEN** a page has 0 child elements
- **THEN** index.md shows "0" in the Children column for that page

### Requirement: Generate index for all output modes

The system SHALL generate index.md whether using single command or all command.

#### Scenario: Single command with -o flag
- **WHEN** user runs `tree -o /output/`
- **THEN** system creates index.md in /output/ with information about processed pages

#### Scenario: All command with -o flag
- **WHEN** user runs `all -o /output/`
- **THEN** system creates index.md in /output/ with information about all pages
