## ADDED Requirements

### Requirement: All pages are exported in convert output

The `convert` and `convert_raw` functions SHALL include all pages from the source `.fig` file in the output JSON, not just the first page.

#### Scenario: Two-page file exports both pages
- **WHEN** a `.fig` file contains 2 pages (both with `parentIndex` pointing to root `"0:0"`)
- **THEN** the output JSON root has a `children` array containing both page nodes

#### Scenario: Single-page file exports correctly
- **WHEN** a `.fig` file contains 1 page
- **THEN** the output JSON root has a `children` array containing exactly 1 page node (no regression)

### Requirement: build_tree provides diagnostic output

The `build_tree` function SHALL emit diagnostic information to stderr when the `FIG2JSON_DEBUG` environment variable is set to `1`.

#### Scenario: Debug mode enabled
- **WHEN** `FIG2JSON_DEBUG=1` is set and `build_tree` is called
- **THEN** stderr receives: total node count, root existence check, root children count, orphan count

#### Scenario: Debug mode disabled
- **WHEN** `FIG2JSON_DEBUG` is not set
- **THEN** `build_tree` produces no stderr output

### Requirement: Orphan nodes are detected

The `build_tree` function SHALL detect nodes that have no `parentIndex` field and are not the root `"0:0"`, reporting them as orphans in debug output.

#### Scenario: Node with no parentIndex
- **WHEN** a node in `nodeChanges` has no `parentIndex` field and its GUID is not `"0:0"`
- **THEN** the node is logged as an orphan in debug output and excluded from the tree
