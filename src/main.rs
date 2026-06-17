use clap::{Parser, Subcommand};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// parse-figma-canvas — AI agent tool for querying Figma canvas.raw.json
/// Use this instead of reading canvas.raw.json directly (file is ~44MB).
#[derive(Parser)]
#[command(name = "parse-figma-canvas")]
#[command(about = "Extract structured data from Figma canvas.raw.json for AI agents")]
#[command(long_about = None)]
struct Cli {
    /// Path to canvas.raw.json
    #[arg(short, long, default_value = "canvas.raw.json")]
    input: PathBuf,

    /// Output directory — save command output to files instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print the full node tree (name, type, size, position) — use this first
    Tree {
        /// Maximum depth to traverse (default: unlimited)
        #[arg(short, long)]
        depth: Option<usize>,
        /// Only show children of this named layer (exact match)
        #[arg(short, long)]
        layer: Option<String>,
    },
    /// Dump all properties of a node by exact name
    Node {
        /// Exact node name to inspect
        name: String,
        /// Which layer to search within (optional, searches all if omitted)
        #[arg(short, long)]
        layer: Option<String>,
    },
    /// List all text nodes with font, size, colour, content
    Texts {
        /// Only within this layer
        #[arg(short, long)]
        layer: Option<String>,
    },
    /// List all image fills with resolved hash→filename mapping
    Images {
        /// Only within this layer
        #[arg(short, long)]
        layer: Option<String>,
        /// Path to images/ directory to verify filenames exist
        #[arg(short = 'd', long)]
        images_dir: Option<PathBuf>,
    },
    /// List all prototype interactions with resolved GUID→node names
    Interactions {
        /// Only within this layer
        #[arg(short, long)]
        layer: Option<String>,
    },
    /// Extract all design tokens: colours, fonts, spacing, radii, effects
    Tokens {
        /// Only within this layer
        #[arg(short, long)]
        layer: Option<String>,
    },
    /// List all layers (top-level frames) on the canvas
    Layers,
    /// Dump a section of a node's raw JSON (for debugging)
    Raw {
        /// Exact node name
        name: String,
        /// JSON pointer path within the node (e.g. "/fillPaints")
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Run all commands and save outputs to -o directory
    All,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

#[allow(dead_code)]
fn get_str<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("")
}

fn get_f64(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(|x| x.as_f64()).unwrap_or(0.0)
}

fn node_type(v: &Value) -> String {
    if let Some(t) = v.get("type") {
        if let Some(obj) = t.as_object() {
            if let Some(e) = obj.get("__enum__") {
                if let Some(val) = e.get("value").and_then(|x| x.as_str()) {
                    return val.to_string();
                }
            }
            if let Some(val) = obj.get("value").and_then(|x| x.as_str()) {
                return val.to_string();
            }
        }
        if let Some(s) = t.as_str() {
            return s.to_string();
        }
    }
    String::new()
}

fn node_name(v: &Value) -> &str {
    v.get("name").and_then(|x| x.as_str()).unwrap_or("?")
}

fn node_size(v: &Value) -> (f64, f64) {
    let s = v.get("size").unwrap_or(&Value::Null);
    (get_f64(s, "x"), get_f64(s, "y"))
}

fn node_pos(v: &Value) -> (f64, f64) {
    let t = v.get("transform").unwrap_or(&Value::Null);
    (get_f64(t, "m02"), get_f64(t, "m12"))
}

fn rgba_to_hex(c: &Value) -> String {
    if c.is_null() {
        return String::from("—");
    }
    let r = (get_f64(c, "r") * 255.0).round() as u8;
    let g = (get_f64(c, "g") * 255.0).round() as u8;
    let b = (get_f64(c, "b") * 255.0).round() as u8;
    let a = get_f64(c, "a");
    if (a - 1.0).abs() < 0.005 {
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    } else {
        format!("#{:02x}{:02x}{:02x} {}%", r, g, b, (a * 100.0).round() as u8)
    }
}

fn hash_to_hex(hash: &Value) -> String {
    if let Some(arr) = hash.as_array() {
        arr.iter()
            .filter_map(|b| b.as_u64())
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    } else {
        String::new()
    }
}

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(|c| c.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
}

/// Walk tree, call f on every node. Returns early if f returns true.
fn walk<'a, F>(node: &'a Value, f: &mut F)
where
    F: FnMut(&'a Value) -> bool,
{
    if f(node) {
        return;
    }
    for child in children(node) {
        walk(child, f);
    }
}

/// Build GUID map: "sessionID:localID" -> node name
fn build_guid_map(root: &Value) -> HashMap<String, String> {
    let mut map = HashMap::new();
    walk(root, &mut |node| {
        if let Some(guid) = node.get("guid") {
            let sid = guid.get("sessionID").and_then(|x| x.as_u64()).unwrap_or(0);
            let lid = guid.get("localID").and_then(|x| x.as_u64()).unwrap_or(0);
            let key = format!("{}:{}", sid, lid);
            map.insert(key, node_name(node).to_string());
        }
        false
    });
    map
}

/// Find the document root
fn get_document(data: &Value) -> &Value {
    data.get("document").unwrap_or(data)
}

/// Find the canvas (first page)
fn get_canvas(data: &Value) -> Option<&Value> {
    get_document(data)
        .get("children")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
}

/// Find all canvases (all pages)
fn get_all_canvases(data: &Value) -> Vec<&Value> {
    get_document(data)
        .get("children")
        .and_then(|c| c.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}

/// Sanitize page name to valid folder name
/// Rules: lowercase, replace spaces with _, remove invalid chars, only a-z, 0-9, _, -
fn sanitize_folder_name(name: &str) -> String {
    let sanitized: String = name
        .to_lowercase()
        .replace(' ', "_")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    
    // Collapse multiple underscores and remove leading/trailing underscores
    sanitized
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// Get page description if available
fn get_page_description(page: &Value) -> Option<String> {
    page.get("description")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string())
}

/// Find a named layer across all pages
/// Returns (page, layer) tuple
fn find_layer_across_pages<'a>(data: &'a Value, name: &str) -> Option<(&'a Value, &'a Value)> {
    for page in get_all_canvases(data) {
        if let Some(layer) = children(page).iter().find(|c| node_name(c) == name) {
            return Some((page, layer));
        }
    }
    None
}

/// Find a node anywhere in the document by name
/// Returns (page, node) tuple
fn find_node_across_pages<'a>(data: &'a Value, name: &str) -> Option<(&'a Value, &'a Value)> {
    for page in get_all_canvases(data) {
        let mut found = None;
        walk(page, &mut |node| {
            if node_name(node) == name {
                found = Some(node);
                true
            } else {
                false
            }
        });
        if let Some(node) = found {
            return Some((page, node));
        }
    }
    None
}

/// Find a named layer in the canvas children
fn find_layer<'a>(data: &'a Value, name: &str) -> Option<&'a Value> {
    let canvas = get_canvas(data)?;
    children(canvas)
        .iter()
        .find(|c| node_name(c) == name)
}

