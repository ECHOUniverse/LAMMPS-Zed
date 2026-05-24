use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity};
use crate::ast::Ast;
use crate::config::DiagnosticsConfig;

use super::make_diagnostic;

pub fn check(ast: &Ast, _config: &DiagnosticsConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for target in ast.include_targets() {
        // Check if the file exists (simple path check)
        // Note: Full IncludeResolver integration requires workspace info.
        // For now, do a basic check: if the path looks like a relative path
        // without wildcards, check that it has expected format.
        let path = target.file_path.trim();

        if path.is_empty() {
            diagnostics.push(make_diagnostic(
                DiagnosticSeverity::ERROR,
                "E003",
                &format!(
                    "E003: {} 目标文件路径为空",
                    if target.is_jump { "jump" } else { "include" }
                ),
                target.node.start_byte(),
                target.node.end_byte(),
                ast.source,
            ));
            continue;
        }

        // Warn about wildcard includes (resolve at runtime)
        if path.contains('*') || path.contains('?') {
            diagnostics.push(make_diagnostic(
                DiagnosticSeverity::INFORMATION,
                "I001",
                &format!(
                    "include 文件路径包含通配符 '{}'。文件存在性检查将跳过。",
                    path
                ),
                target.node.start_byte(),
                target.node.end_byte(),
                ast.source,
            ));
            continue;
        }

        // Check for common path issues
        if path.contains("..") {
            diagnostics.push(make_diagnostic(
                DiagnosticSeverity::INFORMATION,
                "I002",
                &format!(
                    "include 路径使用相对父目录: '{}'。请确认路径正确。",
                    path
                ),
                target.node.start_byte(),
                target.node.end_byte(),
                ast.source,
            ));
        }

        // If this is a jump with a label, note that we can't fully verify the label
        // without resolving the target file
        if target.is_jump && target.label.is_some() {
            let label_name = target.label.as_ref().unwrap();
            diagnostics.push(make_diagnostic(
                DiagnosticSeverity::INFORMATION,
                "I003",
                &format!(
                    "jump 到 label '{}' 在文件 '{}' 中。请确认目标文件中存在此 label。",
                    label_name,
                    path
                ),
                target.node.start_byte(),
                target.node.end_byte(),
                ast.source,
            ));
        }
    }

    diagnostics
}
