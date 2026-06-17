## Context

The `parse-figma-canvas` CLI tool currently has no documentation. Users must read source code or guess usage. The project serves a bilingual (English + Cantonese) dev community in Hong Kong.

## Goals / Non-Goals

**Goals:**
- Provide comprehensive English README with background, installation, and usage examples for every command
- Provide Cantonese README-HK covering same content with natural 廣東話 + emoji
- Include clear usage samples for `-o` flag and `all` command
- Keep docs in sync with actual CLI behavior

**Non-Goals:**
- API documentation (CLI tool, no public API)
- Translations to other languages
- Auto-generated docs from source

## Decisions

### D1: Two separate files, not language switcher

**Choice**: `README.md` (English) and `README-HK.md` (Cantonese) as flat files.

**Why**: Simple, no build tooling needed, works on GitHub natively. A language switcher would require a docs site or custom tooling — overkill for two files.

### D2: Usage examples use real commands

**Choice**: Show actual `cargo run` and binary invocation commands with realistic flags.

**Why**: Users need copy-pasteable examples. Abstract placeholders like `<input>` are less helpful.

### D3: Cantonese uses natural tone, not formal written Chinese

**Choice**: README-HK.md uses 廣東話口語 + emoji, not 書面語.

**Why**: Matches the local dev culture. Formal Chinese would feel stiff for a dev tool README.

## Risks / Trade-offs

- **[Docs drift]** → README may become outdated as CLI changes. Mitigation: Keep examples minimal and focused on stable commands.
- **[Cantonese readability]** → Some readers may not understand 廣東話. Mitigation: English README is the primary doc; HK version is supplementary.