/// Find a node anywhere in the tree by name
fn find_node_by_name<'a>(root: &'a Value, name: &str) -> Option<&'a Value> {
    let mut found = None;
    walk(root, &mut |node| {
        if node_name(node) == name {
            found = Some(node);
            true
        } else {
            false
        }
    });
    found
}

// ─────────────────────────────────────────────────────────────────────────────
// Commands
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_layers(data: &Value, out: &mut dyn Write) {
    writeln!(out, "# Layers (top-level frames on canvas)\n").unwrap();
    writeln!(out, "{:<30} {:>12} {:>20} {}", "Name", "Size (w×h)", "Position (x,y)", "Fill").unwrap();
    writeln!(out, "{}", "─".repeat(80)).unwrap();
    if let Some(canvas) = get_canvas(data) {
        for child in children(canvas) {
            let name = node_name(child);
            let (w, h) = node_size(child);
            let (x, y) = node_pos(child);
            let fill = child
                .get("fillPaints")
                .and_then(|f| f.as_array())
                .and_then(|a| a.first())
                .map(|fp| {
                    let vis = fp.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
                    let c = fp.get("color").unwrap_or(&Value::Null);
                    let hex = rgba_to_hex(c);
                    if vis { hex } else { format!("{} (hidden)", hex) }
                })
                .unwrap_or_else(|| "—".to_string());
            writeln!(out, "{:<30} {:>5}×{:<5} {:>9},{:<9} {}", name, w as i32, h as i32, x as i32, y as i32, fill).unwrap();
        }
    }
}

fn print_tree(node: &Value, indent: usize, max_depth: Option<usize>, out: &mut dyn Write) {
    let depth = indent / 2;
    if let Some(max) = max_depth {
        if depth > max {
            return;
        }
    }
    let name = node_name(node);
    let ntype = node_type(node);
    let (w, h) = node_size(node);
    let (x, y) = node_pos(node);

    // Summarise fill
    let fill_summary = node
        .get("fillPaints")
        .and_then(|f| f.as_array())
        .and_then(|a| a.first())
        .map(|fp| {
            let vis = fp.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
            let ftype = fp.get("type").and_then(|t| {
                if let Some(obj) = t.as_object() {
                    obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    t.as_str().map(|s| s.to_string())
                }
            }).unwrap_or_default();
            if ftype == "IMAGE" {
                if vis { "fill=IMAGE".to_string() } else { "fill=IMAGE(hidden)".to_string() }
            } else {
                let c = fp.get("color").unwrap_or(&Value::Null);
                let hex = rgba_to_hex(c);
                if vis { format!("fill={}", hex) } else { format!("fill={}(hidden)", hex) }
            }
        })
        .unwrap_or_default();

    let cr = node.get("cornerRadius").and_then(|v| v.as_f64())
        .map(|r| format!(" r={}", r)).unwrap_or_default();

    let has_interactions = node
        .get("prototypeInteractions")
        .and_then(|p| p.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);

    let interaction_marker = if has_interactions { " [INTERACTION]" } else { "" };

    writeln!(
        out,
        "{}{} [{}] {}×{} @({},{}){}{}{}",
        "  ".repeat(depth),
        name,
        ntype,
        w as i32, h as i32,
        x as i32, y as i32,
        if fill_summary.is_empty() { String::new() } else { format!(" {}", fill_summary) },
        cr,
        interaction_marker
    ).unwrap();

    for child in children(node) {
        print_tree(child, indent + 2, max_depth, out);
    }
}

fn cmd_tree(data: &Value, depth: Option<usize>, layer: Option<&str>, out: &mut dyn Write) {
    writeln!(out, "# Node Tree\n").unwrap();
    let root: &Value = if let Some(lname) = layer {
        match find_layer(data, lname) {
            Some(l) => l,
            None => {
                eprintln!("Layer '{}' not found. Use `layers` to list available layers.", lname);
                return;
            }
        }
    } else if node_type(data) == "CANVAS" {
        // data is already a page (canvas), use it directly
        data
    } else {
        // data is a full document, get the first page
        get_canvas(data).unwrap_or_else(|| get_document(data))
    };
    print_tree(root, 0, depth, out);
}

fn print_fills(fills: &Value, prefix: &str, out: &mut dyn Write) {
    if let Some(arr) = fills.as_array() {
        for (i, fp) in arr.iter().enumerate() {
            let vis = fp.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
            let ftype = fp.get("type").and_then(|t| {
                if let Some(obj) = t.as_object() {
                    obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    t.as_str().map(|s| s.to_string())
                }
            }).unwrap_or_default();
            let c = fp.get("color").unwrap_or(&Value::Null);
            let hex = rgba_to_hex(c);
            let img_hash = fp.get("image").and_then(|img| img.get("hash"))
                .map(|h| hash_to_hex(h)).unwrap_or_default();
            let img_filename = if !img_hash.is_empty() { format!(" → {}.png", img_hash) } else { String::new() };
            writeln!(out, "{}  [{}] type={} color={} visible={}{}", prefix, i, ftype, hex, vis, img_filename).unwrap();
        }
    }
}

