use crate::error::{FigError, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Build a tree structure from flat nodeChanges array
///
/// Takes the flat array of nodes and builds a hierarchical tree structure
/// by creating parent-child relationships based on parentIndex fields.
///
/// # Arguments
/// * `node_changes` - Array of node objects from decoded Kiwi data
///
/// # Returns
/// * `Ok(JsonValue)` - Root node with children hierarchy
/// * `Err(FigError)` - If tree building fails
///
/// # Examples
/// ```no_run
/// use fig2json::schema::build_tree;
/// use serde_json::json;
///
/// let node_changes = vec![/* node objects */];
/// let root = build_tree(node_changes).unwrap();
/// ```
pub fn build_tree(node_changes: Vec<JsonValue>) -> Result<JsonValue> {
    let debug = std::env::var("FIG2JSON_DEBUG").ok().as_deref() == Some("1");

    // 1. Create map: GUID -> Node and map of parent -> children (position, GUID) tuples
    let mut nodes: HashMap<String, JsonValue> = HashMap::new();
    let mut parent_to_children: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for node in &node_changes {
        let guid = format_guid(node)?;
        nodes.insert(guid, node.clone());
    }

    // 2. Build parent-child relationships (store position and GUID separately)
    for node in &node_changes {
        if let Some(parent_index) = node.get("parentIndex") {
            let parent_guid = format_parent_guid(parent_index)?;
            let child_guid = format_guid(node)?;
            let position = parent_index
                .get("position")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            parent_to_children
                .entry(parent_guid)
                .or_default()
                .push((position, child_guid));
        }
    }

    // 3. Sort children by position
    for children in parent_to_children.values_mut() {
        children.sort_by(|a, b| a.0.cmp(&b.0));
    }

    // 4. Diagnostic logging when FIG2JSON_DEBUG=1
    if debug {
        eprintln!("[fig2json:debug] build_tree: total nodes in nodeChanges: {}", node_changes.len());
        eprintln!("[fig2json:debug] build_tree: unique node GUIDs: {}", nodes.len());

        // Check root "0:0"
        if nodes.contains_key("0:0") {
            let root_children = parent_to_children.get("0:0").map(|c| c.len()).unwrap_or(0);
            eprintln!("[fig2json:debug] build_tree: root '0:0' exists, children: {}", root_children);
            if let Some(children) = parent_to_children.get("0:0") {
                for (_pos, child_guid) in children {
                    let name = nodes.get(child_guid)
                        .and_then(|n| n.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unnamed");
                    eprintln!("[fig2json:debug]   child: {} (name: {})", child_guid, name);
                }
            }
        } else {
            eprintln!("[fig2json:debug] build_tree: WARNING - root '0:0' NOT found in nodes");
        }

        // Detect orphans: nodes with no parentIndex that are not "0:0"
        let mut orphan_count = 0;
        for node in &node_changes {
            if node.get("parentIndex").is_none() {
                if let Ok(guid) = format_guid(node) {
                    if guid != "0:0" {
                        let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed");
                        eprintln!("[fig2json:debug]   orphan: {} (name: {})", guid, name);
                        orphan_count += 1;
                    }
                }
            }
        }
        if orphan_count > 0 {
            eprintln!("[fig2json:debug] build_tree: WARNING - {} orphan nodes (no parentIndex)", orphan_count);
        } else {
            eprintln!("[fig2json:debug] build_tree: no orphan nodes");
        }
    }

    // 5. Build tree recursively from root
    build_node_tree("0:0", &nodes, &parent_to_children)
}

/// Recursively build a node with its children
fn build_node_tree(
    guid: &str,
    nodes: &HashMap<String, JsonValue>,
    parent_to_children: &HashMap<String, Vec<(String, String)>>,
) -> Result<JsonValue> {
    // Get the node
    let mut node = nodes
        .get(guid)
        .ok_or_else(|| FigError::ZipError(format!("Node {} not found", guid)))?
        .clone();

    // Remove parentIndex
    if let Some(obj) = node.as_object_mut() {
        obj.remove("parentIndex");

        // Add children recursively
        if let Some(child_entries) = parent_to_children.get(guid) {
            let mut children = Vec::new();
            for (_position, child_guid) in child_entries {
                let child_node = build_node_tree(child_guid, nodes, parent_to_children)?;
                children.push(child_node);
            }

            if !children.is_empty() {
                obj.insert("children".to_string(), JsonValue::Array(children));
            }
        }
    }

    Ok(node)
}

/// Format a GUID from a node's guid field
///
/// Converts `{sessionID: X, localID: Y}` to string "X:Y"
fn format_guid(node: &JsonValue) -> Result<String> {
    let guid_obj = node
        .get("guid")
        .and_then(|v| v.as_object())
        .ok_or_else(|| FigError::ZipError("Node missing guid field".to_string()))?;

    let session_id = guid_obj
        .get("sessionID")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| FigError::ZipError("Invalid sessionID in guid".to_string()))?;

    let local_id = guid_obj
        .get("localID")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| FigError::ZipError("Invalid localID in guid".to_string()))?;

    Ok(format!("{}:{}", session_id, local_id))
}

