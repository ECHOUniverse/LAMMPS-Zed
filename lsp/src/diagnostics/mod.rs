mod check_command;
mod check_variable;
mod check_include;
mod check_expression;

use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use crate::ast::Ast;
use crate::semantic::SemanticCache;
use crate::config::DiagnosticsConfig;

/// Run all diagnostic checks and return a list of diagnostics.
pub fn run_diagnostics(
    ast: &Ast,
    semantic: &SemanticCache,
    config: &DiagnosticsConfig,
    _uri: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if !config.enable {
        return diagnostics;
    }

    if config.unknown_command || config.argument_count {
        diagnostics.extend(check_command::check(ast, config));
    }
    if config.undefined_variable {
        diagnostics.extend(check_variable::check(ast, semantic, config));
    }
    if config.include_file {
        diagnostics.extend(check_include::check(ast, config));
    }
    if config.expression_errors {
        diagnostics.extend(check_expression::check(ast, semantic, config));
    }

    diagnostics
}

/// Create a diagnostic from a byte range.
pub fn make_diagnostic(
    severity: DiagnosticSeverity,
    code: &str,
    message: &str,
    start_byte: usize,
    end_byte: usize,
    source_text: &str,
) -> Diagnostic {
    let range = byte_range_to_lsp_range(start_byte, end_byte, source_text);

    Diagnostic {
        range,
        severity: Some(severity),
        code: Some(tower_lsp_server::ls_types::NumberOrString::String(code.to_string())),
        code_description: None,
        source: Some("lammps-lsp".to_string()),
        message: message.to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Convert byte offsets to an LSP Range.
fn byte_range_to_lsp_range(start_byte: usize, end_byte: usize, source: &str) -> Range {
    let (start_line, start_char) = crate::ast::byte_to_line_char(source, start_byte);
    let (end_line, end_char) = crate::ast::byte_to_line_char(source, end_byte);

    Range {
        start: Position {
            line: start_line as u32,
            character: start_char as u32,
        },
        end: Position {
            line: end_line as u32,
            character: end_char as u32,
        },
    }
}
