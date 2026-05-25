use tree_sitter::{Node, Tree, Range, TreeCursor};

/// Typed abstraction over tree-sitter AST nodes.
/// Provides cursor-based iteration over LAMMPS-specific constructs.
pub struct Ast<'a> {
    pub source: &'a str,
    pub tree: &'a Tree,
}

impl<'a> Ast<'a> {
    pub fn new(source: &'a str, tree: &'a Tree) -> Self {
        Self { source, tree }
    }

    /// Get the root node of the tree.
    pub fn root_node(&self) -> Node<'a> {
        self.tree.root_node()
    }

    /// Find the deepest node at a given byte offset.
    pub fn node_at_offset(&self, offset: usize) -> Option<Node<'a>> {
        let root = self.root_node();
        self.descendant_at_offset(root, offset)
    }

    /// Get the ancestor chain (from root to leaf) at a given offset.
    pub fn scope_at_offset(&self, offset: usize) -> Vec<Node<'a>> {
        let mut ancestors = Vec::new();
        let root = self.root_node();
        self.collect_ancestors(root, offset, &mut ancestors);
        ancestors
    }

    /// Get text for a node.
    pub fn node_text(&self, node: Node<'a>) -> &'a str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    // ── Iterators ──────────────────────────────────────────

    /// Iterate all command-like nodes (command, fix, compute, variable_def, variable_del, shell).
    pub fn commands(&self) -> Vec<CommandNode<'a>> {
        let mut cmds = Vec::new();
        let mut cursor = self.root_node().walk();
        self.collect_commands(&mut cursor, &mut cmds);
        cmds
    }

    /// Get the command keyword from a command node.
    /// For generic commands: returns the command_name text (e.g., "pair_coeff").
    /// For fix/compute/variable_def/variable_del/shell: returns the type keyword.
    pub fn command_name(&self, node: Node<'a>) -> Option<&'a str> {
        match node.kind() {
            "command" => self.find_command_name_child(node),
            "fix" => Some("fix"),
            "compute" => Some("compute"),
            "variable_def" => Some("variable"),
            "variable_del" => Some("variable"),
            "shell" => Some("shell"),
            _ => None,
        }
    }

    /// Iterate all variable definitions.
    pub fn variable_definitions(&self) -> Vec<VariableDef<'a>> {
        let mut defs = Vec::new();
        let mut cursor = self.root_node().walk();
        self.collect_variable_defs(&mut cursor, &mut defs);
        defs
    }

    /// Iterate all fix definitions.
    pub fn fix_definitions(&self) -> Vec<FixOrComputeDef<'a>> {
        let mut defs = Vec::new();
        let mut cursor = self.root_node().walk();
        self.collect_fix_defs(&mut cursor, &mut defs);
        defs
    }

    /// Iterate all compute definitions.
    pub fn compute_definitions(&self) -> Vec<FixOrComputeDef<'a>> {
        let mut defs = Vec::new();
        let mut cursor = self.root_node().walk();
        self.collect_compute_defs(&mut cursor, &mut defs);
        defs
    }

    /// Find all variable references ($x via simple_expansion, ${x} via var_curly,
    /// $(x) via var_round, v_x/c_x/f_x via underscore_ident).
    ///
    /// For Dollar ($x) and Curly (${x}), `name` is the bare variable name.
    /// For Underscore (v_x/c_x/f_x), `name` includes the prefix (e.g., "v_x").
    /// For Round ($(...)), `name` is the full expression text.
    pub fn variable_references(&self) -> Vec<VariableRef<'a>> {
        let mut refs = Vec::new();
        let mut cursor = self.root_node().walk();
        self.collect_variable_refs(&mut cursor, &mut refs);
        refs
    }

    /// Find all include/jump targets.
    pub fn include_targets(&self) -> Vec<IncludeTarget<'a>> {
        let mut targets = Vec::new();
        for cmd in self.commands() {
            if cmd.kind != CommandKind::General {
                continue;
            }
            let cname = self.command_name(cmd.node).unwrap_or("");
            match cname {
                "include" => {
                    if let Some(file_path) = self.first_arg_text(cmd.node) {
                        targets.push(IncludeTarget {
                            node: cmd.node,
                            file_path,
                            label: None,
                            is_jump: false,
                        });
                    }
                }
                "jump" => {
                    let file_path = self.first_arg_text(cmd.node);
                    let label = self.nth_arg_text(cmd.node, 1);
                    if let Some(fp) = file_path {
                        targets.push(IncludeTarget {
                            node: cmd.node,
                            file_path: fp,
                            label,
                            is_jump: true,
                        });
                    }
                }
                _ => {}
            }
        }
        targets
    }

    /// Find all label definitions (from `label` commands).
    pub fn labels(&self) -> Vec<LabelDef<'a>> {
        let mut labels = Vec::new();
        for cmd in self.commands() {
            if cmd.kind != CommandKind::General {
                continue;
            }
            let cname = self.command_name(cmd.node).unwrap_or("");
            if cname == "label" {
                if let Some(label_name) = self.first_arg_text(cmd.node) {
                    labels.push(LabelDef {
                        node: cmd.node,
                        name: label_name,
                    });
                }
            }
        }
        labels
    }

    /// Iterate all comment nodes.
    pub fn comments(&self) -> Vec<Node<'a>> {
        let mut comments = Vec::new();
        let mut cursor = self.root_node().walk();
        self.collect_comments(&mut cursor, &mut comments);
        comments
    }

    // ── Private: argument extraction ───────────────────────

    /// Extract the first argument text from a command's args_under sections.
    fn first_arg_text(&self, cmd_node: Node<'a>) -> Option<&'a str> {
        self.nth_arg_text(cmd_node, 0)
    }

    /// Extract the nth argument (0-based) from all args_under sections.
    fn nth_arg_text(&self, cmd_node: Node<'a>, n: usize) -> Option<&'a str> {
        let mut arg_idx = 0;
        for i in 0..cmd_node.named_child_count() {
            let child = cmd_node.named_child(i)?;
            if child.kind() == "args_under" {
                for j in 0..child.named_child_count() {
                    let arg = child.named_child(j)?;
                    if arg_idx == n {
                        return Some(self.node_text(arg));
                    }
                    arg_idx += 1;
                }
            }
        }
        None
    }

    /// Find the command_name child within a command node.
    pub fn find_command_name_child(&self, node: Node<'a>) -> Option<&'a str> {
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                if child.kind() == "command_name" {
                    return Some(self.node_text(child));
                }
            }
        }
        None
    }

    // ── Tree walkers ───────────────────────────────────────

    fn collect_commands(&self, cursor: &mut TreeCursor<'a>, out: &mut Vec<CommandNode<'a>>) {
        let node = cursor.node();
        match node.kind() {
            "command" => {
                let name = self.find_command_name_child(node).unwrap_or("");
                out.push(CommandNode { node, kind: CommandKind::General, name });
            }
            "fix" => {
                let name = self.field_text(node, "fix_id").unwrap_or("");
                out.push(CommandNode { node, kind: CommandKind::Fix, name });
            }
            "compute" => {
                let name = self.field_text(node, "compute_id").unwrap_or("");
                out.push(CommandNode { node, kind: CommandKind::Compute, name });
            }
            "variable_def" => {
                let name = self.field_text(node, "name").unwrap_or("");
                out.push(CommandNode { node, kind: CommandKind::VariableDef, name });
            }
            "variable_del" => {
                let name = self.field_text(node, "name").unwrap_or("");
                out.push(CommandNode { node, kind: CommandKind::VariableDel, name });
            }
            "shell" => {
                out.push(CommandNode { node, kind: CommandKind::Shell, name: "shell" });
            }
            _ => {}
        }

        if cursor.goto_first_child() {
            loop {
                self.collect_commands(cursor, out);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn collect_variable_defs(&self, cursor: &mut TreeCursor<'a>, out: &mut Vec<VariableDef<'a>>) {
        let node = cursor.node();
        if node.kind() == "variable_def" {
            let name = self.field_text(node, "name").unwrap_or("");
            let style = self.field_text(node, "style");
            out.push(VariableDef { node, name, style });
        }

        if cursor.goto_first_child() {
            loop {
                self.collect_variable_defs(cursor, out);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn collect_fix_defs(&self, cursor: &mut TreeCursor<'a>, out: &mut Vec<FixOrComputeDef<'a>>) {
        let node = cursor.node();
        if node.kind() == "fix" {
            let name = self.field_text(node, "fix_id").unwrap_or("");
            let style = self.field_text(node, "style");
            out.push(FixOrComputeDef { node, name, style });
        }

        if cursor.goto_first_child() {
            loop {
                self.collect_fix_defs(cursor, out);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn collect_compute_defs(&self, cursor: &mut TreeCursor<'a>, out: &mut Vec<FixOrComputeDef<'a>>) {
        let node = cursor.node();
        if node.kind() == "compute" {
            let name = self.field_text(node, "compute_id").unwrap_or("");
            let style = self.field_text(node, "style");
            out.push(FixOrComputeDef { node, name, style });
        }

        if cursor.goto_first_child() {
            loop {
                self.collect_compute_defs(cursor, out);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn collect_variable_refs(&self, cursor: &mut TreeCursor<'a>, out: &mut Vec<VariableRef<'a>>) {
        let node = cursor.node();
        match node.kind() {
            "simple_expansion" => {
                // $x → simple_expansion node with variable child
                if let Some(var_node) = self.find_named_child(node, "variable") {
                    let name = self.node_text(var_node);
                    if !name.is_empty() {
                        out.push(VariableRef { node, name, ref_kind: VariableRefKind::Dollar });
                    }
                }
            }
            "var_curly" => {
                // ${x} → var_curly node with variable child
                if let Some(var_node) = self.find_named_child(node, "variable") {
                    let name = self.node_text(var_node);
                    if !name.is_empty() {
                        out.push(VariableRef { node, name, ref_kind: VariableRefKind::Curly });
                    }
                }
            }
            "var_round" => {
                // $(expr) → var_round node, name is the full expression
                // Note: var_round child is an expression, not a simple variable
                let text = self.node_text(node);
                if !text.is_empty() {
                    out.push(VariableRef { node, name: text, ref_kind: VariableRefKind::Round });
                }
            }
            "underscore_ident" => {
                // v_x, c_x, f_x → underscore_ident node
                // The node text gives the full form like "v_x", "c_x", "f_x"
                let full_name = self.node_text(node);
                if !full_name.is_empty() {
                    out.push(VariableRef {
                        node,
                        name: full_name,
                        ref_kind: VariableRefKind::Underscore,
                    });
                }
            }
            _ => {}
        }

        if cursor.goto_first_child() {
            loop {
                self.collect_variable_refs(cursor, out);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn collect_comments(&self, cursor: &mut TreeCursor<'a>, out: &mut Vec<Node<'a>>) {
        let node = cursor.node();
        if node.kind() == "comment" {
            out.push(node);
        }

        if cursor.goto_first_child() {
            loop {
                self.collect_comments(cursor, out);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    // ── Node utilities ─────────────────────────────────────

    /// Get the text of a named child with the given field name.
    pub fn field_text(&self, node: Node<'a>, field_name: &str) -> Option<&'a str> {
        node.child_by_field_name(field_name)
            .map(|child| self.node_text(child))
    }

    /// Find a named child by kind.
    fn find_named_child(&self, node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                if child.kind() == kind {
                    return Some(child);
                }
            }
        }
        None
    }

    // ── Original helpers ───────────────────────────────────

    fn descendant_at_offset(&self, node: Node<'a>, offset: usize) -> Option<Node<'a>> {
        if node.start_byte() > offset || node.end_byte() < offset {
            return None;
        }

        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                if child.start_byte() <= offset && child.end_byte() >= offset {
                    return self.descendant_at_offset(child, offset);
                }
            }
        }

        Some(node)
    }

    fn collect_ancestors(&self, node: Node<'a>, offset: usize, result: &mut Vec<Node<'a>>) {
        if node.start_byte() > offset || node.end_byte() < offset {
            return;
        }
        result.push(node);
        for i in 0..node.named_child_count() {
            if let Some(child) = node.named_child(i) {
                if child.start_byte() <= offset && child.end_byte() >= offset {
                    self.collect_ancestors(child, offset, result);
                    return;
                }
            }
        }
    }
}

// ── AST types ──────────────────────────────────────────────

/// The kind of command node found in the AST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    General,       /// Generic command (command_name + args)
    Fix,
    Compute,
    VariableDef,
    VariableDel,
    Shell,
}

/// Typed wrapper for a command node.
#[derive(Debug, Clone)]
pub struct CommandNode<'a> {
    pub node: Node<'a>,
    pub kind: CommandKind,
    pub name: &'a str,
}

impl<'a> CommandNode<'a> {
    pub fn range(&self) -> Range {
        self.node.range()
    }
}

/// A fix or compute definition (shared structure).
#[derive(Debug, Clone)]
pub struct FixOrComputeDef<'a> {
    pub node: Node<'a>,
    pub name: &'a str,
    pub style: Option<&'a str>,
}

/// A variable definition found in the AST.
#[derive(Debug, Clone)]
pub struct VariableDef<'a> {
    pub node: Node<'a>,
    pub name: &'a str,
    pub style: Option<&'a str>,
}

/// A label definition.
#[derive(Debug, Clone)]
pub struct LabelDef<'a> {
    pub node: Node<'a>,
    pub name: &'a str,
}

/// A variable reference found in the AST.
///
/// For Dollar ($x) and Curly (${x}), `name` is the bare variable name (e.g., "x").
/// For Underscore (v_x/c_x/f_x), `name` includes the prefix (e.g., "v_x").
/// For Round ($(...)), `name` is the full expression text.
#[derive(Debug, Clone)]
pub struct VariableRef<'a> {
    pub node: Node<'a>,
    pub name: &'a str,
    pub ref_kind: VariableRefKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableRefKind {
    Dollar,      // $x (via simple_expansion)
    Curly,       // ${x} (via var_curly)
    Round,       // $(x) (via var_round)
    Underscore,  // v_x, c_x, f_x (via underscore_ident)
}

/// An include or jump target.
#[derive(Debug, Clone)]
pub struct IncludeTarget<'a> {
    pub node: Node<'a>,
    pub file_path: &'a str,
    pub label: Option<&'a str>,
    pub is_jump: bool,
}

// ── Coordinate utilities ──────────────────────────────────

/// Convert an LSP Position (0-based line/character) to a byte offset in source.
pub fn position_to_byte_offset(source: &str, position: tower_lsp_server::ls_types::Position) -> usize {
    let mut current_line = 0u32;
    let mut current_char = 0u32;
    for (i, ch) in source.char_indices() {
        if current_line == position.line && current_char == position.character {
            return i;
        }
        if ch == '\n' {
            current_line += 1;
            current_char = 0;
        } else {
            current_char += 1;
        }
    }
    source.len()
}

/// Convert a tree-sitter Range to an LSP Range.
pub fn tree_sitter_range_to_lsp(range: Range, source: &str) -> tower_lsp_server::ls_types::Range {
    let (start_line, start_char) = byte_to_line_char(source, range.start_byte);
    let (end_line, end_char) = byte_to_line_char(source, range.end_byte);

    tower_lsp_server::ls_types::Range {
        start: tower_lsp_server::ls_types::Position {
            line: start_line as u32,
            character: start_char as u32,
        },
        end: tower_lsp_server::ls_types::Position {
            line: end_line as u32,
            character: end_char as u32,
        },
    }
}

pub fn byte_to_line_char(source: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    let mut current_byte = 0;

    for ch in source.chars() {
        if current_byte >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        current_byte += ch.len_utf8();
    }

    (line, col)
}
