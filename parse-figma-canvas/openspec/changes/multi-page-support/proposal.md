## Why

The `parse-figma-canvas` tool currently only processes the **first page** of a multi-page Figma file. The `get_canvas()` function uses `a.first()` which hardcodes this behavior. This means all output (tree, texts, images, tokens, layers) only reflects the first page, missing all other pages in the document.

This is a problem because Figma files often contain multiple pages (e.g., design system components on one page, actual designs on another), and users need to analyze all pages.

## What Changes

- **BREAKING**: All commands now process **all pages** instead of just the first page
- Each page gets its own output subfolder with sanitized naming convention
- `index.md` generated in output root describing all pages and their folder names
- JSON summary output to stdout with success/error status
- `--layer` flag searches across all pages, prefixes output with page name
- `node` command outputs to the folder of the page containing the node
- Error handling outputs structured JSON for both success and failure cases

## Capabilities

### New Capabilities
- `multi-page-processing`: Process all pages in a Figma document instead of just the first
- `page-folder-output`: Organize output into per-page subfolders with sanitized naming
- `index-generation`: Generate index.md describing all pages and their folder structure
- `json-summary-output`: Structured JSON output with success/error status for both success and failure cases

### Modified Capabilities

## Impact

- **Files affected**: `src/main.rs` (major refactor of main loop and command execution)
- **Output structure**: Changes from flat files to per-page subfolder organization
- **CLI behavior**: All commands now process all pages; `--layer` searches across pages
- **JSON output**: New structured JSON output to stdout replaces plain text output
- **Backward compatibility**: Output directory structure changes (breaking)
