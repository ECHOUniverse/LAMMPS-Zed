use tower_lsp_server::ls_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};
use tree_sitter::Node;

use crate::ast::Ast;
use crate::commands::schema::{CommandDef, ParameterType, StyleDef};
use crate::commands::COMMAND_DB;
use crate::semantic::SemanticCache;

/// Handle textDocument/hover request.
/// Returns Markdown-formatted hover documentation for the symbol at the given position.
pub fn run_hover(ast: &Ast, semantic: &SemanticCache, position: Position) -> Option<Hover> {
    let byte_offset = crate::ast::position_to_byte_offset(ast.source, position);
    let node = ast.node_at_offset(byte_offset)?;
    let node_kind = node.kind();
    let node_text = ast.node_text(node);

    let markdown = match node_kind {
        "command_name" => hover_command_name(node_text),
        "fix_style" => hover_style(node_text, "fix"),
        "compute_style" => hover_style(node_text, "compute"),
        "fix_id" => hover_definition(node_text, &semantic.fix_defs, "Fix", ast.source),
        "compute_id" => {
            hover_definition(node_text, &semantic.compute_defs, "Compute", ast.source)
        }
        "variable" => {
            hover_definition(node_text, &semantic.variable_defs, "Variable", ast.source)
        }
        "fix" => hover_command_name("fix"),
        "compute" => hover_command_name("compute"),
        "thermo_kwarg" => hover_thermo_keyword(node_text),
        "func" => hover_math_function(node_text),
        _ => {
            // Math functions like sqrt in expressions are parsed as
            // `func` parent with `identifier` child — check the parent.
            if let Some(parent) = node.parent() {
                if parent.kind() == "func" {
                    hover_math_function(node_text)
                } else {
                    hover_by_context(node, node_text, ast)
                }
            } else {
                hover_by_context(node, node_text, ast)
            }
        }
    }?;

    // For fix/compute nodes, the node range spans the entire line;
    // use the first child (the keyword token) for accurate highlight range.
    let range_node = match node_kind {
        "fix" | "compute" => node.child(0).unwrap_or(node),
        _ => node,
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: Some(crate::ast::tree_sitter_range_to_lsp(
            range_node.range(),
            ast.source,
        )),
    })
}

// ── Command name hover ─────────────────────────────────────

fn hover_command_name(name: &str) -> Option<String> {
    let cmd = COMMAND_DB.general_commands.iter().find(|c| c.name == name)?;
    Some(format!(
        "## `{}`\n\n**Category**: {:?}\n\n{}\n\n---\n*Syntax*: `{}`",
        cmd.name,
        cmd.category,
        cmd.doc_full,
        build_syntax_string(cmd),
    ))
}

// ── Style hover (dedicated node types) ─────────────────────

fn hover_style(name: &str, category: &str) -> Option<String> {
    let styles = style_db(category)?;
    let style = styles.iter().find(|s| s.name == name)?;
    Some(format_style_hover(style))
}

fn style_db(category: &str) -> Option<&Vec<StyleDef>> {
    match category {
        "pair" => Some(&COMMAND_DB.pair_styles),
        "fix" => Some(&COMMAND_DB.fix_styles),
        "compute" => Some(&COMMAND_DB.compute_styles),
        "bond" => Some(&COMMAND_DB.bond_styles),
        "angle" => Some(&COMMAND_DB.angle_styles),
        "dihedral" => Some(&COMMAND_DB.dihedral_styles),
        "improper" => Some(&COMMAND_DB.improper_styles),
        "kspace" => Some(&COMMAND_DB.kspace_styles),
        _ => None,
    }
}

fn format_style_hover(style: &StyleDef) -> String {
    let mut parts = vec![format!(
        "## `{}` ({:?} Style)\n\n{}",
        style.name, style.category, style.doc_full,
    )];

    if let Some(ref ver) = style.since_version {
        parts.push(format!("**Since**: {}", ver));
    }

    if !style.required_args.is_empty() {
        let args: Vec<String> = style.required_args.iter()
            .map(|p| format!("`{}`", p.name))
            .collect();
        parts.push(format!("**Required args**: {}", args.join(", ")));
    }

    if !style.optional_args.is_empty() {
        let args: Vec<String> = style.optional_args.iter()
            .map(|p| format!("`[{}]`", p.name))
            .collect();
        parts.push(format!("**Optional args**: {}", args.join(", ")));
    }

    if !style.related_commands.is_empty() {
        parts.push(format!("**Related**: {}", style.related_commands.join(", ")));
    }

    parts.join("\n\n")
}

