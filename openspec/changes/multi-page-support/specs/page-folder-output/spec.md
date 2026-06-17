## ADDED Requirements

### Requirement: Create per-page subfolders

The system SHALL create a separate subfolder for each page in the output directory.

#### Scenario: Two pages in document
- **WHEN** a Figma document contains 2 pages
- **THEN** the system creates 2 subfolders in the output directory, one for each page

#### Scenario: Output directory structure
- **WHEN** the output directory is `/output/` and pages are "Page 1" and "Page 2"
- **THEN** the directory structure is:
  ```
  /output/
  ├── page_1/
  │   ├── tree.txt
  │   ├── texts.txt
  │   └── ...
  └── page_2/
      ├── tree.txt
      ├── texts.txt
      └── ...
  ```

### Requirement: Sanitize folder names

The system SHALL sanitize page names to create valid folder names.

#### Scenario: Page name with spaces and special characters
- **WHEN** page name is "Lounge booking - make apt flow"
- **THEN** folder name is "lounge_booking_-_make_apt_flow"

#### Scenario: Page name with uppercase letters
- **WHEN** page name is "Design System"
- **THEN** folder name is "design_system"

#### Scenario: Page name with invalid characters
- **WHEN** page name is "Page 1 (Draft)"
- **THEN** folder name is "page_1_draft"

#### Scenario: Page name with multiple consecutive spaces
- **WHEN** page name is "Page  1  Draft"
- **THEN** folder name is "page_1_draft"

### Requirement: Store output files in page folders

The system SHALL store all output files (tree.txt, texts.txt, etc.) in the corresponding page subfolder.

#### Scenario: Tree command output
- **WHEN** user runs `tree` command on document with 2 pages
- **THEN** each page folder contains its own tree.txt file

#### Scenario: All command output
- **WHEN** user runs `all` command on document with 2 pages
- **THEN** each page folder contains all output files (tree.txt, texts.txt, images.txt, etc.)
