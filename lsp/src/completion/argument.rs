use tower_lsp_server::ls_types::{
    CompletionItem, CompletionItemKind, Documentation, MarkupContent, MarkupKind,
};
use tree_sitter::Node;
use crate::ast::Ast;
use crate::commands::schema::{CommandDef, ParameterType};
use crate::commands::COMMAND_DB;
use crate::semantic::SemanticCache;

/// Provide argument / parameter completions based on the enclosing command.
pub fn complete_arguments(
    ast: &Ast,
    semantic: &SemanticCache,
    offset: usize,
    scope: &[Node],
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // ── 1. Thermo keyword completions ──────────────────────────────
    items.extend(complete_thermo_keywords(ast, offset, scope));

    // ── 2. Expression function completions ─────────────────────────
    items.extend(complete_expression_functions(ast, offset, scope));

    // ── 3. Parameter-driven completions for known commands ─────────
    // (covers units/boundary/dimension/atom_style enums from
    //  commands.toml — no more hardcoded duplicate lists)
    items.extend(complete_parameters(ast, semantic, offset, scope));

    items
}

// ────────────────────────────────────────────────────────────────────
// Thermo keywords
// ────────────────────────────────────────────────────────────────────

const THERMO_KEYWORDS: &[(&str, &str)] = &[
    ("step", "Timestep number"),
    ("temp", "Temperature"),
    ("press", "Pressure"),
    ("pe", "Potential energy"),
    ("ke", "Kinetic energy"),
    ("etotal", "Total energy"),
    ("evdwl", "van der Waals energy"),
    ("ecoul", "Coulombic energy"),
    ("elong", "Long-range kspace energy"),
    ("enthalpy", "Enthalpy"),
    ("vol", "Volume"),
    ("density", "Mass density"),
    ("lx", "Box length in x"),
    ("ly", "Box length in y"),
    ("lz", "Box length in z"),
    ("xlo", "Lower x box boundary"),
    ("xhi", "Upper x box boundary"),
    ("ylo", "Lower y box boundary"),
    ("yhi", "Upper y box boundary"),
    ("zlo", "Lower z box boundary"),
    ("zhi", "Upper z box boundary"),
    ("cpu", "CPU time"),
    ("spcpu", "CPU time per step"),
    ("atoms", "Number of atoms"),
    ("nbonds", "Number of bonds"),
    ("nangles", "Number of angles"),
    ("ndihedrals", "Number of dihedrals"),
    ("nimpropers", "Number of impropers"),
    ("fmax", "Maximum force on any atom"),
    ("fnorm", "Total force"),
    ("cellalpha", "Cell angle alpha"),
    ("cellbeta", "Cell angle beta"),
    ("cellgamma", "Cell angle gamma"),
    ("cella", "Cell length a"),
    ("cellb", "Cell length b"),
    ("cellc", "Cell length c"),
];

fn complete_thermo_keywords(
    ast: &Ast,
    offset: usize,
    scope: &[Node],
) -> Vec<CompletionItem> {
    // Only trigger if we're inside a `thermo_style custom` command.
    if !is_inside_thermo_style_custom(ast, scope) {
        return Vec::new();
    }

    let partial = extract_partial_arg(ast.source, offset);
    let mut items = Vec::new();

    for &(kw, doc) in THERMO_KEYWORDS {
        if !partial.is_empty() && !kw.starts_with(partial) {
            continue;
        }
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(doc.to_string()),
            sort_text: Some(format!("1k_{}", kw)),
            ..Default::default()
        });
    }
    items
}

fn is_inside_thermo_style_custom(ast: &Ast, scope: &[Node]) -> bool {
    for node in scope.iter().rev() {
        if node.kind() == "command" {
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    if child.kind() == "command_name" {
                        let name = ast.node_text(child);
                        if name == "thermo_style" {
                            // Check that "custom" appears in the args.
                            let cmd_text = ast.node_text(*node);
                            return cmd_text.contains("custom");
                        }
                    }
                }
            }
        }
    }
    false
}

