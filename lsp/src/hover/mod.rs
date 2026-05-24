use tower_lsp_server::ls_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};
use crate::ast::Ast;
use crate::semantic::SemanticCache;
use crate::commands::COMMAND_DB;

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
        "compute_id" => hover_definition(node_text, &semantic.compute_defs, "Compute", ast.source),
        "variable" => hover_definition(node_text, &semantic.variable_defs, "Variable", ast.source),
        "thermo_kwarg" => hover_thermo_keyword(node_text),
        "func" => hover_math_function(node_text),
        _ => return None,
    }?;

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: Some(crate::ast::tree_sitter_range_to_lsp(node.range(), ast.source)),
    })
}

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

fn hover_style(name: &str, category: &str) -> Option<String> {
    let styles = match category {
        "fix" => &COMMAND_DB.fix_styles,
        "compute" => &COMMAND_DB.compute_styles,
        _ => return None,
    };
    let style = styles.iter().find(|s| s.name == name)?;
    Some(format!(
        "## `{}` ({:?} Style)\n\n{}\n\n**Since**: {}\n\n**Related**: {}",
        style.name,
        style.category,
        style.doc_full,
        style.since_version.as_deref().unwrap_or("unknown"),
        style.related_commands.join(", "),
    ))
}

fn hover_definition(
    name: &str,
    defs: &std::collections::HashMap<String, crate::semantic::SourceLocation>,
    kind: &str,
    source: &str,
) -> Option<String> {
    let loc = defs.get(name)?;
    let line_content = source
        .lines()
        .nth(loc.line as usize)
        .unwrap_or("")
        .trim();
    Some(format!(
        "## {} `{}`\n\nDefined at line {}\n```lammps\n{}\n```",
        kind,
        name,
        loc.line + 1,
        line_content,
    ))
}

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

fn build_syntax_string(cmd: &crate::commands::schema::CommandDef) -> String {
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