// ── Definition hover ───────────────────────────────────────

fn hover_definition(
    name: &str,
    defs: &std::collections::HashMap<String, crate::semantic::SourceLocation>,
    kind: &str,
    source: &str,
) -> Option<String> {
    let loc = defs.get(name)?;
    let line_content = source.lines().nth(loc.line as usize).unwrap_or("").trim();
    Some(format!(
        "## {} `{}`\n\nDefined at line {}\n```lammps\n{}\n```",
        kind,
        name,
        loc.line + 1,
        line_content,
    ))
}

// ── Thermo keyword hover ───────────────────────────────────

fn hover_thermo_keyword(kw: &str) -> Option<String> {
    let doc = match kw {
        "step" => "Timestep number",
        "temp" => "Temperature",
        "press" => "Pressure",
        "pe" => "Potential energy",
        "ke" => "Kinetic energy",
        "etotal" => "Total energy (pe + ke)",
        "vol" => "Volume",
        "density" => "Number density",
        "cpu" => "CPU time in seconds",
        "lx" | "ly" | "lz" => "Box length in x/y/z direction",
        _ => return None,
    };
    Some(format!("## `{}`\n\n**Thermo Keyword**: {}", kw, doc))
}

// ── Math function hover ────────────────────────────────────

fn hover_math_function(name: &str) -> Option<String> {
    let doc = match name {
        "sqrt" => "`sqrt(x)` — Square root",
        "exp" => "`exp(x)` — Exponential (e^x)",
        "log" => "`log(x)` — Base-10 logarithm",
        "ln" => "`ln(x)` — Natural logarithm",
        "abs" => "`abs(x)` — Absolute value",
        "sin" | "cos" | "tan" => &format!("`{0}(x)` — Trigonometric {0}", name),
        "asin" | "acos" | "atan" => &format!("`{0}(x)` — Inverse trigonometric {0}", name),
        "atan2" => "`atan2(y, x)` — Two-argument arctangent",
        "sinh" | "cosh" | "tanh" => &format!("`{0}(x)` — Hyperbolic {0}", name),
        "erf" => "`erf(x)` — Error function",
        "erfc" => "`erfc(x)` — Complementary error function",
        "min" | "max" => &format!("`{0}(a, b)` — {0}imum of two values", name),
        "ceil" => "`ceil(x)` — Ceiling (smallest integer >= x)",
        "floor" => "`floor(x)` — Floor (largest integer <= x)",
        "round" => "`round(x)` — Round to nearest integer",
        _ => return None,
    };
    Some(format!("## `{}`\n\n**Math Function**\n\n{}", name, doc))
}

// ── Context-based hover ────────────────────────────────────

/// Handles hover for nodes that don't have dedicated grammar node types.
/// Walks up to find the enclosing command, then tries:
/// 1. Style hover (pair_style/bond_style/angle_style/... first argument)
/// 2. Parameter-level hover for known commands
fn hover_by_context(node: Node, node_text: &str, ast: &Ast) -> Option<String> {
    // Don't trigger for empty/whitespace text
    if node_text.trim().is_empty() {
        return None;
    }

    let cmd_node = find_parent_command(node)?;
    let cmd_name = resolve_command_name(cmd_node, ast)?;

    // 1. Try style hover for style-setting commands whose grammar lacks dedicated nodes
    if let Some(category) = style_category_for_command(cmd_name) {
        if let Some(hover) = hover_style(node_text, category) {
            return Some(hover);
        }
    }

    // 2. Try parameter-level hover for commands in the database
    hover_parameter(node, cmd_node, cmd_name, ast)
}

/// Walk up the tree to find an enclosing command/fix/compute node.
fn find_parent_command(node: Node) -> Option<Node> {
    let mut current = node;
    loop {
        match current.kind() {
            "command" | "fix" | "compute" | "variable_def" | "variable_del" => {
                return Some(current);
            }
            _ => {}
        }
        current = current.parent()?;
    }
}

