# parse-figma-canvas

CLI tool to extract structured data from Figma `canvas.raw.json` for AI agents.

Use this instead of reading `canvas.raw.json` directly — the file is often **~44MB** and hard to parse manually. This tool gives you targeted, human-readable output for each query.

## Installation

```bash
# Build from source
cargo build --release

# The binary will be at target/release/parse-figma-canvas
# Optionally, install it globally:
cargo install --path .
```

## Usage

```
parse-figma-canvas [OPTIONS] <COMMAND>
```

### Global Options

| Flag | Description |
|------|-------------|
| `-i, --input <FILE>` | Path to `canvas.raw.json` (default: `canvas.raw.json`) |
| `-o, --output <DIR>` | Save output to files instead of stdout |

## Commands

### tree — View node hierarchy

Print the full node tree with name, type, size, and position.

```bash
# Print entire tree
parse-figma-canvas tree

# Limit depth to 3 levels
parse-figma-canvas tree -d 3

# Show only a specific layer's children
parse-figma-canvas tree -l "Header"
```

### node — Inspect a specific node

Dump all properties of a node by exact name.

```bash
# Inspect a node by name
parse-figma-canvas node "Submit Button"

# Search within a specific layer
parse-figma-canvas node "Submit Button" -l "Form"
```

### texts — List all text nodes

List every text node with font, size, colour, and content.

```bash
# List all text nodes
parse-figma-canvas texts

# Only text nodes in a layer
parse-figma-canvas texts -l "Navigation"
```

### images — List all image fills

List all image fills with resolved hash-to-filename mapping.

```bash
# List all images
parse-figma-canvas images

# Verify images exist on disk
parse-figma-canvas images -d ./images/

# Only images in a layer
parse-figma-canvas images -l "Hero"
```

### interactions — List prototype interactions

List all prototype interactions with resolved GUID-to-node names.

```bash
parse-figma-canvas interactions

# Only interactions in a layer
parse-figma-canvas interactions -l "Onboarding"
```

### tokens — Extract design tokens

Extract all design tokens: colours, fonts, spacing, radii, effects.

```bash
parse-figma-canvas tokens

# Tokens from a specific layer
parse-figma-canvas tokens -l "Brand"
```

### layers — List top-level frames

List all layers (top-level frames) on the canvas.

```bash
parse-figma-canvas layers
```

### raw — Debug raw JSON

Dump a section of a node's raw JSON for debugging.

```bash
# Dump full node JSON
parse-figma-canvas raw "Header"

# Dump specific property
parse-figma-canvas raw "Header" -p /fillPaints
```

## Output to Files (`-o`)

Use the `-o` flag to save command output to files instead of stdout. The directory is created automatically if it doesn't exist.

```bash
# Save tree output to a file
parse-figma-canvas -o ./output tree
# → creates ./output/tree.txt

# Save multiple commands
parse-figma-canvas -o ./output texts
parse-figma-canvas -o ./output images
parse-figma-canvas -o ./output layers
```

## Run All Commands (`all`)

The `all` command runs every command (tree, texts, images, interactions, tokens, layers) and saves each output to a file. It **requires** the `-o` flag.

```bash
# Run all commands, save to output directory
parse-figma-canvas -o ./output all

# Creates:
#   ./output/tree.txt
#   ./output/texts.txt
#   ./output/images.txt
#   ./output/interactions.txt
#   ./output/tokens.txt
#   ./output/layers.txt
```

> Note: The `raw` command is excluded from `all` because it requires a node name argument.
