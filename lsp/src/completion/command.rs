use tower_lsp_server::ls_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails,
    Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};
use tree_sitter::Node;
use crate::ast::Ast;
use crate::commands::schema::CommandCategory;
use crate::commands::COMMAND_DB;

/// Provide command-name completions when the cursor is at a position
/// where a LAMMPS command name is expected.
pub fn complete_commands(ast: &Ast, offset: usize, scope: &[Node]) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // Determine whether we're in a command-name context.
    // Heuristics:
    //   1. The deepest scope node is `command_name`          → already typing a command
    //   2. The deepest scope node is `command` and the cursor is near its start
    //   3. The cursor is at the very start of a line (possibly after a `&` continuation)
    let is_command_context = scope.iter().any(|n| {
        let k = n.kind();
        k == "command_name"
            || k == "command"
            || k == "input_script"
    });

    if !is_command_context {
        let text_before = &ast.source[..offset];
        let is_line_start = text_before.is_empty()
            || text_before.ends_with('\n')
            || text_before.ends_with("&\n");
        if !is_line_start {
            return items;
        }
    }

    // Extract the partial command name the user has typed so far (if any).
    let partial = extract_partial_word(ast.source, offset);

    for cmd in COMMAND_DB.general_commands.iter() {
        if !partial.is_empty() && !cmd.name.starts_with(partial) {
            continue;
        }

        let sort_prefix = match cmd.category {
            CommandCategory::Setup => "0a",
            CommandCategory::Simulation => "0b",
            CommandCategory::Output => "0c",
            CommandCategory::Control => "0d",
            CommandCategory::Input => "0e",
        };

        items.push(CompletionItem {
            label: cmd.name.clone(),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(cmd.doc_short.clone()),
                description: Some(format!("{:?}", cmd.category)),
            }),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(build_signature(cmd)),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: cmd.doc_full.clone(),
            })),
            insert_text: Some(build_snippet(&cmd.name, cmd)),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            sort_text: Some(format!("{}_{}", sort_prefix, cmd.name)),
            ..Default::default()
        });
    }

    items
}

/// Extract the partial word before `offset` for filtering.
fn extract_partial_word(source: &str, offset: usize) -> &str {
    let before = &source[..offset];
    let word_start = before
        .rfind(|c: char| c.is_whitespace() || c == '&')
        .map(|i| i + 1)
        .unwrap_or(0);
    before[word_start..].trim_end()
}

/// Build a human-readable signature string, e.g. "pair_style style [args]".
fn build_signature(cmd: &crate::commands::schema::CommandDef) -> String {
    let params: Vec<String> = cmd
        .parameters
        .iter()
        .map(|p| {
            if p.required {
                format!("<{}>", p.name)
            } else {
                format!("[{}]", p.name)
            }
        })
        .collect();
    if params.is_empty() {
        cmd.name.clone()
    } else {
        format!("{} {}", cmd.name, params.join(" "))
    }
}

/// Build a snippet string with tabstops for required parameters.
fn build_snippet(name: &str, cmd: &crate::commands::schema::CommandDef) -> String {
    let required: Vec<String> = cmd
        .parameters
        .iter()
        .filter(|p| p.required)
        .enumerate()
        .map(|(i, p)| format!("${{{}:{}}}", i + 1, p.name))
        .collect();
    if required.is_empty() {
        format!("{} ", name)
    } else {
        format!("{} {}", name, required.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_partial_word_empty() {
        assert_eq!(extract_partial_word("", 0), "");
    }

    #[test]
    fn test_extract_partial_word_simple() {
        assert_eq!(extract_partial_word("pair", 4), "pair");
    }

    #[test]
    fn test_extract_partial_word_after_space() {
        assert_eq!(extract_partial_word("pair_style eam\npai", 17), "pai");
    }
}