/// Format a GUID from a parentIndex's guid field
fn format_parent_guid(parent_index: &JsonValue) -> Result<String> {
    let guid_obj = parent_index
        .get("guid")
        .and_then(|v| v.as_object())
        .ok_or_else(|| FigError::ZipError("parentIndex missing guid field".to_string()))?;

    let session_id = guid_obj
        .get("sessionID")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| FigError::ZipError("Invalid sessionID in parentIndex".to_string()))?;

    let local_id = guid_obj
        .get("localID")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| FigError::ZipError("Invalid localID in parentIndex".to_string()))?;

    Ok(format!("{}:{}", session_id, local_id))
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_guid() {
        let node = json!({
            "guid": {
                "sessionID": 1,
                "localID": 42
            }
        });

        assert_eq!(format_guid(&node).unwrap(), "1:42");
    }

    #[test]
    fn test_format_parent_guid() {
        let parent_index = json!({
            "guid": {
                "sessionID": 0,
                "localID": 1
            },
            "position": "!"
        });

        assert_eq!(format_parent_guid(&parent_index).unwrap(), "0:1");
    }

    #[test]
    fn test_build_tree_simple() {
        let node_changes = vec![
            json!({
                "guid": {"sessionID": 0, "localID": 0},
                "name": "Root",
                "type": "DOCUMENT"
            }),
            json!({
                "guid": {"sessionID": 0, "localID": 1},
                "parentIndex": {
                    "guid": {"sessionID": 0, "localID": 0},
                    "position": "a"
                },
                "name": "Child1"
            }),
            json!({
                "guid": {"sessionID": 0, "localID": 2},
                "parentIndex": {
                    "guid": {"sessionID": 0, "localID": 0},
                    "position": "b"
                },
                "name": "Child2"
            }),
        ];

        let root = build_tree(node_changes).unwrap();

        // Check root
        assert_eq!(root.get("name").and_then(|v| v.as_str()), Some("Root"));

        // Check children
        let children = root.get("children").and_then(|v| v.as_array()).unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].get("name").and_then(|v| v.as_str()), Some("Child1"));
        assert_eq!(children[1].get("name").and_then(|v| v.as_str()), Some("Child2"));

        // Check parentIndex is removed
        assert!(children[0].get("parentIndex").is_none());
    }

    #[test]
    fn test_sort_children_by_position() {
        let node_changes = vec![
            json!({
                "guid": {"sessionID": 0, "localID": 0},
                "name": "Root"
            }),
            json!({
                "guid": {"sessionID": 0, "localID": 1},
                "parentIndex": {
                    "guid": {"sessionID": 0, "localID": 0},
                    "position": "z"  // Should be last
                },
                "name": "Third"
            }),
            json!({
                "guid": {"sessionID": 0, "localID": 2},
                "parentIndex": {
                    "guid": {"sessionID": 0, "localID": 0},
                    "position": "a"  // Should be first
                },
                "name": "First"
            }),
            json!({
                "guid": {"sessionID": 0, "localID": 3},
                "parentIndex": {
                    "guid": {"sessionID": 0, "localID": 0},
                    "position": "m"  // Should be second
                },
                "name": "Second"
            }),
        ];

        let root = build_tree(node_changes).unwrap();
        let children = root.get("children").and_then(|v| v.as_array()).unwrap();

        // Check sorted order
        assert_eq!(children[0].get("name").and_then(|v| v.as_str()), Some("First"));
        assert_eq!(children[1].get("name").and_then(|v| v.as_str()), Some("Second"));
        assert_eq!(children[2].get("name").and_then(|v| v.as_str()), Some("Third"));
    }

    #[test]
    fn test_build_tree_with_two_pages() {
        let node_changes = vec![
            json!({
                "guid": {"sessionID": 0, "localID": 0},
                "name": "Root",
                "type": "DOCUMENT"
            }),
            json!({
                "guid": {"sessionID": 0, "localID": 1},
                "parentIndex": {
                    "guid": {"sessionID": 0, "localID": 0},
                    "position": "a"
                },
                "name": "Page 1",
                "type": "CANVAS"
            }),
            json!({
                "guid": {"sessionID": 0, "localID": 2},
                "parentIndex": {
                    "guid": {"sessionID": 0, "localID": 0},
                    "position": "b"
                },
                "name": "Page 2",
                "type": "CANVAS"
            }),
        ];

        let root = build_tree(node_changes).unwrap();
        let children = root.get("children").and_then(|v| v.as_array()).unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].get("name").and_then(|v| v.as_str()), Some("Page 1"));
        assert_eq!(children[1].get("name").and_then(|v| v.as_str()), Some("Page 2"));
    }

    #[test]
    fn test_build_tree_orphan_detection() {
        // Node with no parentIndex and not root should be detected as orphan
        let node_changes = vec![
            json!({
                "guid": {"sessionID": 0, "localID": 0},
                "name": "Root"
            }),
            json!({
                "guid": {"sessionID": 0, "localID": 1},
                "parentIndex": {
                    "guid": {"sessionID": 0, "localID": 0},
                    "position": "a"
                },
                "name": "Child"
            }),
            json!({
                "guid": {"sessionID": 0, "localID": 99},
                "name": "Orphan"
                // No parentIndex
            }),
        ];

        // Should still build tree successfully (orphans are just logged, not errors)
        let root = build_tree(node_changes).unwrap();
        let children = root.get("children").and_then(|v| v.as_array()).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].get("name").and_then(|v| v.as_str()), Some("Child"));
    }
}
