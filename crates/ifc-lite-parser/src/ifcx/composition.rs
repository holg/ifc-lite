// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! IFCX ECS composition algorithm
//!
//! Flattens the IFCX node graph by:
//! 1. Merging attributes from multiple nodes with same path
//! 2. Resolving inheritance chains (`inherits`)
//! 3. Building parent-child relationships from `children`

use super::types::{ComposedNode, IfcxNode};
use rustc_hash::{FxHashMap, FxHashSet};

/// Compose IFCX nodes into flattened structure
///
/// IFCX allows multiple nodes with the same path (layered data).
/// Later nodes' attributes override earlier ones.
pub fn compose_nodes(nodes: &[IfcxNode]) -> FxHashMap<String, ComposedNode> {
    // Phase 1: Merge nodes with same path
    let mut path_to_node: FxHashMap<String, ComposedNode> = FxHashMap::default();
    let mut child_to_parent: FxHashMap<String, String> = FxHashMap::default();

    for node in nodes {
        let entry = path_to_node
            .entry(node.path.clone())
            .or_insert_with(|| ComposedNode {
                path: node.path.clone(),
                attributes: FxHashMap::default(),
                children: Vec::new(),
                parent: None,
            });

        // Merge attributes (later wins)
        for (key, value) in &node.attributes {
            entry.attributes.insert(key.clone(), value.clone());
        }

        // Collect children
        for (name, child_path) in &node.children {
            if let Some(path) = child_path {
                if !entry.children.contains(path) {
                    entry.children.push(path.clone());
                }
                child_to_parent.insert(path.clone(), node.path.clone());
            }
            // Also store named child reference
            let _ = name; // Name is metadata, path is identity
        }
    }

    // Phase 2: Resolve inheritance
    // Build inheritance graph first to detect cycles
    let mut inherits_from: FxHashMap<String, Vec<String>> = FxHashMap::default();
    for node in nodes {
        for (_, parent_path) in &node.inherits {
            if let Some(parent) = parent_path {
                inherits_from
                    .entry(node.path.clone())
                    .or_default()
                    .push(parent.clone());
            }
        }
    }

    // Resolve inheritance with cycle detection
    let mut resolved: FxHashSet<String> = FxHashSet::default();
    let paths: Vec<_> = path_to_node.keys().cloned().collect();

    for path in &paths {
        resolve_inheritance(
            path,
            &mut path_to_node,
            &inherits_from,
            &mut resolved,
            &mut FxHashSet::default(),
        );
    }

    // Phase 3: Set parent references
    for (child, parent) in child_to_parent {
        if let Some(node) = path_to_node.get_mut(&child) {
            node.parent = Some(parent);
        }
    }

    path_to_node
}

/// Recursively resolve inheritance for a node
fn resolve_inheritance(
    path: &str,
    nodes: &mut FxHashMap<String, ComposedNode>,
    inherits_from: &FxHashMap<String, Vec<String>>,
    resolved: &mut FxHashSet<String>,
    in_progress: &mut FxHashSet<String>,
) {
    if resolved.contains(path) {
        return;
    }

    // Cycle detection
    if in_progress.contains(path) {
        // Cycle detected, skip
        return;
    }

    in_progress.insert(path.to_string());

    // First resolve parents
    if let Some(parents) = inherits_from.get(path) {
        for parent_path in parents {
            resolve_inheritance(parent_path, nodes, inherits_from, resolved, in_progress);
        }

        // Then merge parent attributes into this node (child overrides parent)
        let parent_attrs: Vec<_> = parents
            .iter()
            .filter_map(|p| nodes.get(p))
            .flat_map(|n| n.attributes.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        if let Some(node) = nodes.get_mut(path) {
            for (key, value) in parent_attrs {
                // Only insert if not already present (child overrides parent)
                node.attributes.entry(key).or_insert(value);
            }
        }
    }

    in_progress.remove(path);
    resolved.insert(path.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn make_node(path: &str, attrs: serde_json::Value) -> IfcxNode {
        IfcxNode {
            path: path.to_string(),
            children: HashMap::new(),
            inherits: HashMap::new(),
            attributes: attrs
                .as_object()
                .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
        }
    }

    #[test]
    fn test_merge_same_path() {
        let nodes = vec![
            make_node("a", json!({"x": 1})),
            make_node("a", json!({"y": 2})),
            make_node("a", json!({"x": 3})), // Override x
        ];

        let composed = compose_nodes(&nodes);
        let a = composed.get("a").unwrap();

        assert_eq!(a.attributes.get("x").unwrap(), &json!(3)); // Later wins
        assert_eq!(a.attributes.get("y").unwrap(), &json!(2));
    }
}