fn cmd_node(data: &Value, name: &str, layer: Option<&str>, out: &mut dyn Write) {
    let search_root: &Value = if let Some(lname) = layer {
        match find_layer(data, lname) {
            Some(l) => l,
            None => {
                eprintln!("Layer '{}' not found.", lname);
                return;
            }
        }
    } else {
        get_document(data)
    };

    let node = match find_node_by_name(search_root, name) {
        Some(n) => n,
        None => {
            eprintln!("Node '{}' not found.", name);
            return;
        }
    };

    let ntype = node_type(node);
    let (w, h) = node_size(node);
    let (x, y) = node_pos(node);
    let cr = node.get("cornerRadius").and_then(|v| v.as_f64());
    let sw = node.get("strokeWeight").and_then(|v| v.as_f64());
    let opacity = node.get("opacity").and_then(|v| v.as_f64());
    let visible = node.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
    let is_mask = node.get("mask").and_then(|v| v.as_bool()).unwrap_or(false);

    writeln!(out, "# Node: {}\n", name).unwrap();
    writeln!(out, "type:         {}", ntype).unwrap();
    writeln!(out, "size:         {}×{}", w as i32, h as i32).unwrap();
    writeln!(out, "position:     ({}, {})", x as i32, y as i32).unwrap();
    if let Some(r) = cr { writeln!(out, "cornerRadius: {}px", r).unwrap(); }
    if let Some(o) = opacity { writeln!(out, "opacity:      {}", o).unwrap(); }
    if !visible { writeln!(out, "visible:      false  ← HIDDEN NODE").unwrap(); }
    if is_mask  { writeln!(out, "mask:         true  ← THIS NODE MASKS SIBLINGS").unwrap(); }
    if let Some(w) = sw { writeln!(out, "strokeWeight: {}px", w).unwrap(); }

    // Stroke align
    if let Some(sa) = node.get("strokeAlign") {
        let val = if let Some(obj) = sa.as_object() {
            obj.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string()
        } else {
            sa.as_str().unwrap_or("").to_string()
        };
        if !val.is_empty() { writeln!(out, "strokeAlign:  {}", val).unwrap(); }
    }

    // Fills
    if let Some(fills) = node.get("fillPaints") {
        if fills.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
            writeln!(out, "\nfillPaints:").unwrap();
            print_fills(fills, "", out);
        }
    }

    // Strokes
    if let Some(strokes) = node.get("strokePaints") {
        if strokes.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
            writeln!(out, "\nstrokePaints:").unwrap();
            print_fills(strokes, "", out);
        }
    }

    // Effects
    if let Some(effects) = node.get("effects").and_then(|e| e.as_array()) {
        if !effects.is_empty() {
            writeln!(out, "\neffects:").unwrap();
            for (i, eff) in effects.iter().enumerate() {
                let etype = eff.get("type").and_then(|t| {
                    if let Some(obj) = t.as_object() {
                        obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string())
                    } else { t.as_str().map(|s| s.to_string()) }
                }).unwrap_or_default();
                let c = eff.get("color").unwrap_or(&Value::Null);
                let hex = rgba_to_hex(c);
                let ox = get_f64(eff.get("offset").unwrap_or(&Value::Null), "x");
                let oy = get_f64(eff.get("offset").unwrap_or(&Value::Null), "y");
                let radius = eff.get("radius").and_then(|r| r.as_f64()).unwrap_or(0.0);
                let spread = eff.get("spread").and_then(|s| s.as_f64()).unwrap_or(0.0);
                let vis = eff.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
                writeln!(out, "  [{}] type={} color={} offset=({},{}) radius={} spread={} visible={}", i, etype, hex, ox, oy, radius, spread, vis).unwrap();
            }
        }
    }

    // Auto-layout
    if let Some(sm) = node.get("stackMode") {
        let val = if let Some(obj) = sm.as_object() {
            obj.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string()
        } else { sm.as_str().unwrap_or("").to_string() };
        if !val.is_empty() {
            writeln!(out, "\nauto-layout:").unwrap();
            writeln!(out, "  direction:   {}", val).unwrap();
            if let Some(gap) = node.get("stackSpacing").and_then(|v| v.as_f64()) {
                writeln!(out, "  gap:         {}px", gap).unwrap();
            }
            if let Some(ph) = node.get("stackHorizontalPadding").and_then(|v| v.as_f64()) {
                writeln!(out, "  padH:        {}px", ph).unwrap();
            }
            if let Some(pv) = node.get("stackVerticalPadding").and_then(|v| v.as_f64()) {
                writeln!(out, "  padV:        {}px", pv).unwrap();
            }
            if let Some(pr) = node.get("stackPaddingRight").and_then(|v| v.as_f64()) {
                writeln!(out, "  padRight:    {}px", pr).unwrap();
            }
            if let Some(pb) = node.get("stackPaddingBottom").and_then(|v| v.as_f64()) {
                writeln!(out, "  padBottom:   {}px", pb).unwrap();
            }
            let primary_align = node.get("stackPrimaryAlignItems").and_then(|a| {
                if let Some(obj) = a.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                else { a.as_str().map(|s| s.to_string()) }
            });
            let counter_align = node.get("stackCounterAlignItems").and_then(|a| {
                if let Some(obj) = a.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                else { a.as_str().map(|s| s.to_string()) }
            });
            if let Some(pa) = primary_align { writeln!(out, "  primaryAlign: {} (justify-content)", pa).unwrap(); }
            if let Some(ca) = counter_align { writeln!(out, "  counterAlign: {} (align-items)", ca).unwrap(); }
            if let Some(grow) = node.get("stackChildPrimaryGrow").and_then(|v| v.as_f64()) {
                if grow > 0.0 { writeln!(out, "  childGrow:   {} (flex: {})", grow, grow as i32).unwrap(); }
            }
        }
    }

    // Text
    if ntype == "TEXT" {
        if let Some(td) = node.get("textData") {
            let chars = td.get("characters").and_then(|c| c.as_str()).unwrap_or("");
            writeln!(out, "\ntext content: {:?}", chars).unwrap();
        }
        let fname = node.get("fontName").unwrap_or(&Value::Null);
        let family = fname.get("family").and_then(|v| v.as_str()).unwrap_or("—");
        let style  = fname.get("style").and_then(|v| v.as_str()).unwrap_or("—");
        let fsize  = node.get("fontSize").and_then(|v| v.as_f64()).unwrap_or(0.0);
        writeln!(out, "font:         {} {} {}px", family, style, fsize).unwrap();

        let lh = node.get("lineHeight").unwrap_or(&Value::Null);
        let lh_val = lh.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lh_unit = lh.get("units").and_then(|u| {
            if let Some(obj) = u.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
            else { u.as_str().map(|s| s.to_string()) }
        }).unwrap_or_default();
        writeln!(out, "lineHeight:   {} {}", lh_val, lh_unit).unwrap();

        let ls = node.get("letterSpacing").unwrap_or(&Value::Null);
        let ls_val = ls.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ls_unit = ls.get("units").and_then(|u| {
            if let Some(obj) = u.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
            else { u.as_str().map(|s| s.to_string()) }
        }).unwrap_or_default();
        writeln!(out, "letterSpacing:{} {}", ls_val, ls_unit).unwrap();

        let align_h = node.get("textAlignHorizontal").and_then(|a| {
            if let Some(obj) = a.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
            else { a.as_str().map(|s| s.to_string()) }
        });
        let align_v = node.get("textAlignVertical").and_then(|a| {
            if let Some(obj) = a.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
            else { a.as_str().map(|s| s.to_string()) }
        });
        if let Some(ah) = align_h { writeln!(out, "alignH:       {}", ah).unwrap(); }
        if let Some(av) = align_v { writeln!(out, "alignV:       {}", av).unwrap(); }
    }

    // Prototype interactions
    if let Some(interactions) = node.get("prototypeInteractions").and_then(|p| p.as_array()) {
        if !interactions.is_empty() {
            let guid_map = build_guid_map(get_document(data));
            writeln!(out, "\nprototypeInteractions: {} found", interactions.len()).unwrap();
            for (i, interaction) in interactions.iter().enumerate() {
                let event_type = interaction.get("event")
                    .and_then(|e| e.get("interactionType"))
                    .and_then(|t| {
                        if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                        else { t.as_str().map(|s| s.to_string()) }
                    }).unwrap_or_default();
                writeln!(out, "  [{}] trigger: {}", i, event_type).unwrap();
                if let Some(actions) = interaction.get("actions").and_then(|a| a.as_array()) {
                    for action in actions {
                        let conn_type = action.get("connectionType").and_then(|t| {
                            if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                            else { t.as_str().map(|s| s.to_string()) }
                        }).unwrap_or_default();
                        let nav_type = action.get("navigationType").and_then(|t| {
                            if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                            else { t.as_str().map(|s| s.to_string()) }
                        }).unwrap_or_default();
                        let dest = if let Some(tid) = action.get("transitionNodeID") {
                            let sid = tid.get("sessionID").and_then(|x| x.as_u64()).unwrap_or(0);
                            let lid = tid.get("localID").and_then(|x| x.as_u64()).unwrap_or(0);
                            let key = format!("{}:{}", sid, lid);
                            guid_map.get(&key).cloned().unwrap_or(format!("GUID:{}:{}", sid, lid))
                        } else { "—".to_string() };
                        let url = action.get("connectionURL").and_then(|u| u.as_str()).unwrap_or("");
                        let trans_type = action.get("transitionType").and_then(|t| {
                            if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                            else { t.as_str().map(|s| s.to_string()) }
                        }).unwrap_or_default();
                        let duration = action.get("transitionDuration").and_then(|v| v.as_f64());
                        let easing = action.get("easingType").and_then(|t| {
                            if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                            else { t.as_str().map(|s| s.to_string()) }
                        });
                        write!(out, "      connection={} nav={} dest={}", conn_type, nav_type, dest).unwrap();
                        if !url.is_empty() { write!(out, " url={}", url).unwrap(); }
                        if !trans_type.is_empty() { write!(out, " transition={}", trans_type).unwrap(); }
                        if let Some(d) = duration { write!(out, " duration={}ms", (d * 1000.0) as i32).unwrap(); }
                        if let Some(e) = easing { write!(out, " easing={}", e).unwrap(); }
                        writeln!(out).unwrap();
                    }
                }
            }
        }
    }
}

