## Context

The `parse-figma-canvas` tool is a CLI utility that extracts structured data from Figma's `canvas.raw.json` files. Currently, it only processes the first page due to the `get_canvas()` function using `a.first()`. The tool generates text files (tree.txt, texts.txt, images.txt, etc.) for analysis by AI agents.

The tool is used in the `fig2code` project alongside `fig2json` to analyze Figma design files. The `fig2json` tool correctly outputs all pages, but `parse-figma-canvas` only processes the first one.

## Goals / Non-Goals

**Goals:**
- Process all pages in a Figma document, not just the first
- Organize output into per-page subfolders with sanitized naming
- Generate index.md describing all pages and their folder structure
- Provide structured JSON output with success/error status
- Maintain all existing command functionality (tree, texts, images, etc.)

**Non-Goals:**
- Changing the underlying Figma JSON parsing logic
- Modifying the output format of individual commands (tree, texts, etc.)
- Supporting streaming or real-time processing
- Changing the CLI argument structure (only adding multi-page behavior)

## Decisions

### 1. Output Structure: Per-page subfolders

**Decision**: Create a subfolder for each page in the output directory, named with sanitized page name.

**Rationale**: 
- Keeps output organized when dealing with multiple pages
- Prevents filename conflicts between pages
- Easy to navigate and understand
- Follows the pattern of other design tools that organize by page

**Alternative considered**: Append page name to filenames (e.g., `tree_page1.txt`). Rejected because it becomes unwieldy with many pages and doesn't scale well.

### 2. Folder Naming: Sanitize page names

**Decision**: Convert page names to lowercase, replace spaces with underscores, remove special characters, only allow a-z, 0-9, _, -.

**Rationale**:
- Ensures valid directory names across all operating systems
- Makes folder names predictable and machine-readable
- Prevents issues with special characters in paths
- Follows common conventions for file/directory naming

**Example**:
```
Input:  "Lounge booking - make apt flow"
Output: "lounge_booking_-_make_apt_flow"
```

### 3. JSON Output: Structured success/error

**Decision**: Output JSON to stdout with `success` field (true/false), and `msg` field for errors.

**Rationale**:
- Machine-readable output for programmatic consumption
- Consistent error handling across all commands
- Easy to parse and integrate with other tools
- Follows common API response patterns

**Format**:
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

### 4. --layer Flag: Search across all pages

**Decision**: When `--layer` is specified, search across all pages and prefix output with page name.

**Rationale**:
- Users may not know which page contains a specific layer
- Provides complete search across the entire document
- Prefixing with page name provides context for where the layer was found
- Maintains backward compatibility for single-page documents

### 5. node Command: Output to page folder

**Decision**: When `node` command finds a node, output to the folder of the page containing that node.

**Rationale**:
- Maintains organization by page
- Prevents confusion about which page the node belongs to
- Consistent with other commands' behavior

## Risks / Trade-offs

- **Breaking change**: Output directory structure changes from flat files to per-page subfolders. Users will need to update their workflows.
- **Performance**: Processing all pages may be slower for large documents. Mitigation: Most documents have few pages (<10), so impact is minimal.
- **Naming conflicts**: Multiple pages could sanitize to the same folder name. Mitigation: Add numeric suffix if conflicts occur.
- **Error handling complexity**: More code paths to handle errors. Mitigation: Centralize error handling in helper functions.
