use clap::{Parser, Subcommand};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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

fn cmd_layers(data: &Value) {
    println!("# Layers (top-level frames on canvas)\n");
    println!("{:<30} {:>12} {:>20} {}", "Name", "Size (w×h)", "Position (x,y)", "Fill");
    println!("{}", "─".repeat(80));
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
            println!("{:<30} {:>5}×{:<5} {:>9},{:<9} {}", name, w as i32, h as i32, x as i32, y as i32, fill);
        }
    }
}

fn print_tree(node: &Value, indent: usize, max_depth: Option<usize>) {
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

    println!(
        "{}{} [{}] {}×{} @({},{}){}{}{}",
        "  ".repeat(depth),
        name,
        ntype,
        w as i32, h as i32,
        x as i32, y as i32,
        if fill_summary.is_empty() { String::new() } else { format!(" {}", fill_summary) },
        cr,
        interaction_marker
    );

    for child in children(node) {
        print_tree(child, indent + 2, max_depth);
    }
}

fn cmd_tree(data: &Value, depth: Option<usize>, layer: Option<&str>) {
    println!("# Node Tree\n");
    let root: &Value = if let Some(lname) = layer {
        match find_layer(data, lname) {
            Some(l) => l,
            None => {
                eprintln!("Layer '{}' not found. Use `layers` to list available layers.", lname);
                return;
            }
        }
    } else {
        get_canvas(data).unwrap_or_else(|| get_document(data))
    };
    print_tree(root, 0, depth);
}

fn print_fills(fills: &Value, prefix: &str) {
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
            println!("{}  [{}] type={} color={} visible={}{}", prefix, i, ftype, hex, vis, img_filename);
        }
    }
}