fn cmd_texts(data: &Value, layer: Option<&str>, out: &mut dyn Write) {
    let root: &Value = if let Some(lname) = layer {
        match find_layer(data, lname) {
            Some(l) => l,
            None => { eprintln!("Layer '{}' not found.", lname); return; }
        }
    } else {
        get_document(data)
    };

    writeln!(out, "# Text Nodes\n").unwrap();
    writeln!(out, "{:<40} {:>8} {:>15} {:>8}  {}", "Node path", "Size", "Font", "Color", "Content").unwrap();
    writeln!(out, "{}", "─".repeat(120)).unwrap();

    fn walk_texts(node: &Value, path: &str, out: &mut dyn Write) {
        let name = node_name(node);
        let current_path = if path.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", path, name)
        };

        if node_type(node) == "TEXT" {
            let (w, h) = node_size(node);
            let fname = node.get("fontName").unwrap_or(&Value::Null);
            let family = fname.get("family").and_then(|v| v.as_str()).unwrap_or("—");
            let style  = fname.get("style").and_then(|v| v.as_str()).unwrap_or("—");
            let fsize  = node.get("fontSize").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let color = node.get("fillPaints")
                .and_then(|f| f.as_array())
                .and_then(|a| a.first())
                .map(|fp| rgba_to_hex(fp.get("color").unwrap_or(&Value::Null)))
                .unwrap_or_else(|| "—".to_string());
            let chars = node.get("textData")
                .and_then(|td| td.get("characters"))
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let preview: String = chars.chars().take(60).collect();
            writeln!(
                out,
                "{:<40} {:>4}×{:<4} {:>12}/{:<3} {:>8}px  {}",
                if current_path.len() > 40 { &current_path[current_path.len()-40..] } else { &current_path },
                w as i32, h as i32,
                family, style,
                fsize as i32,
                color,
            ).unwrap();
            writeln!(out, "{:<40}   content: {:?}", "", preview).unwrap();
        }

        for child in children(node) {
            walk_texts(child, &current_path, out);
        }
    }

    walk_texts(root, "", out);
}

fn cmd_images(data: &Value, layer: Option<&str>, images_dir: Option<&PathBuf>, out: &mut dyn Write) {
    let root: &Value = if let Some(lname) = layer {
        match find_layer(data, lname) {
            Some(l) => l,
            None => { eprintln!("Layer '{}' not found.", lname); return; }
        }
    } else {
        get_document(data)
    };

    writeln!(out, "# Image Fills\n").unwrap();

    // Collect unique hashes with their usage
    let mut seen: HashMap<String, Vec<(String, i32, i32, i32, i32)>> = HashMap::new();

    fn walk_images(node: &Value, path: &str, seen: &mut HashMap<String, Vec<(String, i32, i32, i32, i32)>>) {
        let name = node_name(node);
        let current_path = if path.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", path, name)
        };

        if let Some(fills) = node.get("fillPaints").and_then(|f| f.as_array()) {
            for fp in fills {
                let ftype = fp.get("type").and_then(|t| {
                    if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                    else { t.as_str().map(|s| s.to_string()) }
                }).unwrap_or_default();
                if ftype == "IMAGE" {
                    if let Some(img) = fp.get("image") {
                        let hash_hex = hash_to_hex(img.get("hash").unwrap_or(&Value::Null));
                        if !hash_hex.is_empty() {
                            let (w, h) = node_size(node);
                            let (x, y) = node_pos(node);
                            seen.entry(format!("{}.png", hash_hex))
                                .or_default()
                                .push((current_path.clone(), w as i32, h as i32, x as i32, y as i32));
                        }
                    }
                }
            }
        }

        for child in children(node) {
            walk_images(child, &current_path, seen);
        }
    }

    walk_images(root, "", &mut seen);

    // Deduplicate by first occurrence of each hash
    let mut entries: Vec<(String, Vec<(String, i32, i32, i32, i32)>)> = seen.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    for (filename, usages) in &entries {
        let exists = images_dir
            .map(|d| d.join(filename).exists())
            .map(|e| if e { "✓" } else { "✗ MISSING" })
            .unwrap_or("?");

        writeln!(out, "File: {} [{}]", filename, exists).unwrap();
        // Deduplicate usages by path
        let mut seen_paths: Vec<&str> = Vec::new();
        for (path, w, h, x, y) in usages {
            if !seen_paths.contains(&path.as_str()) {
                seen_paths.push(path.as_str());
                // Show last 2 path components for readability
                let short_path: String = path.split('/').rev().take(3).collect::<Vec<_>>().iter().rev().cloned().collect::<Vec<_>>().join("/");
                writeln!(out, "  used in: {} (node: {}×{} @({},{}))", short_path, w, h, x, y).unwrap();
            }
        }
        writeln!(out).unwrap();
    }

    writeln!(out, "Total unique image hashes: {}", entries.len()).unwrap();
}