// ────────────────────────────────────────────────────────────────────
// Expression functions
// ────────────────────────────────────────────────────────────────────

const EXPR_FUNCTIONS: &[(&str, &str)] = &[
    ("abs(x)", "Absolute value"),
    ("acos(x)", "Arc cosine"),
    ("asin(x)", "Arc sine"),
    ("atan(x)", "Arc tangent"),
    ("atan2(y,x)", "Arc tangent of y/x"),
    ("ceil(x)", "Ceiling"),
    ("cos(x)", "Cosine"),
    ("cosh(x)", "Hyperbolic cosine"),
    ("exp(x)", "Exponential"),
    ("floor(x)", "Floor"),
    ("log(x)", "Natural logarithm"),
    ("log10(x)", "Base-10 logarithm"),
    ("max(x,y)", "Maximum"),
    ("min(x,y)", "Minimum"),
    ("mod(x,y)", "Modulo"),
    ("pow(x,y)", "Power"),
    ("round(x)", "Round to nearest integer"),
    ("sign(x)", "Sign of x"),
    ("sin(x)", "Sine"),
    ("sinh(x)", "Hyperbolic sine"),
    ("sqrt(x)", "Square root"),
    ("tan(x)", "Tangent"),
    ("tanh(x)", "Hyperbolic tangent"),
    ("erf(x)", "Error function"),
    ("erfc(x)", "Complementary error function"),
    ("random(x,y,z)", "Random number"),
    ("normal(x,y,z)", "Normal-distributed random"),
    ("ramp(x,y)", "Linear ramp from x to y"),
    ("strcmp(s1,s2)", "String comparison"),
    ("strlen(s)", "String length"),
    ("strfind(s,t)", "Find substring"),
    ("is_file(name)", "Check if file exists"),
    ("extract_setting(name)", "Extract LAMMPS setting"),
    ("is_double(name)", "Check variable is double"),
    ("is_integer(name)", "Check variable is integer"),
    ("is_string(name)", "Check variable is string"),
    ("is_active(category,style,name)", "Check if style is active"),
    ("is_available(category,style,name)", "Check if style is available"),
    ("is_defined(name,mode)", "Check if variable is defined"),
    ("fraction2atom(type,fraction)", "Convert fraction to atom"),
    ("gmask(g)", "Group bitmask"),
    ("gmask2(gname)", "Group bitmask by name"),
    ("gname(g)", "Group name from bitmask"),
    ("gname2(gid)", "Group name from group-ID"),
    ("grpname(idx)", "Group name from index"),
    ("xcm(group,dim)", "Center of mass"),
    ("vcm(group,dim)", "Center of mass velocity"),
    ("fcm(group,dim)", "Center of mass force"),
    ("displace(a,b)", "Distance between atom a and b"),
    ("temperature(dof,ke)", "Compute temperature"),
    ("bondlen(idx)", "Bond length"),
    ("anglen(idx)", "Angle value"),
    ("dihedlen(idx)", "Dihedral value"),
    ("imprlen(idx)", "Improper value"),
];

fn complete_expression_functions(
    ast: &Ast,
    offset: usize,
    scope: &[Node],
) -> Vec<CompletionItem> {
    // Only trigger inside expressions.
    let in_expression = scope.iter().any(|n| n.kind() == "expression");
    if !in_expression {
        return Vec::new();
    }

    let partial = extract_partial_arg(ast.source, offset);
    if partial.is_empty() {
        return Vec::new();
    }

    let mut items = Vec::new();
    for &(sig, doc) in EXPR_FUNCTIONS {
        let name = sig.split('(').next().unwrap_or(sig);
        if !name.starts_with(partial) {
            continue;
        }
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(doc.to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("`{}`\n\n{}", sig, doc),
            })),
            insert_text: Some(sig.to_string()),
            sort_text: Some(format!("1f_{}", sig)),
            ..Default::default()
        });
    }
    items
}

// ────────────────────────────────────────────────────────────────────
// Parameter-driven completions
// ────────────────────────────────────────────────────────────────────

