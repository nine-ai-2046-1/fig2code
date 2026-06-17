## ADDED Requirements

### Requirement: Internal pages are excluded from processing
The system SHALL filter out pages whose name contains "Internal Only" before processing.

#### Scenario: Internal page excluded
- **WHEN** document contains a page named "Internal Only Canvas"
- **THEN** that page SHALL NOT be included in the output for any command

#### Scenario: Normal page included
- **WHEN** document contains a page named "demo-1"
- **THEN** that page SHALL be included in the output
