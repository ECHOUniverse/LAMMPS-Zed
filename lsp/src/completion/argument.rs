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
    _semantic: &SemanticCache,
    offset: usize,
    scope: &[Node],
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // ── 1. Enumerate-style keyword completions ──────────────────────
    items.extend(complete_thermo_keywords(ast, offset, scope));
    items.extend(complete_units_enum(ast, offset, scope));
    items.extend(complete_boundary_enum(ast, offset, scope));
    items.extend(complete_atom_style_enum(ast, offset, scope));
    items.extend(complete_dimension_enum(ast, offset, scope));

    // ── 2. Expression function completions ─────────────────────────
    items.extend(complete_expression_functions(ast, offset, scope));

    // ── 3. Parameter-driven completions for known commands ─────────
    items.extend(complete_parameters(ast, offset, scope));

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
// Enum completions
// ────────────────────────────────────────────────────────────────────

fn complete_enum_option(
    ast: &Ast,
    offset: usize,
    scope: &[Node],
    command_name: &str,
    values: &[(&str, &str)],
    sort_prefix: &str,
) -> Vec<CompletionItem> {
    if !is_command_in_scope(ast, scope, command_name) {
        return Vec::new();
    }

    let partial = extract_partial_arg(ast.source, offset);
    // Don't complete if we're on the command name itself.
    if partial == command_name {
        return Vec::new();
    }

    let mut items = Vec::new();
    for &(val, doc) in values {
        if !partial.is_empty() && !val.starts_with(partial) {
            continue;
        }
        items.push(CompletionItem {
            label: val.to_string(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some(doc.to_string()),
            sort_text: Some(format!("{}_{}", sort_prefix, val)),
            ..Default::default()
        });
    }
    items
}

fn complete_units_enum(
    ast: &Ast,
    offset: usize,
    scope: &[Node],
) -> Vec<CompletionItem> {
    complete_enum_option(
        ast,
        offset,
        scope,
        "units",
        &[
            ("lj", "Lennard-Jones reduced units"),
            ("real", "Real units (kcal/mol, Angstroms)"),
            ("metal", "Metal units (eV, Angstroms)"),
            ("si", "SI units (Joules, meters)"),
            ("cgs", "CGS units (ergs, cm)"),
            ("electron", "Electron units (Hartree, Bohr)"),
            ("micro", "Micro units (pg, microns, microseconds)"),
            ("nano", "Nano units (ag, nm, ps)"),
        ],
        "1u",
    )
}

fn complete_boundary_enum(
    ast: &Ast,
    offset: usize,
    scope: &[Node],
) -> Vec<CompletionItem> {
    complete_enum_option(
        ast,
        offset,
        scope,
        "boundary",
        &[
            ("p", "Periodic"),
            ("f", "Fixed"),
            ("s", "Shrink-wrapped"),
            ("m", "Shrink-wrapped with minimum value"),
        ],
        "1u",
    )
}

fn complete_atom_style_enum(
    ast: &Ast,
    offset: usize,
    scope: &[Node],
) -> Vec<CompletionItem> {
    complete_enum_option(
        ast,
        offset,
        scope,
        "atom_style",
        &[
            ("atomic", "Point particles"),
            ("charge", "Point particles with charge"),
            ("sphere", "Spherical particles"),
            ("bond", "Bonded particles"),
            ("angle", "Angled particles"),
            ("full", "Molecular particles with charge"),
            ("molecular", "Uncharged molecular particles"),
            ("hybrid", "Multiple atom styles"),
            ("body", "Rigid body particles"),
            ("dipole", "Point particles with dipole moment"),
            ("dpd", "Dissipative particle dynamics"),
            ("ellipsoid", "Ellipsoidal particles"),
            ("line", "Line segment particles"),
            ("meso", "Smoothed particle hydrodynamics"),
            ("peri", "Peridynamic particles"),
            ("smd", "Smooth mach dynamics"),
            ("spin", "SPIN particles"),
            ("template", "Template-based particles"),
            ("tri", "Triangular particles"),
            ("wavepacket", "Wave packets"),
        ],
        "1u",
    )
}

fn complete_dimension_enum(
    ast: &Ast,
    offset: usize,
    scope: &[Node],
) -> Vec<CompletionItem> {
    complete_enum_option(
        ast,
        offset,
        scope,
        "dimension",
        &[("2", "2-dimensional simulation"), ("3", "3-dimensional simulation")],
        "1u",
    )
}

fn is_command_in_scope(ast: &Ast, scope: &[Node], target: &str) -> bool {
    for node in scope.iter().rev() {
        if node.kind() == "command" {
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    if child.kind() == "command_name" && ast.node_text(child) == target {
                        return true;
                    }
                }
            }
            // Only check the innermost command.
            break;
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
        items.extend(complete_for_param_type(param, ast, offset));
    }

    // Also offer all remaining parameter names as snippets.
    for (i, param) in all_params.iter().enumerate() {
        if i < arg_index {
            continue;
        }
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

    items
}

/// Find which CommandDef corresponds to the enclosing command node.
fn find_enclosing_command_def<'a>(
    ast: &Ast<'a>,
    scope: &[Node<'a>],
) -> Option<&'a CommandDef> {
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
fn estimate_argument_index(_ast: &Ast, scope: &[Node], offset: usize) -> usize {
    // Find the enclosing command node.
    let cmd_node = scope.iter().rev().find(|n| n.kind() == "command");
    let Some(cmd_node) = cmd_node else {
        return 0;
    };

    // Count arguments before the cursor position.
    // The first named child is `command_name`, subsequent ones are args.
    let mut arg_count = 0;
    for i in 0..cmd_node.named_child_count() {
        if let Some(child) = cmd_node.named_child(i) {
            let k = child.kind();
            // Skip the command_name itself.
            if k == "command_name" {
                continue;
            }
            // If this argument ends before the cursor, count it.
            if child.end_byte() <= offset {
                arg_count += 1;
            } else {
                break;
            }
        }
    }
    arg_count
}

/// Generate completions specific to a ParameterType.
fn complete_for_param_type(
    param: &crate::commands::schema::Parameter,
    _ast: &Ast,
    _offset: usize,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    match &param.param_type {
        ParameterType::Enum(values) => {
            for v in values {
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
            items.push(CompletionItem {
                label: kw.clone(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(format!("{}: {}", param.name, param.doc)),
                sort_text: Some(format!("1k_{}", kw)),
                ..Default::default()
            });
        }
        _ => {
            // For other types (Integer, Float, String, etc.) we just show the
            // parameter hint. The snippet-based completion (shown in
            // complete_parameters above) is sufficient.
        }
    }

    items
}

/// Extract the partial argument text at the cursor for filtering.
fn extract_partial_arg(source: &str, offset: usize) -> &str {
    let before = &source[..offset];
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
        assert_eq!(extract_partial_arg("thermo_style custom step", 23), "step");
        assert_eq!(extract_partial_arg("units me", 8), "me");
        assert_eq!(extract_partial_arg("pair_style ", 11), "");
    }
}