fn complete_parameters(
    ast: &Ast,
    semantic: &SemanticCache,
    offset: usize,
    scope: &[Node],
) -> Vec<CompletionItem> {
    // Find the enclosing command definition.
    let cmd_def = find_enclosing_command_def(ast, scope);
    let Some(cmd_def) = cmd_def else {
        return Vec::new();
    };

    // Determine which argument position the cursor is at.
    let arg_index = estimate_argument_index(ast, scope, offset);

    let mut items = Vec::new();

    // Offer parameter completions based on the position.
    let all_params: Vec<&crate::commands::schema::Parameter> =
        cmd_def.parameters.iter().collect();

    if arg_index < all_params.len() {
        let param = &all_params[arg_index];
        items.extend(complete_for_param_type(param, semantic, ast, offset));
    }

    // Offer the current + next parameter name as snippets (at most 2),
    // instead of dumping all remaining parameters which clutters the UI.
    // For fix/compute commands, skip snippets entirely — the type-based
    // completions are sufficient and parameter-name hints are noisy.
    let is_fix_or_compute = cmd_def.name == "fix" || cmd_def.name == "compute";
    if !is_fix_or_compute {
        let snippet_end = std::cmp::min(arg_index + 2, all_params.len());
        for i in arg_index..snippet_end {
            let param = &all_params[i];
            let prefix = if param.required { "" } else { "[" };
            let suffix = if param.required { "" } else { "]" };
            items.push(CompletionItem {
                label: format!("{}{}{}", prefix, param.name, suffix),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some(param.doc.clone()),
                sort_text: Some(format!("1p_{:03}_{}", i, param.name)),
                ..Default::default()
            });
        }
    }

    items
}

/// Find which CommandDef corresponds to the enclosing command node.
/// Recognises both generic `command` nodes and specialised `fix`/`compute`
/// definition nodes (including ERROR-wrapped incomplete forms).
fn find_enclosing_command_def<'a>(
    ast: &Ast<'a>,
    scope: &[Node<'a>],
) -> Option<&'a CommandDef> {
    // Check for fix / compute — look both in scope and in ERROR wrapper nodes.
    let is_fix =
        scope.iter().any(|n| matches!(n.kind(), "fix" | "fix_id"))
        || scope.iter().any(|n| {
            n.kind() == "ERROR"
                && (0..n.named_child_count()).any(|i| {
                    n.named_child(i).map_or(false, |c| c.kind() == "fix_id")
                })
        });
    let is_compute =
        scope.iter().any(|n| matches!(n.kind(), "compute" | "compute_id"))
        || scope.iter().any(|n| {
            n.kind() == "ERROR"
                && (0..n.named_child_count()).any(|i| {
                    n.named_child(i).map_or(false, |c| c.kind() == "compute_id")
                })
        });
    if is_fix {
        return COMMAND_DB.general_commands.iter().find(|c| c.name == "fix");
    }
    if is_compute {
        return COMMAND_DB.general_commands.iter().find(|c| c.name == "compute");
    }

    // Generic command path.
    for node in scope.iter().rev() {
        if node.kind() == "command" {
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    if child.kind() == "command_name" {
                        let name = ast.node_text(child);
                        return COMMAND_DB
                            .general_commands
                            .iter()
                            .find(|c| c.name == name);
                    }
                }
            }
            break;
        }
    }
    None
}

