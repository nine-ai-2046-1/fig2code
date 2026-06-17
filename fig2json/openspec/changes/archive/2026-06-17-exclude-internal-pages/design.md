## Context

The `pages` vector is collected at line 1137 via `get_all_canvases(&data)`. Both the `all` command and other commands iterate over this vector to process each page.

## Goals / Non-Goals

**Goals:**
- Exclude "Internal Only Canvas" from processing
- Keep filter simple and maintainable

**Non-Goals:**
- Configurable exclusion patterns (overkill for this use case)

## Decisions

### D1: Filter pages by name contains "Internal Only"

**Choice**: Filter the `pages` vector immediately after collection.

```rust
let pages: Vec<&Value> = pages.into_iter()
    .filter(|p| !node_name(p).contains("Internal Only"))
    .collect();
```

**Why**: Single filter point, applies to all commands, minimal code change.

## Risks / Trade-offs

- **[Hardcoded]** → If page naming convention changes, filter needs update. Mitigation: Simple pattern, easy to modify.