fn cmd_interactions(data: &Value, layer: Option<&str>, out: &mut dyn Write) {
    let root: &Value = if let Some(lname) = layer {
        match find_layer(data, lname) {
            Some(l) => l,
            None => { eprintln!("Layer '{}' not found.", lname); return; }
        }
    } else {
        get_document(data)
    };

    let guid_map = build_guid_map(get_document(data));

    writeln!(out, "# Prototype Interactions\n").unwrap();
    writeln!(out, "{:<3} {:<30} {:<15} {:<12} {:<12} {:<30} {:<15} {:>8}", "#", "Trigger node", "Parent layer", "Event", "Action", "Destination", "Transition", "Duration").unwrap();
    writeln!(out, "{}", "─".repeat(140)).unwrap();

    let mut count = 0;

    fn walk_interactions(node: &Value, path: &str, guid_map: &HashMap<String, String>, count: &mut i32, out: &mut dyn Write) {
        let name = node_name(node);
        let current_path = if path.is_empty() { name.to_string() } else { format!("{}/{}", path, name) };

        if let Some(interactions) = node.get("prototypeInteractions").and_then(|p| p.as_array()) {
            for interaction in interactions {
                *count += 1;
                let event_type = interaction.get("event")
                    .and_then(|e| e.get("interactionType"))
                    .and_then(|t| {
                        if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                        else { t.as_str().map(|s| s.to_string()) }
                    }).unwrap_or_default();

                let parent = current_path.split('/').rev().nth(1).unwrap_or("—").to_string();

                if let Some(actions) = interaction.get("actions").and_then(|a| a.as_array()) {
                    for action in actions {
                        let conn_type = action.get("connectionType").and_then(|t| {
                            if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                            else { t.as_str().map(|s| s.to_string()) }
                        }).unwrap_or_default();
                        let nav_type = action.get("navigationType").and_then(|t| {
                            if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                            else { t.as_str().map(|s| s.to_string()) }
                        }).unwrap_or_default();
                        let dest = if let Some(tid) = action.get("transitionNodeID") {
                            let sid = tid.get("sessionID").and_then(|x| x.as_u64()).unwrap_or(0);
                            let lid = tid.get("localID").and_then(|x| x.as_u64()).unwrap_or(0);
                            let key = format!("{}:{}", sid, lid);
                            guid_map.get(&key).cloned().unwrap_or(if sid == 0 && lid == 0 { "—".to_string() } else { format!("GUID:{}:{}", sid, lid) })
                        } else {
                            action.get("connectionURL").and_then(|u| u.as_str())
                                .map(|u| format!("URL:{}", u))
                                .unwrap_or_else(|| "—".to_string())
                        };
                        let trans_type = action.get("transitionType").and_then(|t| {
                            if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                            else { t.as_str().map(|s| s.to_string()) }
                        }).unwrap_or_default();
                        let duration = action.get("transitionDuration").and_then(|v| v.as_f64())
                            .map(|d| format!("{}ms", (d * 1000.0) as i32))
                            .unwrap_or_else(|| "—".to_string());
                        writeln!(
                            out,
                            "{:<3} {:<30} {:<15} {:<12} {:<12} {:<30} {:<15} {:>8}",
                            count,
                            if name.len() > 30 { &name[..30] } else { name },
                            if parent.len() > 15 { &parent[..15] } else { &parent },
                            event_type,
                            format!("{}/{}", conn_type, nav_type),
                            if dest.len() > 30 { &dest[..30] } else { &dest },
                            trans_type,
                            duration
                        ).unwrap();
                        if !conn_type.contains("NONE") {
                            // Show full destination if truncated
                            if dest.len() > 30 {
                                writeln!(out, "    dest (full): {}", dest).unwrap();
                            }
                        }
                    }
                }
            }
        }

        for child in children(node) {
            walk_interactions(child, &current_path, guid_map, count, out);
        }
    }

    walk_interactions(root, "", &guid_map, &mut count, out);
    writeln!(out, "\nTotal interactions: {}", count).unwrap();
}

fn cmd_tokens(data: &Value, layer: Option<&str>, out: &mut dyn Write) {
    let root: &Value = if let Some(lname) = layer {
        match find_layer(data, lname) {
            Some(l) => l,
            None => { eprintln!("Layer '{}' not found.", lname); return; }
        }
    } else {
        get_document(data)
    };

    let mut colors: Vec<(String, String)> = Vec::new(); // (hex, usage)
    let mut fonts: Vec<(String, String, f64, String)> = Vec::new(); // (family, style, size, usage)
    let mut radii: Vec<(f64, String)> = Vec::new();
    let mut gaps: Vec<(f64, String)> = Vec::new();

    fn walk_tokens(
        node: &Value, path: &str,
        colors: &mut Vec<(String, String)>,
        fonts: &mut Vec<(String, String, f64, String)>,
        radii: &mut Vec<(f64, String)>,
        gaps: &mut Vec<(f64, String)>,
    ) {
        let name = node_name(node);
        let current_path = if path.is_empty() { name.to_string() } else { format!("{}/{}", path, name) };
        let short: String = current_path.split('/').rev().take(2).collect::<Vec<_>>().iter().rev().cloned().collect::<Vec<_>>().join("/");

        // Colors from fills
        let empty_vec: Vec<Value> = Vec::new();
        let fill_arr = node.get("fillPaints").and_then(|f| f.as_array()).unwrap_or(&empty_vec);
        for fp in fill_arr {
            let ftype = fp.get("type").and_then(|t| {
                if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                else { t.as_str().map(|s| s.to_string()) }
            }).unwrap_or_default();
            if ftype == "SOLID" {
                let hex = rgba_to_hex(fp.get("color").unwrap_or(&Value::Null));
                if !colors.iter().any(|(h, _)| h == &hex) {
                    colors.push((hex, short.clone()));
                }
            }
        }
        // Colors from strokes
        let empty_vec2: Vec<Value> = Vec::new();
        let stroke_arr = node.get("strokePaints").and_then(|f| f.as_array()).unwrap_or(&empty_vec2);
        for sp in stroke_arr {
            let hex = rgba_to_hex(sp.get("color").unwrap_or(&Value::Null));
            if hex != "—" && !colors.iter().any(|(h, _)| h == &hex) {
                colors.push((hex, format!("stroke:{}", short)));
            }
        }

        // Fonts
        if node_type(node) == "TEXT" {
            let fname = node.get("fontName").unwrap_or(&Value::Null);
            let family = fname.get("family").and_then(|v| v.as_str()).unwrap_or("—").to_string();
            let style  = fname.get("style").and_then(|v| v.as_str()).unwrap_or("—").to_string();
            let fsize  = node.get("fontSize").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if !fonts.iter().any(|(f, s, sz, _)| f == &family && s == &style && (sz - fsize).abs() < 0.1) {
                fonts.push((family, style, fsize, short.clone()));
            }
        }

        // Corner radius
        if let Some(cr) = node.get("cornerRadius").and_then(|v| v.as_f64()) {
            if cr > 0.0 && !radii.iter().any(|(r, _)| (r - cr).abs() < 0.1) {
                radii.push((cr, short.clone()));
            }
        }

        // Auto-layout gap
        if let Some(gap) = node.get("stackSpacing").and_then(|v| v.as_f64()) {
            if gap > 0.0 && !gaps.iter().any(|(g, _)| (g - gap).abs() < 0.1) {
                gaps.push((gap, short.clone()));
            }
        }

        for child in children(node) {
            walk_tokens(child, &current_path, colors, fonts, radii, gaps);
        }
    }

    walk_tokens(root, "", &mut colors, &mut fonts, &mut radii, &mut gaps);

    writeln!(out, "# Design Tokens\n").unwrap();

    writeln!(out, "## Colours ({} unique)\n", colors.len()).unwrap();
    for (hex, usage) in &colors {
        writeln!(out, "  {}  (first seen: {})", hex, usage).unwrap();
    }

    writeln!(out, "\n## Typography ({} unique combinations)\n", fonts.len()).unwrap();
    fonts.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    for (family, style, size, usage) in &fonts {
        writeln!(out, "  {} {} {}px  (e.g. {})", family, style, size, usage).unwrap();
    }

    writeln!(out, "\n## Corner Radii\n").unwrap();
    radii.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    for (r, usage) in &radii {
        writeln!(out, "  {}px  (e.g. {})", r, usage).unwrap();
    }

    writeln!(out, "\n## Auto-layout Gaps\n").unwrap();
    for (g, usage) in &gaps {
        writeln!(out, "  {}px  (e.g. {})", g, usage).unwrap();
    }
}

