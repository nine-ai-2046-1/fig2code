# parse-figma-canvas 🎨

CLI 工具，幫你從 Figma 嘅 `canvas.raw.json` 搵有用嘅資料，專為 AI agent 設計 🤖

唔好直接開 `canvas.raw.json` 嚟睇 — 個檔通常有 **~44MB**，根本睇唔晒。呢個工具會幫你 query 搵返啲 human-readable 嘅 output。

## 安裝 🔧

```bash
# 由 source build
cargo build --release

# binary 會喺 target/release/parse-figma-canvas
# 如果想全域安裝：
cargo install --path .
```

## 用法 🚀

```
parse-figma-canvas [OPTIONS] <COMMAND>
```

### 全域 Options

| Flag | 說明 |
|------|------|
| `-i, --input <FILE>` | `canvas.raw.json` 嘅路徑（預設：`canvas.raw.json`） |
| `-o, --output <DIR>` | 將 output 存去檔案，而唔係 stdout |

## Commands 📋

### tree — 睇 node 層級結構 🌳

印出成個 node tree，包括 name、type、size、position。

```bash
# 印出成個 tree
parse-figma-canvas tree

# 限制深度去 3 層
parse-figma-canvas tree -d 3

# 只睇某個 layer 嘅 children
parse-figma-canvas tree -l "Header"
```

### node — 檢查某個 node 🔍

Dump 某個 node 嘅所有 properties（用 exact name）。

```bash
# 用 name 檢查 node
parse-figma-canvas node "Submit Button"

# 喺某個 layer 入面 search
parse-figma-canvas node "Submit Button" -l "Form"
```

### texts — 列出所有文字 nodes ✏️

列出所有 text nodes，包括 font、size、colour、content。

```bash
# 列出所有文字
parse-figma-canvas texts

# 只睇某個 layer 嘅文字
parse-figma-canvas texts -l "Navigation"
```

### images — 列出所有圖片 fills 🖼️

列出所有 image fills，解決 hash → filename 嘅對應。

```bash
# 列出所有圖片
parse-figma-canvas images

# 檢查圖片有冇喺 disk 上面
parse-figma-canvas images -d ./images/

# 只睇某個 layer 嘅圖片
parse-figma-canvas images -l "Hero"
```

### interactions — 列出 prototype interactions 🔗

列出所有 prototype interactions，解決 GUID → node name。

```bash
parse-figma-canvas interactions

# 只睇某個 layer
parse-figma-canvas interactions -l "Onboarding"
```

### tokens — 搵 design tokens 💎

攞晒所有 design tokens：colours、fonts、spacing、radii、effects。

```bash
parse-figma-canvas tokens

# 某個 layer 嘅 tokens
parse-figma-canvas tokens -l "Brand"
```

### layers — 列出最頂層 frames 📐

列出 canvas 上面所有最頂層嘅 frames。

```bash
parse-figma-canvas layers
```

### raw — Debug raw JSON 🐛

Dump 某個 node 嘅 raw JSON（debug 用）。

```bash
# Dump 成個 node 嘅 JSON
parse-figma-canvas raw "Header"

# Dump 某個 property
parse-figma-canvas raw "Header" -p /fillPaints
```

## 存去檔案 (`-o`) 💾

用 `-o` flag 可以將 output 存去檔案，而唔係印去 stdout。個 directory 會自動建立。

```bash
# 將 tree output 存去檔案
parse-figma-canvas -o ./output tree
# → 會建立 ./output/tree.txt

# 存多個 commands
parse-figma-canvas -o ./output texts
parse-figma-canvas -o ./output images
parse-figma-canvas -o ./output layers
```

## 跑晒所有 Commands (`all`) 🏃‍♂️

`all` command 會跑晒所有 commands（tree、texts、images、interactions、tokens、layers），然後將每個 output 存去檔案。**一定要用 `-o`** flag。

```bash
# 跑所有 commands，存去 output 目錄
parse-figma-canvas -o ./output all

# 會建立：
#   ./output/tree.txt
#   ./output/texts.txt
#   ./output/images.txt
#   ./output/interactions.txt
#   ./output/tokens.txt
#   ./output/layers.txt
```

> ⚠️ `raw` command 唔會喺 `all` 入面跑，因為佢需要指定 node name。
