use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use crate::ast::Ast;
use crate::semantic::SemanticCache;
use crate::config::DiagnosticsConfig;

pub fn check(
    _ast: &Ast,
    semantic: &SemanticCache,
    _config: &DiagnosticsConfig,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check each variable reference against definitions
    for (ref_name, location) in &semantic.variable_refs {
        // For Underscore refs (v_x, c_x, f_x), prefix determines the lookup namespace
        if ref_name.starts_with("v_") {
            let bare_name = &ref_name[2..]; // strip "v_" prefix
            if !semantic.variable_defs.contains_key(bare_name) {
                diagnostics.push(Diagnostic {
                    range: tower_lsp_server::ls_types::Range {
                        start: tower_lsp_server::ls_types::Position {
                            line: location.line,
                            character: location.character,
                        },
                        end: tower_lsp_server::ls_types::Position {
                            line: location.line,
                            character: location.character + ref_name.len() as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(tower_lsp_server::ls_types::NumberOrString::String(
                        "E002".to_string(),
                    )),
                    source: Some("lammps-lsp".to_string()),
                    message: format!(
                        "E002: 未定义的变量 '{}': 没有找到对应的 variable 定义",
                        ref_name
                    ),
                    ..Default::default()
                });
            }
        } else if ref_name.starts_with("c_") {
            let bare_name = &ref_name[2..]; // strip "c_" prefix
            if !semantic.compute_defs.contains_key(bare_name) {
                diagnostics.push(Diagnostic {
                    range: tower_lsp_server::ls_types::Range {
                        start: tower_lsp_server::ls_types::Position {
                            line: location.line,
                            character: location.character,
                        },
                        end: tower_lsp_server::ls_types::Position {
                            line: location.line,
                            character: location.character + ref_name.len() as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(tower_lsp_server::ls_types::NumberOrString::String(
                        "E002".to_string(),
                    )),
                    source: Some("lammps-lsp".to_string()),
                    message: format!(
                        "E002: 未定义的 compute '{}': 没有找到对应的 compute 定义",
                        ref_name
                    ),
                    ..Default::default()
                });
            }
        } else if ref_name.starts_with("f_") {
            let bare_name = &ref_name[2..]; // strip "f_" prefix
            if !semantic.fix_defs.contains_key(bare_name) {
                diagnostics.push(Diagnostic {
                    range: tower_lsp_server::ls_types::Range {
                        start: tower_lsp_server::ls_types::Position {
                            line: location.line,
                            character: location.character,
                        },
                        end: tower_lsp_server::ls_types::Position {
                            line: location.line,
                            character: location.character + ref_name.len() as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(tower_lsp_server::ls_types::NumberOrString::String(
                        "E002".to_string(),
                    )),
                    source: Some("lammps-lsp".to_string()),
                    message: format!(
                        "E002: 未定义的 fix '{}': 没有找到对应的 fix 定义",
                        ref_name
                    ),
                    ..Default::default()
                });
            }
        } else {
            // Dollar ($x) or Curly (${x}) refs — always look up in variable_defs
            if !semantic.variable_defs.contains_key(ref_name) {
                diagnostics.push(Diagnostic {
                    range: tower_lsp_server::ls_types::Range {
                        start: tower_lsp_server::ls_types::Position {
                            line: location.line,
                            character: location.character,
                        },
                        end: tower_lsp_server::ls_types::Position {
                            line: location.line,
                            character: location.character + ref_name.len() as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(tower_lsp_server::ls_types::NumberOrString::String(
                        "E002".to_string(),
                    )),
                    source: Some("lammps-lsp".to_string()),
                    message: format!(
                        "E002: 未定义的变量 '{}': 没有找到对应的 variable 定义",
                        ref_name
                    ),
                    ..Default::default()
                });
            }
        }
    }

    // Check for duplicate variable definitions (same name, same type)
    let mut seen_vars: std::collections::HashMap<&str, &crate::semantic::SourceLocation> =
        std::collections::HashMap::new();
    for (name, loc) in &semantic.variable_defs {
        if let Some(prev_loc) = seen_vars.get(name.as_str()) {
            let msg = format!(
                "W002: 变量 '{}' 重复定义。首次定义在第 {} 行。",
                name,
                prev_loc.line + 1
            );
            diagnostics.push(Diagnostic {
                range: tower_lsp_server::ls_types::Range {
                    start: tower_lsp_server::ls_types::Position {
                        line: loc.line,
                        character: loc.character,
                    },
                    end: tower_lsp_server::ls_types::Position {
                        line: loc.line,
                        character: loc.character + name.len() as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(tower_lsp_server::ls_types::NumberOrString::String(
                    "W002".to_string(),
                )),
                source: Some("lammps-lsp".to_string()),
                message: msg,
                ..Default::default()
            });
        } else {
            seen_vars.insert(name.as_str(), loc);
        }
    }

    diagnostics
}