fn cmd_raw(data: &Value, name: &str, path: Option<&str>, out: &mut dyn Write) {
    let node = match find_node_by_name(get_document(data), name) {
        Some(n) => n,
        None => { eprintln!("Node '{}' not found.", name); return; }
    };

    let target = if let Some(ptr) = path {
        // Simple JSON pointer traversal
        let mut cur = node;
        for key in ptr.trim_start_matches('/').split('/') {
            if key.is_empty() { continue; }
            if let Ok(idx) = key.parse::<usize>() {
                cur = cur.get(idx).unwrap_or(&Value::Null);
            } else {
                cur = cur.get(key).unwrap_or(&Value::Null);
            }
        }
        cur
    } else {
        node
    };

    writeln!(out, "{}", serde_json::to_string_pretty(target).unwrap_or_default()).unwrap();
}

// ─────────────────────────────────────────────────────────────────────────────
// Output routing helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Map command variant to output filename
fn cmd_filename(cmd: &Commands) -> &'static str {
    match cmd {
        Commands::Tree { .. } => "tree.txt",
        Commands::Node { .. } => "node.txt",
        Commands::Texts { .. } => "texts.txt",
        Commands::Images { .. } => "images.txt",
        Commands::Interactions { .. } => "interactions.txt",
        Commands::Tokens { .. } => "tokens.txt",
        Commands::Layers => "layers.txt",
        Commands::Raw { .. } => "raw.txt",
        Commands::All => "all.txt",
    }
}

/// Dispatch command to appropriate cmd_xxx function
fn run_cmd(cmd: &Commands, data: &Value, out: &mut dyn Write) {
    match cmd {
        Commands::Layers => cmd_layers(data, out),
        Commands::Tree { depth, layer } => cmd_tree(data, *depth, layer.as_deref(), out),
        Commands::Node { name, layer } => cmd_node(data, name, layer.as_deref(), out),
        Commands::Texts { layer } => cmd_texts(data, layer.as_deref(), out),
        Commands::Images { layer, images_dir } => cmd_images(data, layer.as_deref(), images_dir.as_ref(), out),
        Commands::Interactions { layer } => cmd_interactions(data, layer.as_deref(), out),
        Commands::Tokens { layer } => cmd_tokens(data, layer.as_deref(), out),
        Commands::Raw { name, path } => cmd_raw(data, name, path.as_deref(), out),
        Commands::All => unreachable!("all should be handled separately"),
    }
}

/// Execute a single command, saving output to a file in output_dir
fn execute_to_file(cmd: &Commands, canvas: &Value, output_dir: &Path) {
    let filename = cmd_filename(cmd);
    let filepath = output_dir.join(filename);
    match fs::File::create(&filepath) {
        Ok(mut file) => {
            run_cmd(cmd, canvas, &mut file);
            eprintln!("✓ Saved {:?} to {:?}", cmd_filename(cmd), filepath);
        }
        Err(e) => {
            eprintln!("✗ Failed to create {:?}: {}", filepath, e);
        }
    }
}

/// Run all commands (tree, texts, images, interactions, tokens, layers) and save each to file
fn run_all_for_page(page: &Value, page_dir: &Path) {
    let commands: Vec<Commands> = vec![
        Commands::Tree { depth: None, layer: None },
        Commands::Texts { layer: None },
        Commands::Images { layer: None, images_dir: None },
        Commands::Interactions { layer: None },
        Commands::Tokens { layer: None },
        Commands::Layers,
    ];

    for cmd in &commands {
        let name = cmd_filename(cmd);
        eprint!("Running {:<15}", name);
        execute_to_file(cmd, page, page_dir);
    }
}

/// Generate index.md in output root
fn generate_index_md(pages: &[(&str, &str, &str)], output_dir: &Path) -> Result<(), String> {
    let filepath = output_dir.join("index.md");
    let mut file = fs::File::create(&filepath)
        .map_err(|e| format!("Failed to create index.md: {}", e))?;
    
    writeln!(file, "# Pages\n").map_err(|e| format!("Failed to write index.md: {}", e))?;
    writeln!(file, "| # | Page Name | Folder | Children |").map_err(|e| format!("Failed to write index.md: {}", e))?;
    writeln!(file, "|---|-----------|--------|----------|").map_err(|e| format!("Failed to write index.md: {}", e))?;
    
    for (i, (name, folder, _desc)) in pages.iter().enumerate() {
        writeln!(file, "| {} | {} | {} | - |", i + 1, name, folder)
            .map_err(|e| format!("Failed to write index.md: {}", e))?;
    }
    
    Ok(())
}

/// Print error JSON to stdout
fn print_error(msg: &str) {
    println!("{}", serde_json::json!({
        "success": false,
        "msg": msg
    }));
}

