## ADDED Requirements

### Requirement: Output JSON summary to stdout

The system SHALL output a structured JSON summary to stdout after processing.

#### Scenario: Successful processing
- **WHEN** the system successfully processes a document with 2 pages
- **THEN** stdout contains:
  ```json
  {
    "success": true,
    "page_count": 2,
    "pages": [
      {"name": "Page 1", "folder": "page_1"},
      {"name": "Page 2", "folder": "page_2"}
    ]
  }
  ```

### Requirement: Include success field

The system SHALL include a `success` field in the JSON output indicating whether the operation succeeded.

#### Scenario: Successful operation
- **WHEN** the operation completes without errors
- **THEN** JSON output has `"success": true`

#### Scenario: Failed operation
- **WHEN** the operation fails (file not found, parse error, etc.)
- **THEN** JSON output has `"success": false`

### Requirement: Include error message for failures

The system SHALL include a `msg` field in the JSON output for failed operations.

#### Scenario: File not found
- **WHEN** the input file does not exist
- **THEN** JSON output has `"success": false` and `"msg": "Error reading input file: No such file or directory"`

#### Scenario: JSON parse error
- **WHEN** the input file is not valid JSON
- **THEN** JSON output has `"success": false` and `"msg": "Error parsing JSON: ..."`

#### Scenario: Layer not found
- **WHEN** user specifies a layer that doesn't exist in any page
- **THEN** JSON output has `"success": false` and `"msg": "Layer 'xyz' not found in any page"`

#### Scenario: Node not found
- **WHEN** user specifies a node that doesn't exist in any page
- **THEN** JSON output has `"success": false` and `"msg": "Node 'xyz' not found in any page"`

### Requirement: Error messages to stderr

The system SHALL output error messages to stderr, not stdout.

#### Scenario: Error during processing
- **WHEN** an error occurs during processing
- **THEN** error details are written to stderr, and JSON summary (with success: false) is written to stdout

#### Scenario: Progress messages
- **WHEN** the system is processing pages
- **THEN** progress messages are written to stderr, not stdout