/// Estimate which parameter index the cursor is at.
///
/// For generic `command` nodes, it counts `args_under` sections past the cursor.
/// For `fix`/`compute` definition nodes, it counts the structured children
/// (`fix_id`, `group_id`, `fix_style`, `args_under`) that end before the cursor.
fn estimate_argument_index(_ast: &Ast, scope: &[Node], offset: usize) -> usize {
    // Check if we're inside a fix / compute context.
    // Also check ERROR wrapper nodes that contain fix_id / compute_id children.
    let in_fix =
        scope.iter().any(|n| matches!(n.kind(), "fix" | "fix_id"))
        || scope.iter().any(|n| {
            n.kind() == "ERROR"
                && (0..n.named_child_count()).any(|i| {
                    n.named_child(i).map_or(false, |c| c.kind() == "fix_id")
                })
        });
    let in_compute =
        scope.iter().any(|n| matches!(n.kind(), "compute" | "compute_id"))
        || scope.iter().any(|n| {
            n.kind() == "ERROR"
                && (0..n.named_child_count()).any(|i| {
                    n.named_child(i).map_or(false, |c| c.kind() == "compute_id")
                })
        });

    if in_fix || in_compute {
        // The enclosing definition may be an ERROR node (partial parse) or
        // a proper fix/compute node.
        let def_node = scope.iter().rev().find(|n| {
            matches!(n.kind(), "fix" | "compute" | "ERROR")
        });
        if let Some(node) = def_node {
            return estimate_definition_arg_index(*node, offset);
        }
    }

    // Fallback: generic command.
    let cmd_node = scope.iter().rev().find(|n| n.kind() == "command");
    if let Some(node) = cmd_node {
        return estimate_generic_arg_index(*node, offset);
    }

    0
}

/// Count arguments for generic `command` nodes (command_name + args_under).
fn estimate_generic_arg_index(cmd_node: Node, offset: usize) -> usize {
    let mut arg_count = 0;
    for i in 0..cmd_node.named_child_count() {
        if let Some(child) = cmd_node.named_child(i) {
            let k = child.kind();
            if k == "command_name" {
                continue;
            }
            // Unwrap args_under container to count individual arguments.
            // Use strict "<" so that partial text the user is still typing
            // is not counted as a completed argument.
            if k == "args_under" {
                for j in 0..child.named_child_count() {
                    if let Some(arg) = child.named_child(j) {
                        if arg.end_byte() < offset {
                            arg_count += 1;
                        } else {
                            return arg_count;
                        }
                    }
                }
            } else if child.end_byte() < offset {
                arg_count += 1;
            } else {
                break;
            }
        }
    }
    arg_count
}

/// Count arguments for `fix`/`compute` definition nodes.
/// The fixed-position children are fix_id / group_id / fix_style (or compute_id / group_id / compute_style),
/// followed by an optional args_under.
fn estimate_definition_arg_index(def_node: Node, offset: usize) -> usize {
    let mut arg_count = 0;
    for i in 0..def_node.named_child_count() {
        if let Some(child) = def_node.named_child(i) {
            let k = child.kind();
            // args_under is a container — skip it, handle its children
            // individually below so that partial args are counted correctly.
            if k == "args_under" {
                for j in 0..child.named_child_count() {
                    if let Some(arg) = child.named_child(j) {
                        if arg.end_byte() <= offset {
                            arg_count += 1;
                        } else {
                            return arg_count + 2; // +2 for fix_id + group_id
                        }
                    }
                }
            } else {
                // fix_id / group_id / fix_style — count if fully typed.
                if child.end_byte() <= offset {
                    arg_count += 1;
                } else {
                    break;
                }
            }
        }
    }
    arg_count
}

