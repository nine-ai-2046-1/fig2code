## ADDED Requirements

### Requirement: Process all pages in document

The system SHALL process all pages in a Figma document, not just the first page.

#### Scenario: Document with multiple pages
- **WHEN** a Figma document contains 3 pages
- **THEN** the system processes all 3 pages and generates output for each

#### Scenario: Document with single page
- **WHEN** a Figma document contains 1 page
- **THEN** the system processes that page normally (no regression)

### Requirement: --layer searches across all pages

The system SHALL search for the specified layer across all pages when --layer flag is used.

#### Scenario: Layer found in second page
- **WHEN** user specifies `--layer "Design System"` and the layer exists in page 2
- **THEN** the system finds and processes that layer from page 2

#### Scenario: Layer not found in any page
- **WHEN** user specifies `--layer "Nonexistent"` and the layer doesn't exist in any page
- **THEN** the system outputs error JSON with message indicating layer not found

### Requirement: node command searches across all pages

The system SHALL search for the specified node across all pages when node command is used.

#### Scenario: Node found in third page
- **WHEN** user specifies `node "Button"` and the node exists in page 3
- **THEN** the system finds and processes that node from page 3

#### Scenario: Node not found in any page
- **WHEN** user specifies `node "Nonexistent"` and the node doesn't exist in any page
- **THEN** the system outputs error JSON with message indicating node not found