/// Print success JSON to stdout
fn print_success(pages: &[(&str, &str)]) {
    println!("{}", serde_json::json!({
        "success": true,
        "page_count": pages.len(),
        "pages": pages.iter().map(|(name, folder)| {
            serde_json::json!({"name": name, "folder": folder})
        }).collect::<Vec<_>>()
    }));
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    let content = match fs::read_to_string(&cli.input) {
        Ok(c) => c,
        Err(e) => {
            print_error(&format!("Error reading input file: {}", e));
            std::process::exit(1);
        }
    };

    let data: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            print_error(&format!("Error parsing JSON: {}", e));
            std::process::exit(1);
        }
    };

    let pages = get_all_canvases(&data);
    // Filter out internal pages
    let pages: Vec<&Value> = pages.into_iter()
        .filter(|p| !node_name(p).contains("Internal Only"))
        .collect();
    if pages.is_empty() {
        print_error("No pages found in document");
        std::process::exit(1);
    }

    // Handle 'all' command first
    if let Commands::All = &cli.command {
        if let Some(ref output_dir) = cli.output {
            fs::create_dir_all(output_dir).unwrap_or_else(|e| {
                print_error(&format!("Error creating output directory: {}", e));
                std::process::exit(1);
            });

            let mut page_infos: Vec<(&str, &str)> = Vec::new();
            let mut used_folders: std::collections::HashSet<String> = std::collections::HashSet::new();

            for page in &pages {
                let name = node_name(page);
                let mut folder = sanitize_folder_name(name);
                
                // Handle folder name conflicts
                if used_folders.contains(&folder) {
                    let mut counter = 1;
                    loop {
                        let new_folder = format!("{}_{}", folder, counter);
                        if !used_folders.contains(&new_folder) {
                            folder = new_folder;
                            break;
                        }
                        counter += 1;
                    }
                }
                used_folders.insert(folder.clone());

                let page_dir = output_dir.join(&folder);
                fs::create_dir_all(&page_dir).unwrap_or_else(|e| {
                    eprintln!("✗ Failed to create directory {:?}: {}", page_dir, e);
                });

                eprintln!("Processing page: {} -> {:?}", name, folder);
                run_all_for_page(page, &page_dir);
                page_infos.push((name, folder.leak()));
            }

            // Generate index.md
            let page_refs: Vec<(&str, &str, &str)> = page_infos.iter()
                .map(|(name, folder)| (*name, *folder, ""))
                .collect();
            if let Err(e) = generate_index_md(&page_refs, output_dir) {
                eprintln!("✗ Failed to generate index.md: {}", e);
            }

            print_success(&page_infos);
        } else {
            print_error("'all' command requires the -o flag to specify output directory.");
            std::process::exit(1);
        }
        return;
    }

    // Single command execution
    if let Some(ref output_dir) = cli.output {
        fs::create_dir_all(output_dir).unwrap_or_else(|e| {
            print_error(&format!("Error creating output directory: {}", e));
            std::process::exit(1);
        });

        let mut page_infos: Vec<(&str, &str)> = Vec::new();
        let mut used_folders: std::collections::HashSet<String> = std::collections::HashSet::new();

        match &cli.command {
            Commands::Node { name, layer } => {
                // Search for node across all pages (or within specified layer)
                if let Some(_layer_name) = layer {
                    // When layer is specified, search within that layer across all pages
                    match find_layer_across_pages(&data, layer.as_ref().unwrap()) {
                        Some((page, _layer)) => {
                            let page_name = node_name(page);
                            let mut folder = sanitize_folder_name(page_name);
                            
                            if used_folders.contains(&folder) {
                                let mut counter = 1;
                                loop {
                                    let new_folder = format!("{}_{}", folder, counter);
                                    if !used_folders.contains(&new_folder) {
                                        folder = new_folder;
                                        break;
                                    }
                                    counter += 1;
                                }
                            }
                            used_folders.insert(folder.clone());

                            let page_dir = output_dir.join(&folder);
                            fs::create_dir_all(&page_dir).unwrap_or_else(|e| {
                                eprintln!("✗ Failed to create directory {:?}: {}", page_dir, e);
                            });

                            execute_to_file(&cli.command, page, &page_dir);
                            page_infos.push((page_name, folder.leak()));
                        }
                        None => {
                            print_error(&format!("Layer '{}' not found in any page", layer.as_ref().unwrap()));
                            std::process::exit(1);
                        }
                    }
                } else {
                    // Search for node across all pages
                    match find_node_across_pages(&data, name) {
                        Some((page, _node)) => {
                            let page_name = node_name(page);
                            let mut folder = sanitize_folder_name(page_name);
                            
                            if used_folders.contains(&folder) {
                                let mut counter = 1;
                                loop {
                                    let new_folder = format!("{}_{}", folder, counter);
                                    if !used_folders.contains(&new_folder) {
                                        folder = new_folder;
                                        break;
                                    }
                                    counter += 1;
                                }
                            }
                            used_folders.insert(folder.clone());

                            let page_dir = output_dir.join(&folder);
                            fs::create_dir_all(&page_dir).unwrap_or_else(|e| {
                                eprintln!("✗ Failed to create directory {:?}: {}", page_dir, e);
                            });

                            execute_to_file(&cli.command, page, &page_dir);
                            page_infos.push((page_name, folder.leak()));
                        }
                        None => {
                            print_error(&format!("Node '{}' not found in any page", name));
                            std::process::exit(1);
                        }
                    }
                }
            }
            _ => {
                // For all other commands, process all pages
                for page in &pages {
                    let name = node_name(page);
                    let mut folder = sanitize_folder_name(name);
                    
                    if used_folders.contains(&folder) {
                        let mut counter = 1;
                        loop {
                            let new_folder = format!("{}_{}", folder, counter);
                            if !used_folders.contains(&new_folder) {
                                folder = new_folder;
                                break;
                            }
                            counter += 1;
                        }
                    }
                    used_folders.insert(folder.clone());

                    let page_dir = output_dir.join(&folder);
                    fs::create_dir_all(&page_dir).unwrap_or_else(|e| {
                        eprintln!("✗ Failed to create directory {:?}: {}", page_dir, e);
                    });

                    execute_to_file(&cli.command, page, &page_dir);
                    page_infos.push((name, folder.leak()));
                }
            }
        }

        // Generate index.md
        let page_refs: Vec<(&str, &str, &str)> = page_infos.iter()
            .map(|(name, folder)| (*name, *folder, ""))
            .collect();
        if let Err(e) = generate_index_md(&page_refs, output_dir) {
            eprintln!("✗ Failed to generate index.md: {}", e);
        }

        print_success(&page_infos);
    } else {
        // Print to stdout (no output directory specified)
        // For single page, just run the command
        if pages.len() == 1 {
            run_cmd(&cli.command, pages[0], &mut std::io::stdout());
        } else {
            // For multiple pages, we need to specify -o flag
            print_error("Multiple pages found. Please use -o flag to specify output directory.");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sanitize_folder_name() {
        assert_eq!(sanitize_folder_name("Page 1"), "page_1");
        assert_eq!(sanitize_folder_name("Lounge booking - make apt flow"), "lounge_booking_-_make_apt_flow");
        assert_eq!(sanitize_folder_name("Design System"), "design_system");
        assert_eq!(sanitize_folder_name("Page 1 (Draft)"), "page_1_draft");
        assert_eq!(sanitize_folder_name("Page  1  Draft"), "page_1_draft");
        assert_eq!(sanitize_folder_name("ABC"), "abc");
        assert_eq!(sanitize_folder_name("123"), "123");
        assert_eq!(sanitize_folder_name("a_b-c"), "a_b-c");
    }

    #[test]
    fn test_get_all_canvases() {
        let data = json!({
            "document": {
                "children": [
                    {"name": "Page 1", "type": "CANVAS"},
                    {"name": "Page 2", "type": "CANVAS"}
                ]
            }
        });
        let canvases = get_all_canvases(&data);
        assert_eq!(canvases.len(), 2);
        assert_eq!(node_name(canvases[0]), "Page 1");
        assert_eq!(node_name(canvases[1]), "Page 2");
    }

    #[test]
    fn test_get_all_canvases_empty() {
        let data = json!({
            "document": {}
        });
        let canvases = get_all_canvases(&data);
        assert_eq!(canvases.len(), 0);
    }

    #[test]
    fn test_find_layer_across_pages() {
        let data = json!({
            "document": {
                "children": [
                    {
                        "name": "Page 1",
                        "children": [
                            {"name": "Frame 1", "type": "FRAME"},
                            {"name": "Frame 2", "type": "FRAME"}
                        ]
                    },
                    {
                        "name": "Page 2",
                        "children": [
                            {"name": "Frame 3", "type": "FRAME"}
                        ]
                    }
                ]
            }
        });
        
        let result = find_layer_across_pages(&data, "Frame 3");
        assert!(result.is_some());
        let (page, layer) = result.unwrap();
        assert_eq!(node_name(page), "Page 2");
        assert_eq!(node_name(layer), "Frame 3");
    }

    #[test]
    fn test_find_layer_across_pages_not_found() {
        let data = json!({
            "document": {
                "children": [
                    {
                        "name": "Page 1",
                        "children": [
                            {"name": "Frame 1", "type": "FRAME"}
                        ]
                    }
                ]
            }
        });
        
        let result = find_layer_across_pages(&data, "Nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_node_across_pages() {
        let data = json!({
            "document": {
                "children": [
                    {
                        "name": "Page 1",
                        "children": [
                            {
                                "name": "Button",
                                "type": "FRAME",
                                "children": [
                                    {"name": "Text", "type": "TEXT"}
                                ]
                            }
                        ]
                    },
                    {
                        "name": "Page 2",
                        "children": [
                            {"name": "Card", "type": "FRAME"}
                        ]
                    }
                ]
            }
        });
        
        let result = find_node_across_pages(&data, "Text");
        assert!(result.is_some());
        let (page, node) = result.unwrap();
        assert_eq!(node_name(page), "Page 1");
        assert_eq!(node_name(node), "Text");
    }

    #[test]
    fn test_find_node_across_pages_not_found() {
        let data = json!({
            "document": {
                "children": [
                    {
                        "name": "Page 1",
                        "children": [
                            {"name": "Button", "type": "FRAME"}
                        ]
                    }
                ]
            }
        });
        
        let result = find_node_across_pages(&data, "Nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_index_md() {
        let temp_dir = std::env::temp_dir().join("test_generate_index_md");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        
        let pages = vec![
            ("Page 1", "page_1", "Description 1"),
            ("Page 2", "page_2", "Description 2"),
        ];
        
        let result = generate_index_md(&pages, &temp_dir);
        assert!(result.is_ok());
        
        let content = fs::read_to_string(temp_dir.join("index.md")).unwrap();
        assert!(content.contains("# Pages"));
        assert!(content.contains("Page 1"));
        assert!(content.contains("page_1"));
        assert!(content.contains("Page 2"));
        assert!(content.contains("page_2"));
        
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_print_error() {
        // This just verifies the function doesn't panic
        print_error("Test error message");
    }

    #[test]
    fn test_print_success() {
        let pages = vec![("Page 1", "page_1"), ("Page 2", "page_2")];
        // This just verifies the function doesn't panic
        print_success(&pages);
    }

    #[test]
    fn test_integration_multi_page_all_command() {
        let temp_dir = std::env::temp_dir().join("test_integration_multi_page");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let data = json!({
            "document": {
                "children": [
                    {
                        "name": "Page 1",
                        "children": [
                            {"name": "Frame 1", "type": "FRAME"}
                        ]
                    },
                    {
                        "name": "Page 2",
                        "children": [
                            {"name": "Frame 2", "type": "FRAME"}
                        ]
                    }
                ]
            }
        });

        let canvases = get_all_canvases(&data);
        assert_eq!(canvases.len(), 2);

        let mut folder_map = HashMap::new();
        for (idx, canvas) in canvases.iter().enumerate() {
            let raw_name = node_name(canvas);
            let mut folder_name = sanitize_folder_name(raw_name);
            let entry = folder_map.entry(folder_name.clone()).or_insert(0);
            if *entry > 0 {
                folder_name = format!("{}_{}", folder_name, entry);
            }
            *entry += 1;

            let page_dir = temp_dir.join(&folder_name);
            fs::create_dir_all(&page_dir).unwrap();

            execute_to_file(&Commands::Tree { depth: None, layer: None }, canvas, &page_dir);

            let tree_path = page_dir.join("tree.txt");
            assert!(tree_path.exists());
        }

        let page_refs: Vec<(&str, &str, &str)> = canvases.iter()
            .map(|c| (node_name(c), "folder", ""))
            .collect();
        let result = generate_index_md(&page_refs, &temp_dir);
        assert!(result.is_ok());

        let index_path = temp_dir.join("index.md");
        assert!(index_path.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_integration_error_handling_invalid_input() {
        let temp_dir = std::env::temp_dir().join("test_integration_error_handling");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        execute_to_file(&Commands::Tree { depth: None, layer: None }, &json!({}), &temp_dir);

        let tree_path = temp_dir.join("tree.txt");
        assert!(tree_path.exists());

        let content = fs::read_to_string(tree_path).unwrap();
        assert!(content.contains("# Node Tree"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_tree_multi_screen_page() {
        // Test that cmd_tree shows all screens when processing a page with multiple frames
        let page = json!({
            "name": "test-page",
            "type": {"__enum__": "NodeType", "value": "CANVAS"},
            "children": [
                {
                    "name": "[screen] profile",
                    "type": {"__enum__": "NodeType", "value": "FRAME"},
                    "size": {"x": 1024.0, "y": 1366.0},
                    "transform": {"m02": 0.0, "m12": 0.0},
                    "children": []
                },
                {
                    "name": "[screen] home",
                    "type": {"__enum__": "NodeType", "value": "FRAME"},
                    "size": {"x": 1024.0, "y": 1366.0},
                    "transform": {"m02": 0.0, "m12": 0.0},
                    "children": []
                }
            ]
        });

        let mut output = Vec::new();
        cmd_tree(&page, None, None, &mut output);
        let result = String::from_utf8(output).unwrap();

        // Both screens should be present in the output
        assert!(result.contains("[screen] profile"), "Missing [screen] profile in output");
        assert!(result.contains("[screen] home"), "Missing [screen] home in output");
        assert!(result.contains("# Node Tree"), "Missing header");
    }

    #[test]
    fn test_exclude_internal_pages() {
        let data = json!({
            "document": {
                "children": [
                    {
                        "name": "demo-1",
                        "type": "CANVAS",
                        "children": [{"name": "Screen", "type": "FRAME"}]
                    },
                    {
                        "name": "Internal Only Canvas",
                        "type": "CANVAS",
                        "children": [{"name": "Brush", "type": "FRAME"}]
                    }
                ]
            }
        });
        
        let pages = get_all_canvases(&data);
        // Apply the same filter as main()
        let pages: Vec<&serde_json::Value> = pages.into_iter()
            .filter(|p| !node_name(p).contains("Internal Only"))
            .collect();
        
        assert_eq!(pages.len(), 1);
        assert_eq!(node_name(pages[0]), "demo-1");
    }
}
