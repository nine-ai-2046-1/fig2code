//! Doctor module for diagnostic analysis of .fig files.
//!
//! Provides functions to analyze .fig file structure and output diagnostic
//! information to help debug parsing issues.

use crate::parser;
use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Run the doctor diagnostic on a .fig file.
///
/// Reads the file, parses it through the full pipeline, collects diagnostic
/// data at each stage, and writes the output to `doctor.log` in the current
/// working directory.
///
/// # Arguments
/// * `input_path` - Path to the .fig file to analyze
/// * `verbose` - Whether to include additional detail in the output
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(...)` if file reading or parsing fails
pub fn run_doctor(input_path: &Path, verbose: bool) -> Result<()> {
    if verbose {
        eprintln!("Reading input file: {}", input_path.display());
    }

    // Read input file
    let raw_bytes = fs::read(input_path)
        .with_context(|| format!("Failed to read input file: {}", input_path.display()))?;
    let raw_file_size = raw_bytes.len();

    if verbose {
        eprintln!("File size: {} bytes", raw_file_size);
    }

    // Extract from ZIP if needed — doctor now handles both raw and ZIP-wrapped .fig files
    let is_zip = parser::is_zip_container(&raw_bytes);
    let bytes = if is_zip {
        parser::extract_from_zip(&raw_bytes)
            .context("Failed to extract canvas.fig from ZIP container")?
    } else {
        raw_bytes
    };

    if verbose {
        eprintln!("Container type: {}", if is_zip { "ZIP" } else { "raw .fig" });
        if is_zip {
            eprintln!("Extracted canvas.fig size: {} bytes", bytes.len());
        }
    }

    // Collect all diagnostic sections
    let file_metadata = collect_file_metadata(&bytes, is_zip, raw_file_size);
    let schema_info = collect_schema_info(&bytes);
    let node_changes_summary = collect_node_changes_summary(&bytes);
    let page_breakdown = collect_page_breakdown(&bytes, verbose);
    let blob_summary = collect_blob_summary(&bytes);
    let tree_stats = collect_tree_stats(&bytes);

    // Format the output
    let output = format_doctor_output(
        &file_metadata,
        &schema_info,
        &node_changes_summary,
        &page_breakdown,
        &blob_summary,
        &tree_stats,
        verbose,
    );

    // Write to doctor.log in current working directory
    fs::write("doctor.log", &output)
        .context("Failed to write doctor.log")?;

    eprintln!("Doctor log written to doctor.log");

    Ok(())
}

/// File metadata collected from the .fig file header.
struct FileMetadata {
    file_size: usize,
    magic_header: String,
    version: u32,
    chunk_count: usize,
    is_zip: bool,
    raw_file_size: usize,
}

/// Collect file metadata from raw bytes.
///
/// Extracts the magic header, version, and chunk count from the .fig file.
/// When the source is a ZIP container, records both the raw ZIP size and
/// the extracted canvas.fig size.
fn collect_file_metadata(bytes: &[u8], is_zip: bool, raw_file_size: usize) -> FileMetadata {
    let file_size = bytes.len();

    // Extract magic header (first 8 bytes)
    let magic_header = if bytes.len() >= 8 {
        String::from_utf8_lossy(&bytes[0..8]).to_string()
    } else {
        format!("{:?}", bytes)
    };

    // Extract version (bytes 8-11, little-endian)
    let version = if bytes.len() >= 12 {
        u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]])
    } else {
        0
    };

    // Count chunks
    let chunk_count = if parser::is_zip_container(bytes) {
        0 // ZIP files don't have chunks in the same way
    } else {
        match parser::extract_chunks(bytes) {
            Ok(parsed) => parsed.chunks.len(),
            Err(_) => 0,
        }
    };

    FileMetadata {
        file_size,
        magic_header,
        version,
        chunk_count,
        is_zip,
        raw_file_size,
    }
}

/// Schema info collected from the Kiwi schema.
struct SchemaInfo {
    definition_count: usize,
    root_message_type: String,
    definition_names: Vec<String>,
}

/// Collect Kiwi schema information by decoding the schema chunk.
///
/// Returns definition count, root message type, and all definition names.
fn collect_schema_info(bytes: &[u8]) -> SchemaInfo {
    // Try to extract and decode schema
    let empty = SchemaInfo {
        definition_count: 0,
        root_message_type: "N/A".to_string(),
        definition_names: vec![],
    };

    if parser::is_zip_container(bytes) {
        return empty;
    }

    let parsed = match parser::extract_chunks(bytes) {
        Ok(p) => p,
        Err(_) => return empty,
    };

    let schema_bytes = match parsed.schema_chunk() {
        Some(s) => match parser::decompress_chunk(s) {
            Ok(b) => b,
            Err(_) => return empty,
        },
        None => return empty,
    };

    // Decode schema using kiwi_schema
    let schema = match kiwi_schema::Schema::decode(&schema_bytes) {
        Ok(s) => s,
        Err(_) => return empty,
    };

    // Find root message type
    let root_message_type = schema
        .defs
        .iter()
        .find(|def| {
            def.name == "Message"
                && def.fields.iter().any(|f| f.name == "nodeChanges")
                && def.fields.iter().any(|f| f.name == "blobs")
        })
        .map(|def| def.name.clone())
        .unwrap_or_else(|| "Not found".to_string());

    let definition_names = schema.defs.iter().map(|def| def.name.clone()).collect();

    SchemaInfo {
        definition_count: schema.defs.len(),
        root_message_type,
        definition_names,
    }
}

/// NodeChanges summary with type distribution.
struct NodeChangesSummary {
    total_count: usize,
    type_distribution: HashMap<String, usize>,
    sample_guids: Vec<String>,
}

/// Collect nodeChanges summary from decoded data.
///
/// Counts total nodes and their type distribution.
fn collect_node_changes_summary(bytes: &[u8]) -> NodeChangesSummary {
    let empty = NodeChangesSummary {
        total_count: 0,
        type_distribution: HashMap::new(),
        sample_guids: vec![],
    };

    if parser::is_zip_container(bytes) {
        return empty;
    }

    let parsed = match parser::extract_chunks(bytes) {
        Ok(p) => p,
        Err(_) => return empty,
    };

    let schema_bytes = match parsed.schema_chunk() {
        Some(s) => match parser::decompress_chunk(s) {
            Ok(b) => b,
            Err(_) => return empty,
        },
        None => return empty,
    };

    let data_bytes = match parsed.data_chunk() {
        Some(s) => match parser::decompress_chunk(s) {
            Ok(b) => b,
            Err(_) => return empty,
        },
        None => return empty,
    };

    let json = match crate::schema::decode_fig_to_json(&schema_bytes, &data_bytes) {
        Ok(j) => j,
        Err(_) => return empty,
    };

    let node_changes = match json.get("nodeChanges").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return empty,
    };

    let mut type_distribution: HashMap<String, usize> = HashMap::new();
    let mut sample_guids = Vec::new();

    for (i, node) in node_changes.iter().enumerate() {
        // Count type distribution
        if let Some(type_val) = node.get("type") {
            let type_name = match type_val {
                JsonValue::Object(obj) => obj
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                JsonValue::String(s) => s.clone(),
                _ => "Unknown".to_string(),
            };
            *type_distribution.entry(type_name).or_insert(0) += 1;
        }

        // Collect sample GUIDs (first 10)
        if i < 10 {
            if let Some(guid_obj) = node.get("guid").and_then(|v| v.as_object()) {
                let session = guid_obj
                    .get("sessionID")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let local = guid_obj
                    .get("localID")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                sample_guids.push(format!("{}:{}", session, local));
            }
        }
    }

    NodeChangesSummary {
        total_count: node_changes.len(),
        type_distribution,
        sample_guids,
    }
}

/// Page breakdown with child counts and orphan detection.
struct PageBreakdown {
    page_count: usize,
    pages: Vec<PageInfo>,
    orphan_count: usize,
}

/// Info about a single page.
struct PageInfo {
    name: String,
    guid: String,
    child_count: usize,
}

/// Collect page-level breakdown from the document tree.
///
/// Identifies pages (direct children of root "0:0") and counts orphans.
fn collect_page_breakdown(bytes: &[u8], verbose: bool) -> PageBreakdown {
    let empty = PageBreakdown {
        page_count: 0,
        pages: vec![],
        orphan_count: 0,
    };

    if parser::is_zip_container(bytes) {
        return empty;
    }

    let parsed = match parser::extract_chunks(bytes) {
        Ok(p) => p,
        Err(_) => return empty,
    };

    let schema_bytes = match parsed.schema_chunk() {
        Some(s) => match parser::decompress_chunk(s) {
            Ok(b) => b,
            Err(_) => return empty,
        },
        None => return empty,
    };

    let data_bytes = match parsed.data_chunk() {
        Some(s) => match parser::decompress_chunk(s) {
            Ok(b) => b,
            Err(_) => return empty,
        },
        None => return empty,
    };

    let json = match crate::schema::decode_fig_to_json(&schema_bytes, &data_bytes) {
        Ok(j) => j,
        Err(_) => return empty,
    };

    let node_changes = match json.get("nodeChanges").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return empty,
    };

    // Build the full node map and parent-child relationships
    // (replicating build_tree logic to analyze structure)
    let mut nodes: HashMap<String, JsonValue> = HashMap::new();
    let mut parent_to_children: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_guids: Vec<String> = Vec::new();
    let mut reachable_guids: Vec<String> = Vec::new();

    for node in node_changes {
        if let Some(guid_str) = format_node_guid(node) {
            nodes.insert(guid_str.clone(), node.clone());
            all_guids.push(guid_str);
        }
    }

    for node in node_changes {
        if let Some(parent_guid) = get_parent_guid(node) {
            if let Some(child_guid) = format_node_guid(node) {
                parent_to_children
                    .entry(parent_guid)
                    .or_default()
                    .push(child_guid);
            }
        }
    }

    // Find pages: children of root "0:0"
    let root_children = parent_to_children.get("0:0").cloned().unwrap_or_default();
    let mut pages = Vec::new();

    for page_guid in &root_children {
        if let Some(page_node) = nodes.get(page_guid) {
            let name = page_node
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed")
                .to_string();
            let child_count = parent_to_children
                .get(page_guid)
                .map(|c| c.len())
                .unwrap_or(0);

            pages.push(PageInfo {
                name,
                guid: page_guid.clone(),
                child_count,
            });

            // Collect reachable nodes under this page
            collect_reachable(page_guid, &parent_to_children, &mut reachable_guids);
        }
    }

    // Count orphans: nodes that are not the root and not reachable
    let orphan_count = all_guids
        .iter()
        .filter(|guid| **guid != "0:0" && !reachable_guids.contains(guid))
        .count();

    if verbose {
        eprintln!("Pages found: {}", pages.len());
        eprintln!("Orphaned nodes: {}", orphan_count);
    }

    PageBreakdown {
        page_count: pages.len(),
        pages,
        orphan_count,
    }
}

/// Collect all GUIDs reachable from a given root GUID.
fn collect_reachable(
    root: &str,
    parent_to_children: &HashMap<String, Vec<String>>,
    reachable: &mut Vec<String>,
) {
    if let Some(children) = parent_to_children.get(root) {
        for child in children {
            reachable.push(child.clone());
            collect_reachable(child, parent_to_children, reachable);
        }
    }
}

/// Format a node's GUID as "sessionID:localID".
fn format_node_guid(node: &JsonValue) -> Option<String> {
    let guid_obj = node.get("guid").and_then(|v| v.as_object())?;
    let session = guid_obj.get("sessionID").and_then(|v| v.as_u64())?;
    let local = guid_obj.get("localID").and_then(|v| v.as_u64())?;
    Some(format!("{}:{}", session, local))
}

/// Get the parent GUID from a node's parentIndex field.
fn get_parent_guid(node: &JsonValue) -> Option<String> {
    let parent_index = node.get("parentIndex")?;
    let guid_obj = parent_index.get("guid").and_then(|v| v.as_object())?;
    let session = guid_obj.get("sessionID").and_then(|v| v.as_u64())?;
    let local = guid_obj.get("localID").and_then(|v| v.as_u64())?;
    Some(format!("{}:{}", session, local))
}

/// Blob summary with type distribution.
struct BlobSummary {
    total_count: usize,
    type_distribution: HashMap<String, usize>,
    sample_blobs: Vec<String>,
}

/// Collect blob summary from decoded data.
///
/// Counts blobs and their type distribution.
fn collect_blob_summary(bytes: &[u8]) -> BlobSummary {
    let empty = BlobSummary {
        total_count: 0,
        type_distribution: HashMap::new(),
        sample_blobs: vec![],
    };

    if parser::is_zip_container(bytes) {
        return empty;
    }

    let parsed = match parser::extract_chunks(bytes) {
        Ok(p) => p,
        Err(_) => return empty,
    };

    let schema_bytes = match parsed.schema_chunk() {
        Some(s) => match parser::decompress_chunk(s) {
            Ok(b) => b,
            Err(_) => return empty,
        },
        None => return empty,
    };

    let data_bytes = match parsed.data_chunk() {
        Some(s) => match parser::decompress_chunk(s) {
            Ok(b) => b,
            Err(_) => return empty,
        },
        None => return empty,
    };

    let json = match crate::schema::decode_fig_to_json(&schema_bytes, &data_bytes) {
        Ok(j) => j,
        Err(_) => return empty,
    };

    let blobs = match json.get("blobs").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return empty,
    };

    let mut type_distribution: HashMap<String, usize> = HashMap::new();
    let mut sample_blobs = Vec::new();

    for (i, blob) in blobs.iter().enumerate() {
        // Count type distribution
        if let Some(type_val) = blob.get("type") {
            let type_name = match type_val {
                JsonValue::Object(obj) => obj
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                JsonValue::String(s) => s.clone(),
                _ => "Unknown".to_string(),
            };
            *type_distribution.entry(type_name).or_insert(0) += 1;
        }

        // Collect sample blobs (first 5)
        if i < 5 {
            let summary = if let Some(bytes_val) = blob.get("bytes") {
                match bytes_val {
                    JsonValue::Array(arr) => format!("bytes[{}]", arr.len()),
                    JsonValue::String(s) => format!("bytes(base64)[{}]", s.len()),
                    _ => "bytes(unknown)".to_string(),
                }
            } else {
                "no bytes".to_string()
            };
            sample_blobs.push(summary);
        }
    }

    BlobSummary {
        total_count: blobs.len(),
        type_distribution,
        sample_blobs,
    }
}

/// Tree structure statistics.
struct TreeStats {
    max_depth: usize,
    total_reachable: usize,
    nodes_per_page: Vec<(String, usize)>,
}

/// Collect tree structure statistics.
///
/// Computes max depth, total reachable nodes, and nodes per page.
fn collect_tree_stats(bytes: &[u8]) -> TreeStats {
    let empty = TreeStats {
        max_depth: 0,
        total_reachable: 0,
        nodes_per_page: vec![],
    };

    if parser::is_zip_container(bytes) {
        return empty;
    }

    let parsed = match parser::extract_chunks(bytes) {
        Ok(p) => p,
        Err(_) => return empty,
    };

    let schema_bytes = match parsed.schema_chunk() {
        Some(s) => match parser::decompress_chunk(s) {
            Ok(b) => b,
            Err(_) => return empty,
        },
        None => return empty,
    };

    let data_bytes = match parsed.data_chunk() {
        Some(s) => match parser::decompress_chunk(s) {
            Ok(b) => b,
            Err(_) => return empty,
        },
        None => return empty,
    };

    let json = match crate::schema::decode_fig_to_json(&schema_bytes, &data_bytes) {
        Ok(j) => j,
        Err(_) => return empty,
    };

    let node_changes = match json.get("nodeChanges").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return empty,
    };

    // Build parent-child map
    let mut parent_to_children: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_guids: Vec<String> = Vec::new();

    for node in node_changes {
        if let Some(guid_str) = format_node_guid(node) {
            all_guids.push(guid_str.clone());
            if let Some(parent_guid) = get_parent_guid(node) {
                parent_to_children
                    .entry(parent_guid)
                    .or_default()
                    .push(guid_str);
            }
        }
    }

    // Find pages (children of "0:0")
    let root_children = parent_to_children.get("0:0").cloned().unwrap_or_default();
    let mut nodes_per_page = Vec::new();
    let mut max_depth = 0;
    let mut total_reachable = 0;

    for page_guid in &root_children {
        let mut page_nodes = Vec::new();
        let depth = compute_subtree_stats(
            page_guid,
            &parent_to_children,
            &mut page_nodes,
            1,
        );
        max_depth = max_depth.max(depth);
        total_reachable += page_nodes.len();

        // Get page name (if available in nodes)
        let page_name = page_guid.clone(); // Fallback to GUID
        nodes_per_page.push((page_name, page_nodes.len()));
    }

    // Add root itself
    total_reachable += 1; // for "0:0"

    TreeStats {
        max_depth,
        total_reachable,
        nodes_per_page,
    }
}

/// Compute subtree statistics recursively.
///
/// Returns the maximum depth and collects all reachable node GUIDs.
fn compute_subtree_stats(
    node: &str,
    parent_to_children: &HashMap<String, Vec<String>>,
    reachable: &mut Vec<String>,
    current_depth: usize,
) -> usize {
    let mut max_depth = current_depth;

    if let Some(children) = parent_to_children.get(node) {
        for child in children {
            reachable.push(child.clone());
            let child_depth = compute_subtree_stats(child, parent_to_children, reachable, current_depth + 1);
            max_depth = max_depth.max(child_depth);
        }
    }

    max_depth
}

/// Format all diagnostic sections into a human-readable string.
fn format_doctor_output(
    file_metadata: &FileMetadata,
    schema_info: &SchemaInfo,
    node_changes_summary: &NodeChangesSummary,
    page_breakdown: &PageBreakdown,
    blob_summary: &BlobSummary,
    tree_stats: &TreeStats,
    verbose: bool,
) -> String {
    let mut output = String::new();

    // File Metadata section
    output.push_str("=== File Metadata ===\n");
    if file_metadata.is_zip {
        output.push_str("Container type: ZIP\n");
        output.push_str(&format!("ZIP file size: {} bytes\n", file_metadata.raw_file_size));
        output.push_str(&format!("Extracted canvas.fig size: {} bytes\n", file_metadata.file_size));
    } else {
        output.push_str("Container type: raw .fig\n");
        output.push_str(&format!("File size: {} bytes\n", file_metadata.file_size));
    }
    output.push_str(&format!("Magic header: {}\n", file_metadata.magic_header));
    output.push_str(&format!("Version: {}\n", file_metadata.version));
    output.push_str(&format!("Chunk count: {}\n", file_metadata.chunk_count));
    output.push('\n');

    // Kiwi Schema section
    output.push_str("=== Kiwi Schema ===\n");
    output.push_str(&format!("Definition count: {}\n", schema_info.definition_count));
    output.push_str(&format!("Root message type: {}\n", schema_info.root_message_type));
    if verbose && !schema_info.definition_names.is_empty() {
        output.push_str("Definitions:\n");
        for name in &schema_info.definition_names {
            output.push_str(&format!("  - {}\n", name));
        }
    }
    output.push('\n');

    // NodeChanges section
    output.push_str("=== NodeChanges ===\n");
    output.push_str(&format!("Total nodes: {}\n", node_changes_summary.total_count));
    output.push_str("Type distribution:\n");
    let mut sorted_types: Vec<_> = node_changes_summary.type_distribution.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1));
    for (type_name, count) in &sorted_types {
        output.push_str(&format!("  {}: {}\n", type_name, count));
    }
    if verbose && !node_changes_summary.sample_guids.is_empty() {
        output.push_str("Sample GUIDs (first 10):\n");
        for guid in &node_changes_summary.sample_guids {
            output.push_str(&format!("  {}\n", guid));
        }
    }
    output.push('\n');

    // Pages section
    output.push_str("=== Pages ===\n");
    output.push_str(&format!("Page count: {}\n", page_breakdown.page_count));
    output.push_str(&format!("Orphaned nodes: {}\n", page_breakdown.orphan_count));
    for page in &page_breakdown.pages {
        output.push_str(&format!(
            "  {} ({}) — {} children\n",
            page.name, page.guid, page.child_count
        ));
    }
    output.push('\n');

    // Blobs section
    output.push_str("=== Blobs ===\n");
    output.push_str(&format!("Total blobs: {}\n", blob_summary.total_count));
    if !blob_summary.type_distribution.is_empty() {
        output.push_str("Type distribution:\n");
        for (type_name, count) in &blob_summary.type_distribution {
            output.push_str(&format!("  {}: {}\n", type_name, count));
        }
    }
    if verbose && !blob_summary.sample_blobs.is_empty() {
        output.push_str("Sample blobs (first 5):\n");
        for blob in &blob_summary.sample_blobs {
            output.push_str(&format!("  {}\n", blob));
        }
    }
    output.push('\n');

    // Tree Stats section
    output.push_str("=== Tree Stats ===\n");
    output.push_str(&format!("Max depth: {}\n", tree_stats.max_depth));
    output.push_str(&format!("Total reachable nodes: {}\n", tree_stats.total_reachable));
    if !tree_stats.nodes_per_page.is_empty() {
        output.push_str("Nodes per page:\n");
        for (page_name, count) in &tree_stats.nodes_per_page {
            output.push_str(&format!("  {}: {}\n", page_name, count));
        }
    }
    output.push('\n');

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn test_format_node_guid() {
        let node = json!({
            "guid": {
                "sessionID": 1,
                "localID": 42
            }
        });
        assert_eq!(format_node_guid(&node), Some("1:42".to_string()));
    }

    #[test]
    fn test_format_node_guid_missing_guid() {
        let node = json!({"name": "test"});
        assert_eq!(format_node_guid(&node), None);
    }

    #[test]
    fn test_format_node_guid_missing_fields() {
        let node = json!({"guid": {}});
        assert_eq!(format_node_guid(&node), None);
    }

    #[test]
    fn test_get_parent_guid() {
        let node = json!({
            "parentIndex": {
                "guid": {
                    "sessionID": 0,
                    "localID": 1
                },
                "position": "a"
            }
        });
        assert_eq!(get_parent_guid(&node), Some("0:1".to_string()));
    }

    #[test]
    fn test_get_parent_guid_no_parent() {
        let node = json!({"name": "root"});
        assert_eq!(get_parent_guid(&node), None);
    }

    #[test]
    fn test_compute_subtree_stats_empty() {
        let parent_to_children: HashMap<String, Vec<String>> = HashMap::new();
        let mut reachable = Vec::new();
        let depth = compute_subtree_stats("0:0", &parent_to_children, &mut reachable, 1);
        assert_eq!(depth, 1);
        assert!(reachable.is_empty());
    }

    #[test]
    fn test_compute_subtree_stats_with_children() {
        let mut parent_to_children: HashMap<String, Vec<String>> = HashMap::new();
        parent_to_children.insert("0:0".to_string(), vec!["0:1".to_string(), "0:2".to_string()]);
        parent_to_children.insert("0:1".to_string(), vec!["0:3".to_string()]);

        let mut reachable = Vec::new();
        let depth = compute_subtree_stats("0:0", &parent_to_children, &mut reachable, 1);

        assert_eq!(depth, 3); // 0:0(depth=1) -> 0:1(depth=2) -> 0:3(depth=3)
        assert_eq!(reachable.len(), 3); // 0:1, 0:2, 0:3
        assert!(reachable.contains(&"0:1".to_string()));
        assert!(reachable.contains(&"0:2".to_string()));
        assert!(reachable.contains(&"0:3".to_string()));
    }

    #[test]
    fn test_collect_reachable() {
        let mut parent_to_children: HashMap<String, Vec<String>> = HashMap::new();
        parent_to_children.insert("root".to_string(), vec!["a".to_string(), "b".to_string()]);
        parent_to_children.insert("a".to_string(), vec!["c".to_string()]);

        let mut reachable = Vec::new();
        collect_reachable("root", &parent_to_children, &mut reachable);

        assert_eq!(reachable.len(), 3);
        assert!(reachable.contains(&"a".to_string()));
        assert!(reachable.contains(&"b".to_string()));
        assert!(reachable.contains(&"c".to_string()));
    }

    #[test]
    fn test_format_doctor_output() {
        let metadata = FileMetadata {
            file_size: 1024,
            magic_header: "fig-kiwi".to_string(),
            version: 48,
            chunk_count: 3,
            is_zip: false,
            raw_file_size: 1024,
        };
        let schema = SchemaInfo {
            definition_count: 10,
            root_message_type: "Message".to_string(),
            definition_names: vec!["Message".to_string(), "Node".to_string()],
        };
        let node_changes = NodeChangesSummary {
            total_count: 50,
            type_distribution: {
                let mut m = HashMap::new();
                m.insert("FRAME".to_string(), 10);
                m.insert("TEXT".to_string(), 20);
                m
            },
            sample_guids: vec!["0:1".to_string(), "0:2".to_string()],
        };
        let pages = PageBreakdown {
            page_count: 2,
            pages: vec![
                PageInfo {
                    name: "Page 1".to_string(),
                    guid: "0:1".to_string(),
                    child_count: 5,
                },
                PageInfo {
                    name: "Page 2".to_string(),
                    guid: "0:2".to_string(),
                    child_count: 3,
                },
            ],
            orphan_count: 0,
        };
        let blobs = BlobSummary {
            total_count: 5,
            type_distribution: {
                let mut m = HashMap::new();
                m.insert("IMAGE".to_string(), 3);
                m
            },
            sample_blobs: vec!["bytes[100]".to_string()],
        };
        let stats = TreeStats {
            max_depth: 4,
            total_reachable: 51,
            nodes_per_page: vec![("0:1".to_string(), 25), ("0:2".to_string(), 25)],
        };

        let output = format_doctor_output(&metadata, &schema, &node_changes, &pages, &blobs, &stats, false);

        assert!(output.contains("=== File Metadata ==="));
        assert!(output.contains("File size: 1024 bytes"));
        assert!(output.contains("Magic header: fig-kiwi"));
        assert!(output.contains("Version: 48"));
        assert!(output.contains("=== Kiwi Schema ==="));
        assert!(output.contains("Definition count: 10"));
        assert!(output.contains("=== NodeChanges ==="));
        assert!(output.contains("Total nodes: 50"));
        assert!(output.contains("=== Pages ==="));
        assert!(output.contains("Page count: 2"));
        assert!(output.contains("Orphaned nodes: 0"));
        assert!(output.contains("Page 1"));
        assert!(output.contains("=== Blobs ==="));
        assert!(output.contains("Total blobs: 5"));
        assert!(output.contains("=== Tree Stats ==="));
        assert!(output.contains("Max depth: 4"));
        assert!(output.contains("Total reachable nodes: 51"));
    }

    #[test]
    fn test_format_doctor_output_verbose() {
        let metadata = FileMetadata {
            file_size: 100,
            magic_header: "fig-kiwi".to_string(),
            version: 48,
            chunk_count: 2,
            is_zip: false,
            raw_file_size: 100,
        };
        let schema = SchemaInfo {
            definition_count: 5,
            root_message_type: "Message".to_string(),
            definition_names: vec!["Message".to_string(), "Node".to_string()],
        };
        let node_changes = NodeChangesSummary {
            total_count: 10,
            type_distribution: HashMap::new(),
            sample_guids: vec!["0:1".to_string()],
        };
        let pages = PageBreakdown {
            page_count: 0,
            pages: vec![],
            orphan_count: 0,
        };
        let blobs = BlobSummary {
            total_count: 2,
            type_distribution: HashMap::new(),
            sample_blobs: vec!["bytes[50]".to_string()],
        };
        let stats = TreeStats {
            max_depth: 1,
            total_reachable: 1,
            nodes_per_page: vec![],
        };

        let output = format_doctor_output(&metadata, &schema, &node_changes, &pages, &blobs, &stats, true);

        // Verbose should include definitions, sample GUIDs, sample blobs
        assert!(output.contains("Definitions:"));
        assert!(output.contains("  - Message"));
        assert!(output.contains("Sample GUIDs (first 10):"));
        assert!(output.contains("  0:1"));
        assert!(output.contains("Sample blobs (first 5):"));
        assert!(output.contains("  bytes[50]"));
    }

    #[test]
    fn test_collect_file_metadata_invalid() {
        let bytes = b"invalid";
        let metadata = collect_file_metadata(bytes, false, 7);
        assert_eq!(metadata.file_size, 7);
        assert_eq!(metadata.version, 0);
        assert_eq!(metadata.chunk_count, 0);
        assert!(!metadata.is_zip);
    }

    #[test]
    fn test_collect_schema_info_invalid() {
        let bytes = b"invalid";
        let info = collect_schema_info(bytes);
        assert_eq!(info.definition_count, 0);
        assert_eq!(info.root_message_type, "N/A");
    }

    #[test]
    fn test_collect_node_changes_summary_invalid() {
        let bytes = b"invalid";
        let summary = collect_node_changes_summary(bytes);
        assert_eq!(summary.total_count, 0);
        assert!(summary.type_distribution.is_empty());
    }

    #[test]
    fn test_collect_page_breakdown_invalid() {
        let bytes = b"invalid";
        let breakdown = collect_page_breakdown(bytes, false);
        assert_eq!(breakdown.page_count, 0);
        assert_eq!(breakdown.orphan_count, 0);
    }

    #[test]
    fn test_collect_blob_summary_invalid() {
        let bytes = b"invalid";
        let summary = collect_blob_summary(bytes);
        assert_eq!(summary.total_count, 0);
    }

    #[test]
    fn test_collect_tree_stats_invalid() {
        let bytes = b"invalid";
        let stats = collect_tree_stats(bytes);
        assert_eq!(stats.max_depth, 0);
        assert_eq!(stats.total_reachable, 0);
    }

    #[test]
    fn test_format_doctor_output_zip_container() {
        let metadata = FileMetadata {
            file_size: 2048,
            magic_header: "fig-kiwi".to_string(),
            version: 48,
            chunk_count: 3,
            is_zip: true,
            raw_file_size: 5120,
        };
        let schema = SchemaInfo {
            definition_count: 10,
            root_message_type: "Message".to_string(),
            definition_names: vec![],
        };
        let node_changes = NodeChangesSummary {
            total_count: 50,
            type_distribution: HashMap::new(),
            sample_guids: vec![],
        };
        let pages = PageBreakdown {
            page_count: 2,
            pages: vec![],
            orphan_count: 0,
        };
        let blobs = BlobSummary {
            total_count: 5,
            type_distribution: HashMap::new(),
            sample_blobs: vec![],
        };
        let stats = TreeStats {
            max_depth: 4,
            total_reachable: 51,
            nodes_per_page: vec![],
        };

        let output = format_doctor_output(&metadata, &schema, &node_changes, &pages, &blobs, &stats, false);

        assert!(output.contains("Container type: ZIP"));
        assert!(output.contains("ZIP file size: 5120 bytes"));
        assert!(output.contains("Extracted canvas.fig size: 2048 bytes"));
    }

    /// Helper: create an in-memory ZIP with a single `canvas.fig` entry.
    fn create_mock_zip_with_canvas(canvas_content: &[u8]) -> Vec<u8> {
        use std::io::Write;
        use zip::write::FileOptions;
        use zip::ZipWriter;

        let mut buf = Vec::new();
        {
            let mut writer = ZipWriter::new(Cursor::new(&mut buf));
            let options = FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("canvas.fig", options).unwrap();
            writer.write_all(canvas_content).unwrap();
            writer.finish().unwrap();
        }
        buf
    }

    /// Helper: create an in-memory ZIP without a `canvas.fig` entry.
    fn create_mock_zip_without_canvas() -> Vec<u8> {
        use std::io::Write;
        use zip::write::FileOptions;
        use zip::ZipWriter;

        let mut buf = Vec::new();
        {
            let mut writer = ZipWriter::new(Cursor::new(&mut buf));
            let options = FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("other_file.txt", options).unwrap();
            writer.write_all(b"not a fig file").unwrap();
            writer.finish().unwrap();
        }
        buf
    }

    #[test]
    fn test_collect_file_metadata_from_zip() {
        let canvas_content = b"fig-kiwi\x30\x00\x00\x00fake fig data";
        let zip_bytes = create_mock_zip_with_canvas(canvas_content);

        // Extract from ZIP (simulating what run_doctor now does)
        let extracted = parser::extract_from_zip(&zip_bytes).unwrap();
        let metadata = collect_file_metadata(&extracted, true, zip_bytes.len());

        assert!(metadata.is_zip);
        assert_eq!(metadata.raw_file_size, zip_bytes.len());
        assert_eq!(metadata.file_size, canvas_content.len());
    }

    #[test]
    fn test_collect_all_sections_from_zip() {
        // Create a minimal valid .fig header for the extracted content
        let canvas_content = b"fig-kiwi\x30\x00\x00\x00minimal";
        let zip_bytes = create_mock_zip_with_canvas(canvas_content);
        let extracted = parser::extract_from_zip(&zip_bytes).unwrap();

        // All collect_* functions should work on the extracted bytes
        let metadata = collect_file_metadata(&extracted, true, zip_bytes.len());
        assert!(metadata.is_zip);

        let schema = collect_schema_info(&extracted);
        // Minimal content won't have a valid schema, but shouldn't panic
        assert_eq!(schema.definition_count, 0);

        let node_changes = collect_node_changes_summary(&extracted);
        assert_eq!(node_changes.total_count, 0);

        let pages = collect_page_breakdown(&extracted, false);
        assert_eq!(pages.page_count, 0);

        let blobs = collect_blob_summary(&extracted);
        assert_eq!(blobs.total_count, 0);

        let stats = collect_tree_stats(&extracted);
        assert_eq!(stats.max_depth, 0);
    }

    #[test]
    fn test_zip_without_canvas_fig_returns_error() {
        let zip_bytes = create_mock_zip_without_canvas();
        let result = parser::extract_from_zip(&zip_bytes);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("canvas.fig") || err_msg.contains("not found"));
    }
}
