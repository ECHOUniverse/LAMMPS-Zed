use std::collections::HashMap;
use tree_sitter::Node;
use crate::ast::Ast;

/// Cached semantic analysis results per document.
#[derive(Debug, Clone, Default)]
pub struct SemanticCache {
    pub variable_defs: HashMap<String, SourceLocation>,
    pub fix_defs: HashMap<String, SourceLocation>,
    pub compute_defs: HashMap<String, SourceLocation>,
    pub labels: HashMap<String, SourceLocation>,
    pub variable_refs: Vec<(String, SourceLocation)>,
    pub include_targets: Vec<IncludeTargetInfo>,
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub line: u32,
    pub character: u32,
    pub uri: String,
}

impl SourceLocation {
    /// Create a SourceLocation from a tree-sitter Node and a URI.
    pub fn from_node(node: Node, uri: &str) -> Self {
        let pos = node.start_position();
        Self {
            line: pos.row as u32,
            character: pos.column as u32,
            uri: uri.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IncludeTargetInfo {
    pub file_path: String,
    pub label: Option<String>,
    pub is_jump: bool,
    pub line: u32,
}

impl SemanticCache {
    /// Build the semantic cache from an AST by collecting all definitions,
    /// references, and include targets.
    pub fn build(ast: &Ast, uri: &str) -> Self {
        let mut cache = Self::default();

        // 1. Variable definitions
        for def in ast.variable_definitions() {
            cache.variable_defs.insert(
                def.name.to_string(),
                SourceLocation::from_node(def.node, uri),
            );
        }

        // 2. Fix definitions
        for def in ast.fix_definitions() {
            cache.fix_defs.insert(
                def.name.to_string(),
                SourceLocation::from_node(def.node, uri),
            );
        }

        // 3. Compute definitions
        for def in ast.compute_definitions() {
            cache.compute_defs.insert(
                def.name.to_string(),
                SourceLocation::from_node(def.node, uri),
            );
        }

        // 4. Variable references
        for r in ast.variable_references() {
            cache.variable_refs.push((
                r.name.to_string(),
                SourceLocation::from_node(r.node, uri),
            ));
        }

        // 5. Include targets
        for t in ast.include_targets() {
            cache.include_targets.push(IncludeTargetInfo {
                file_path: t.file_path.to_string(),
                label: t.label.map(|s| s.to_string()),
                is_jump: t.is_jump,
                line: t.node.start_position().row as u32,
            });
        }

        // 6. Labels
        for l in ast.labels() {
            cache.labels.insert(
                l.name.to_string(),
                SourceLocation::from_node(l.node, uri),
            );
        }

        cache
    }

    pub fn invalidate(&mut self) {
        *self = Self::default();
    }
}