/// Generate completions specific to a ParameterType, filtered by the
/// partial text the user has already typed at the cursor.
fn complete_for_param_type(
    param: &crate::commands::schema::Parameter,
    semantic: &SemanticCache,
    ast: &Ast,
    offset: usize,
) -> Vec<CompletionItem> {
    let partial = extract_partial_arg(ast.source, offset);
    let mut items = Vec::new();

    match &param.param_type {
        ParameterType::Enum(values) => {
            for v in values {
                if !partial.is_empty() && !v.starts_with(partial) {
                    continue;
                }
                items.push(CompletionItem {
                    label: v.clone(),
                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                    detail: Some(format!("{}: {}", param.name, param.doc)),
                    sort_text: Some(format!("1e_{}", v)),
                    ..Default::default()
                });
            }
        }
        ParameterType::Boolean => {
            for &val in &[".true.", ".false.", "yes", "no", "on", "off"] {
                if !partial.is_empty() && !val.starts_with(partial) {
                    continue;
                }
                items.push(CompletionItem {
                    label: val.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(format!("{}: {}", param.name, param.doc)),
                    sort_text: Some(format!("1b_{}", val)),
                    ..Default::default()
                });
            }
        }
        ParameterType::Keyword(kw) => {
            if partial.is_empty() || kw.starts_with(partial) {
                items.push(CompletionItem {
                    label: kw.clone(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(format!("{}: {}", param.name, param.doc)),
                    sort_text: Some(format!("1k_{}", kw)),
                    ..Default::default()
                });
            }
        }
        ParameterType::FixId => {
            for name in semantic.fix_defs.keys() {
                if !partial.is_empty() && !name.starts_with(partial) {
                    continue;
                }
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    detail: Some(format!("{}: {}", param.name, param.doc)),
                    sort_text: Some(format!("1i_fix_{}", name)),
                    ..Default::default()
                });
            }
        }
        ParameterType::ComputeId => {
            for name in semantic.compute_defs.keys() {
                if !partial.is_empty() && !name.starts_with(partial) {
                    continue;
                }
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    detail: Some(format!("{}: {}", param.name, param.doc)),
                    sort_text: Some(format!("1i_comp_{}", name)),
                    ..Default::default()
                });
            }
        }
        ParameterType::GroupId => {
            // "all" is the built-in default group.
            if partial.is_empty() || "all".starts_with(partial) {
                items.push(CompletionItem {
                    label: "all".to_string(),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: Some("Built-in group: all atoms".to_string()),
                    sort_text: Some("1g_all".to_string()),
                    ..Default::default()
                });
            }
            for name in semantic.group_defs.keys() {
                if !partial.is_empty() && !name.starts_with(partial) {
                    continue;
                }
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: Some(format!("{}: {}", param.name, param.doc)),
                    sort_text: Some(format!("1g_{}", name)),
                    ..Default::default()
                });
            }
        }
        ParameterType::VariableName => {
            for name in semantic.variable_defs.keys() {
                if !partial.is_empty() && !name.starts_with(partial) {
                    continue;
                }
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(format!("{}: {}", param.name, param.doc)),
                    sort_text: Some(format!("1v_{}", name)),
                    ..Default::default()
                });
            }
        }
        ParameterType::Variable => {
            for name in semantic.variable_defs.keys() {
                if !partial.is_empty() && !name.starts_with(partial) {
                    continue;
                }
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(format!("{}: {}", param.name, param.doc)),
                    sort_text: Some(format!("1v_{}", name)),
                    ..Default::default()
                });
            }
        }
        ParameterType::Label => {
            for name in semantic.labels.keys() {
                if !partial.is_empty() && !name.starts_with(partial) {
                    continue;
                }
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(format!("{}: {}", param.name, param.doc)),
                    sort_text: Some(format!("1l_{}", name)),
                    ..Default::default()
                });
            }
        }
        _ => {
            // For other types (Integer, Float, String, Style, FileName,
            // Expression, Repeat) we just show the parameter hint snippet
            // from complete_parameters above.
        }
    }

    items
}

/// Extract the partial argument text at the cursor for filtering.
/// Returns empty string when cursor is at a whitespace boundary (start of a new word).
fn extract_partial_arg(source: &str, offset: usize) -> &str {
    if offset == 0 {
        return "";
    }
    let before = &source[..offset];
    // If the character just before the cursor is whitespace, the user hasn't
    // started typing the current word yet — return empty so all options show.
    if before.chars().last().map_or(true, |c| c.is_whitespace()) {
        return "";
    }
    let word_start = before
        .rfind(|c: char| c.is_whitespace())
        .map(|i| i + 1)
        .unwrap_or(0);
    before[word_start..].trim_end()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_partial_arg() {
        // offset 24 = end of "step" (cursor past the last char)
        assert_eq!(extract_partial_arg("thermo_style custom step", 24), "step");
        // offset 8 = end of "me"
        assert_eq!(extract_partial_arg("units me", 8), "me");
        // offset 11 = right after "pair_style " (at the space, no word typed yet)
        assert_eq!(extract_partial_arg("pair_style ", 11), "");
    }
}