/// Get the logical command name from a command node.
fn resolve_command_name<'a>(cmd_node: Node<'a>, ast: &Ast<'a>) -> Option<&'a str> {
    match cmd_node.kind() {
        "command" => ast.find_command_name_child(cmd_node),
        "fix" => Some("fix"),
        "compute" => Some("compute"),
        "variable_def" | "variable_del" => Some("variable"),
        _ => None,
    }
}

/// Map a command name to a style category if it's a style-setting command.
fn style_category_for_command(cmd_name: &str) -> Option<&'static str> {
    match cmd_name {
        "pair_style" => Some("pair"),
        "bond_style" => Some("bond"),
        "angle_style" => Some("angle"),
        "dihedral_style" => Some("dihedral"),
        "improper_style" => Some("improper"),
        "kspace_style" => Some("kspace"),
        _ => None,
    }
}

/// Try parameter-level hover: show documentation for the specific argument
/// at the cursor position, with a link to the parent command's documentation.
fn hover_parameter(
    node: Node,
    cmd_node: Node,
    cmd_name: &str,
    _ast: &Ast,
) -> Option<String> {
    let cmd = COMMAND_DB
        .general_commands
        .iter()
        .find(|c| c.name == cmd_name)?;

    let arg_idx = find_arg_position(node, cmd_node)?;
    let param = cmd.parameters.get(arg_idx)?;

    let mut result = format!(
        "**`{}`** — *{}*\n\n{}",
        param.name,
        format_param_type(&param.param_type),
        param.doc,
    );

    // Append documentation link extracted from the command's doc_full.
    if let Some(link) = extract_doc_link(&cmd.doc_full) {
        result.push_str(&format!("\n\n---\n[📖 {} 文档]({})", cmd_name, link));
    }

    Some(result)
}

/// Extract the first markdown link URL from a doc string.
/// Looks for patterns like `[Documentation](https://...)`.
fn extract_doc_link(doc: &str) -> Option<&str> {
    // Find markdown link pattern: [text](url)
    if let Some(start) = doc.find("](") {
        let url_start = start + 2;
        if let Some(end) = doc[url_start..].find(')') {
            return Some(&doc[url_start..url_start + end]);
        }
    }
    // Also try raw URL (https://...)
    if let Some(start) = doc.find("https://") {
        let rest = &doc[start..];
        let end = rest.find(|c: char| c.is_whitespace() || c == ')')
            .unwrap_or(rest.len());
        return Some(&rest[..end]);
    }
    None
}

/// Determine the 0-based argument index that `node` occupies within `cmd_node`.
fn find_arg_position(node: Node, cmd_node: Node) -> Option<usize> {
    let mut arg_idx = 0;
    for i in 0..cmd_node.named_child_count() {
        let child = cmd_node.named_child(i)?;
        if child.kind() == "args_under" {
            for j in 0..child.named_child_count() {
                let arg = child.named_child(j)?;
                if node == arg || is_descendant_of(node, arg) {
                    return Some(arg_idx);
                }
                arg_idx += 1;
            }
        }
    }
    None
}

fn is_descendant_of(mut node: Node, ancestor: Node) -> bool {
    while let Some(parent) = node.parent() {
        if parent == ancestor {
            return true;
        }
        node = parent;
    }
    false
}

// ── Syntax formatting ──────────────────────────────────────

fn build_syntax_string(cmd: &CommandDef) -> String {
    let params: Vec<String> = cmd
        .parameters
        .iter()
        .map(|p| {
            if p.required {
                p.name.clone()
            } else {
                format!("[{}]", p.name)
            }
        })
        .collect();
    format!("{} {}", cmd.name, params.join(" "))
}