fn cmd_node(data: &Value, name: &str, layer: Option<&str>) {
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

    println!("# Node: {}\n", name);
    println!("type:         {}", ntype);
    println!("size:         {}×{}", w as i32, h as i32);
    println!("position:     ({}, {})", x as i32, y as i32);
    if let Some(r) = cr { println!("cornerRadius: {}px", r); }
    if let Some(o) = opacity { println!("opacity:      {}", o); }
    if !visible { println!("visible:      false  ← HIDDEN NODE"); }
    if is_mask  { println!("mask:         true  ← THIS NODE MASKS SIBLINGS"); }
    if let Some(w) = sw { println!("strokeWeight: {}px", w); }

    // Stroke align
    if let Some(sa) = node.get("strokeAlign") {
        let val = if let Some(obj) = sa.as_object() {
            obj.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string()
        } else {
            sa.as_str().unwrap_or("").to_string()
        };
        if !val.is_empty() { println!("strokeAlign:  {}", val); }
    }

    // Fills
    if let Some(fills) = node.get("fillPaints") {
        if fills.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
            println!("\nfillPaints:");
            print_fills(fills, "");
        }
    }

    // Strokes
    if let Some(strokes) = node.get("strokePaints") {
        if strokes.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
            println!("\nstrokePaints:");
            print_fills(strokes, "");
        }
    }

    // Effects
    if let Some(effects) = node.get("effects").and_then(|e| e.as_array()) {
        if !effects.is_empty() {
            println!("\neffects:");
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
                println!("  [{}] type={} color={} offset=({},{}) radius={} spread={} visible={}", i, etype, hex, ox, oy, radius, spread, vis);
            }
        }
    }

    // Auto-layout
    if let Some(sm) = node.get("stackMode") {
        let val = if let Some(obj) = sm.as_object() {
            obj.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string()
        } else { sm.as_str().unwrap_or("").to_string() };
        if !val.is_empty() {
            println!("\nauto-layout:");
            println!("  direction:   {}", val);
            if let Some(gap) = node.get("stackSpacing").and_then(|v| v.as_f64()) {
                println!("  gap:         {}px", gap);
            }
            if let Some(ph) = node.get("stackHorizontalPadding").and_then(|v| v.as_f64()) {
                println!("  padH:        {}px", ph);
            }
            if let Some(pv) = node.get("stackVerticalPadding").and_then(|v| v.as_f64()) {
                println!("  padV:        {}px", pv);
            }
            if let Some(pr) = node.get("stackPaddingRight").and_then(|v| v.as_f64()) {
                println!("  padRight:    {}px", pr);
            }
            if let Some(pb) = node.get("stackPaddingBottom").and_then(|v| v.as_f64()) {
                println!("  padBottom:   {}px", pb);
            }
            let primary_align = node.get("stackPrimaryAlignItems").and_then(|a| {
                if let Some(obj) = a.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                else { a.as_str().map(|s| s.to_string()) }
            });
            let counter_align = node.get("stackCounterAlignItems").and_then(|a| {
                if let Some(obj) = a.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                else { a.as_str().map(|s| s.to_string()) }
            });
            if let Some(pa) = primary_align { println!("  primaryAlign: {} (justify-content)", pa); }
            if let Some(ca) = counter_align { println!("  counterAlign: {} (align-items)", ca); }
            if let Some(grow) = node.get("stackChildPrimaryGrow").and_then(|v| v.as_f64()) {
                if grow > 0.0 { println!("  childGrow:   {} (flex: {})", grow, grow as i32); }
            }
        }
    }

    // Text
    if ntype == "TEXT" {
        if let Some(td) = node.get("textData") {
            let chars = td.get("characters").and_then(|c| c.as_str()).unwrap_or("");
            println!("\ntext content: {:?}", chars);
        }
        let fname = node.get("fontName").unwrap_or(&Value::Null);
        let family = fname.get("family").and_then(|v| v.as_str()).unwrap_or("—");
        let style  = fname.get("style").and_then(|v| v.as_str()).unwrap_or("—");
        let fsize  = node.get("fontSize").and_then(|v| v.as_f64()).unwrap_or(0.0);
        println!("font:         {} {} {}px", family, style, fsize);

        let lh = node.get("lineHeight").unwrap_or(&Value::Null);
        let lh_val = lh.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lh_unit = lh.get("units").and_then(|u| {
            if let Some(obj) = u.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
            else { u.as_str().map(|s| s.to_string()) }
        }).unwrap_or_default();
        println!("lineHeight:   {} {}", lh_val, lh_unit);

        let ls = node.get("letterSpacing").unwrap_or(&Value::Null);
        let ls_val = ls.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ls_unit = ls.get("units").and_then(|u| {
            if let Some(obj) = u.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
            else { u.as_str().map(|s| s.to_string()) }
        }).unwrap_or_default();
        println!("letterSpacing:{} {}", ls_val, ls_unit);

        let align_h = node.get("textAlignHorizontal").and_then(|a| {
            if let Some(obj) = a.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
            else { a.as_str().map(|s| s.to_string()) }
        });
        let align_v = node.get("textAlignVertical").and_then(|a| {
            if let Some(obj) = a.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
            else { a.as_str().map(|s| s.to_string()) }
        });
        if let Some(ah) = align_h { println!("alignH:       {}", ah); }
        if let Some(av) = align_v { println!("alignV:       {}", av); }
    }

    // Prototype interactions
    if let Some(interactions) = node.get("prototypeInteractions").and_then(|p| p.as_array()) {
        if !interactions.is_empty() {
            let guid_map = build_guid_map(get_document(data));
            println!("\nprototypeInteractions: {} found", interactions.len());
            for (i, interaction) in interactions.iter().enumerate() {
                let event_type = interaction.get("event")
                    .and_then(|e| e.get("interactionType"))
                    .and_then(|t| {
                        if let Some(obj) = t.as_object() { obj.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()) }
                        else { t.as_str().map(|s| s.to_string()) }
                    }).unwrap_or_default();
                println!("  [{}] trigger: {}", i, event_type);
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
                        print!("      connection={} nav={} dest={}", conn_type, nav_type, dest);
                        if !url.is_empty() { print!(" url={}", url); }
                        if !trans_type.is_empty() { print!(" transition={}", trans_type); }
                        if let Some(d) = duration { print!(" duration={}ms", (d * 1000.0) as i32); }
                        if let Some(e) = easing { print!(" easing={}", e); }
                        println!();
                    }
                }
            }
        }
    }
}

fn cmd_texts(data: &Value, layer: Option<&str>) {
    let root: &Value = if let Some(lname) = layer {
        match find_layer(data, lname) {
            Some(l) => l,
            None => { eprintln!("Layer '{}' not found.", lname); return; }
        }
    } else {
        get_document(data)
    };

    println!("# Text Nodes\n");
    println!("{:<40} {:>8} {:>15} {:>8}  {}", "Node path", "Size", "Font", "Color", "Content");
    println!("{}", "─".repeat(120));

    let mut _path_stack: Vec<String> = Vec::new();

    fn walk_texts(node: &Value, path: &str) {
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
            println!(
                "{:<40} {:>4}×{:<4} {:>12}/{:<3} {:>8}px  {}",
                if current_path.len() > 40 { &current_path[current_path.len()-40..] } else { &current_path },
                w as i32, h as i32,
                family, style,
                fsize as i32,
                color,
            );
            println!("{:<40}   content: {:?}", "", preview);
        }

        for child in children(node) {
            walk_texts(child, &current_path);
        }
    }

    walk_texts(root, "");
    let _ = _path_stack.len();
}