fn format_param_type(pt: &ParameterType) -> String {
    match pt {
        ParameterType::Style => "style".to_string(),
        ParameterType::GroupId => "group ID".to_string(),
        ParameterType::FixId => "fix ID".to_string(),
        ParameterType::ComputeId => "compute ID".to_string(),
        ParameterType::Variable => "variable".to_string(),
        ParameterType::VariableName => "variable name".to_string(),
        ParameterType::FileName => "filename".to_string(),
        ParameterType::Label => "label".to_string(),
        ParameterType::Integer => "integer".to_string(),
        ParameterType::Float => "float".to_string(),
        ParameterType::Boolean => "boolean".to_string(),
        ParameterType::String => "string".to_string(),
        ParameterType::Enum(values) => format!("`{}`", values.join("` | `")),
        ParameterType::Expression => "expression".to_string(),
        ParameterType::Keyword(k) => format!("`\"{}\"`", k),
        ParameterType::Repeat(inner) => format!("{}…", format_param_type(inner)),
    }
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_hover(source: &str, line: u32, character: u32) -> Option<String> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_lammps::LANGUAGE.into())
            .expect("load grammar");
        let tree = parser.parse(source, None).expect("parse");
        let ast = Ast::new(source, &tree);
        let semantic = SemanticCache::build(&ast, "file:///test.in");
        let result = run_hover(&ast, &semantic, Position { line, character });
        result.map(|h| match h.contents {
            HoverContents::Markup(m) => m.value,
            _ => String::new(),
        })
    }

    /// Helper: get hover content as string, panicking if None.
    fn hover_text(source: &str, line: u32, character: u32) -> String {
        parse_and_hover(source, line, character).unwrap_or_else(|| {
            panic!(
                "Expected hover at line {}, char {} in:\n{}",
                line, character, source
            )
        })
    }

    // ── Command name hover ────────────────────────────────

    #[test]
    fn test_command_name_units() {
        let src = "units metal\n";
        let h = hover_text(src, 0, 0);
        assert!(h.contains("`units`"), "should show command name: {}", h);
        assert!(h.contains("Set the units"), "should show doc: {}", h);
    }

    #[test]
    fn test_command_name_create_box() {
        let src = "create_box 1 box\n";
        let h = hover_text(src, 0, 1);
        assert!(h.contains("`create_box`"), "should show command name: {}", h);
        assert!(h.contains("Create a simulation box"), "should show doc: {}", h);
    }

    #[test]
    fn test_command_name_pair_style() {
        let src = "pair_style eam/alloy\n";
        let h = hover_text(src, 0, 0);
        assert!(h.contains("`pair_style`"), "should show command name: {}", h);
        assert!(h.contains("pairwise interactions"), "should show doc: {}", h);
    }

    #[test]
    fn test_command_name_unknown_returns_none() {
        let src = "nonexistent_cmd arg1 arg2\n";
        assert!(parse_and_hover(src, 0, 0).is_none());
    }

    // ── Parameter-level hover ─────────────────────────────

    #[test]
    fn test_parameter_hover_units_metal() {
        let src = "units metal\n";
        let h = hover_text(src, 0, 7); // cursor on "metal"
        assert!(h.contains("style"), "should show param name 'style': {}", h);
        assert!(h.contains("Unit style"), "should show param doc: {}", h);
    }

    #[test]
    fn test_parameter_hover_create_box_n() {
        let src = "create_box 1 box\n";
        let h = hover_text(src, 0, 12); // cursor on "1"
        assert!(h.contains("N"), "should show param name: {}", h);
        assert!(h.contains("atom types"), "should show doc: {}", h);
    }

    #[test]
    fn test_parameter_hover_dimension() {
        let src = "dimension 3\n";
        let h = hover_text(src, 0, 11); // cursor on "3"
        assert!(h.contains("N"), "should show param name: {}", h);
    }

    #[test]
    fn test_parameter_hover_boundary() {
        let src = "boundary p p p\n";
        let h = hover_text(src, 0, 10); // cursor on first "p"
        assert!(h.contains("x"), "should show param name x: {}", h);
    }

    // ── Fix/compute style hover (dedicated node, existing) ─

    #[test]
    fn test_fix_style_hover() {
        let src = "fix 1 all npt temp 300 300 0.1\n";
        let h = hover_text(src, 0, 11); // cursor on "npt"
        assert!(h.contains("npt"), "should show style name: {}", h);
        assert!(h.contains("Fix"), "should show Fix category: {}", h);
    }

    #[test]
    fn test_compute_style_hover() {
        let src = "compute myTemp all temp\n";
        let h = hover_text(src, 0, 19); // cursor on "temp"
        assert!(h.contains("temp"), "should show style name: {}", h);
        assert!(h.contains("Compute"), "should show Compute category: {}", h);
    }

    // ── Context-based pair/bond/angle style hover ─────────

    #[test]
    fn test_pair_style_lj_cut_hover() {
        let src = "pair_style lj/cut 10.0\n";
        let h = hover_text(src, 0, 12); // cursor on "lj/cut"
        assert!(h.contains("lj/cut"), "should show style name: {}", h);
        assert!(h.contains("Lennard-Jones"), "should show doc: {}", h);
    }

    #[test]
    fn test_pair_style_eam_alloy_hover() {
        let src = "pair_style eam/alloy\n";
        let h = hover_text(src, 0, 12); // cursor on "eam/alloy"
        assert!(h.contains("eam/alloy"), "should show style name: {}", h);
        assert!(h.contains("Pair"), "should show Pair category: {}", h);
    }

    #[test]
    fn test_bond_style_harmonic_hover() {
        let src = "bond_style harmonic\n";
        let h = hover_text(src, 0, 12); // cursor on "harmonic"
        assert!(h.contains("harmonic"), "should show style name: {}", h);
        assert!(h.contains("Bond"), "should show Bond category: {}", h);
    }

    #[test]
    fn test_angle_style_hover() {
        let src = "angle_style cosine\n";
        let h = hover_text(src, 0, 13); // cursor on "cosine"
        assert!(h.contains("cosine"), "should show style name: {}", h);
        assert!(h.contains("Angle"), "should show Angle category: {}", h);
    }

    #[test]
    fn test_pair_style_none_hover() {
        let src = "pair_style none\n";
        let h = hover_text(src, 0, 12); // cursor on "none"
        assert!(h.contains("none"), "should show style: {}", h);
        assert!(h.contains("pairwise"), "should show doc: {}", h);
    }

    // ── Thermo and math (existing, regression) ────────────

    #[test]
    fn test_thermo_keyword_hover_parameter() {
        let src = "thermo_style custom step temp press\n";
        // "step" is not parsed as thermo_kwarg by the grammar;
        // context-based hover shows parameter docs from the thermo_style command
        let h = hover_text(src, 0, 21); // cursor on "step"
        assert!(h.contains("keywords"), "should show param name: {}", h);
    }

    #[test]
    fn test_math_function_hover() {
        let src = "variable x equal sqrt(4.0)\n";
        let h = hover_text(src, 0, 17); // cursor on "sqrt"
        assert!(h.contains("sqrt"), "should show function name: {}", h);
        assert!(h.contains("Square root"), "should show doc: {}", h);
    }

    // ── Edge cases ────────────────────────────────────────

    #[test]
    fn test_comment_no_hover() {
        let src = "# this is a comment\n";
        assert!(parse_and_hover(src, 0, 3).is_none());
    }

    #[test]
    fn test_whitespace_no_hover() {
        let src = "units   metal\n";
        // cursor on spaces between "units" and "metal"
        assert!(parse_and_hover(src, 0, 6).is_none());
    }

    #[test]
    fn test_variable_definition_hover() {
        let src = "variable T equal 300.0\n";
        let h = hover_text(src, 0, 9); // cursor on "T" (variable name)
        assert!(h.contains("Variable"), "should show Variable def: {}", h);
        assert!(h.contains("line 1"), "should show line number: {}", h);
    }

    // ── Fix/compute command keyword hover ──────────────────

    #[test]
    fn test_fix_keyword_hover() {
        let src = "fix 1 all npt temp 300 300 0.1\n";
        let h = hover_text(src, 0, 0); // cursor on "fix"
        assert!(h.contains("`fix`"), "should show command name: {}", h);
        assert!(h.contains("Set a fix"), "should show doc: {}", h);
    }

    #[test]
    fn test_compute_keyword_hover() {
        let src = "compute myTemp all temp\n";
        let h = hover_text(src, 0, 0); // cursor on "compute"
        assert!(h.contains("`compute`"), "should show command name: {}", h);
        assert!(h.contains("compute"), "should show doc: {}", h);
    }

    // ── Doc link in parameter hover ───────────────────────

    #[test]
    fn test_parameter_hover_has_doc_link() {
        let src = "units metal\n";
        let h = hover_text(src, 0, 7); // cursor on "metal"
        assert!(h.contains("docs.lammps.org/units"), "should contain doc link: {}", h);
    }

    #[test]
    fn test_parameter_hover_boundary_has_link() {
        let src = "boundary p p p\n";
        let h = hover_text(src, 0, 10); // cursor on first "p"
        assert!(h.contains("docs.lammps.org/boundary"), "should contain doc link: {}", h);
    }

}