fn cmd_images(data: &Value, layer: Option<&str>, images_dir: Option<&PathBuf>) {
    let root: &Value = if let Some(lname) = layer {
        match find_layer(data, lname) {
            Some(l) => l,
            None => { eprintln!("Layer '{}' not found.", lname); return; }
        }
    } else {
        get_document(data)
    };

    println!("# Image Fills\n");

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

        println!("File: {} [{}]", filename, exists);
        // Deduplicate usages by path
        let mut seen_paths: Vec<&str> = Vec::new();
        for (path, w, h, x, y) in usages {
            if !seen_paths.contains(&path.as_str()) {
                seen_paths.push(path.as_str());
                // Show last 2 path components for readability
                let short_path: String = path.split('/').rev().take(3).collect::<Vec<_>>().iter().rev().cloned().collect::<Vec<_>>().join("/");
                println!("  used in: {} (node: {}×{} @({},{}))", short_path, w, h, x, y);
            }
        }
        println!();
    }

    println!("Total unique image hashes: {}", entries.len());
}

fn cmd_interactions(data: &Value, layer: Option<&str>) {
    let root: &Value = if let Some(lname) = layer {
        match find_layer(data, lname) {
            Some(l) => l,
            None => { eprintln!("Layer '{}' not found.", lname); return; }
        }
    } else {
        get_document(data)
    };

    let guid_map = build_guid_map(get_document(data));

    println!("# Prototype Interactions\n");
    println!("{:<3} {:<30} {:<15} {:<12} {:<12} {:<30} {:<15} {:>8}", "#", "Trigger node", "Parent layer", "Event", "Action", "Destination", "Transition", "Duration");
    println!("{}", "─".repeat(140));

    let mut count = 0;

    fn walk_interactions(node: &Value, path: &str, guid_map: &HashMap<String, String>, count: &mut i32) {
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
                        println!(
                            "{:<3} {:<30} {:<15} {:<12} {:<12} {:<30} {:<15} {:>8}",
                            count,
                            if name.len() > 30 { &name[..30] } else { name },
                            if parent.len() > 15 { &parent[..15] } else { &parent },
                            event_type,
                            format!("{}/{}", conn_type, nav_type),
                            if dest.len() > 30 { &dest[..30] } else { &dest },
                            trans_type,
                            duration
                        );
                        if !conn_type.contains("NONE") {
                            // Show full destination if truncated
                            if dest.len() > 30 {
                                println!("    dest (full): {}", dest);
                            }
                        }
                    }
                }
            }
        }

        for child in children(node) {
            walk_interactions(child, &current_path, guid_map, count);
        }
    }

    walk_interactions(root, "", &guid_map, &mut count);
    println!("\nTotal interactions: {}", count);
}

fn cmd_tokens(data: &Value, layer: Option<&str>) {
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

    println!("# Design Tokens\n");

    println!("## Colours ({} unique)\n", colors.len());
    for (hex, usage) in &colors {
        println!("  {}  (first seen: {})", hex, usage);
    }

    println!("\n## Typography ({} unique combinations)\n", fonts.len());
    fonts.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    for (family, style, size, usage) in &fonts {
        println!("  {} {} {}px  (e.g. {})", family, style, size, usage);
    }

    println!("\n## Corner Radii\n");
    radii.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    for (r, usage) in &radii {
        println!("  {}px  (e.g. {})", r, usage);
    }

    println!("\n## Auto-layout Gaps\n");
    for (g, usage) in &gaps {
        println!("  {}px  (e.g. {})", g, usage);
    }
}

fn cmd_raw(data: &Value, name: &str, path: Option<&str>) {
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

    println!("{}", serde_json::to_string_pretty(target).unwrap_or_default());
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    let content = match fs::read_to_string(&cli.input) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading {:?}: {}", cli.input, e);
            std::process::exit(1);
        }
    };

    let data: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing JSON: {}", e);
            std::process::exit(1);
        }
    };

    match &cli.command {
        Commands::Layers => cmd_layers(&data),
        Commands::Tree { depth, layer } => cmd_tree(&data, *depth, layer.as_deref()),
        Commands::Node { name, layer } => cmd_node(&data, name, layer.as_deref()),
        Commands::Texts { layer } => cmd_texts(&data, layer.as_deref()),
        Commands::Images { layer, images_dir } => cmd_images(&data, layer.as_deref(), images_dir.as_ref()),
        Commands::Interactions { layer } => cmd_interactions(&data, layer.as_deref()),
        Commands::Tokens { layer } => cmd_tokens(&data, layer.as_deref()),
        Commands::Raw { name, path } => cmd_raw(&data, name, path.as_deref()),
    }
}
