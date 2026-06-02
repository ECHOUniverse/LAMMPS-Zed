pub mod command;
pub mod style;
pub mod variable;
pub mod argument;

use tower_lsp_server::ls_types::{
    CompletionItem, CompletionList, CompletionResponse, Position,
};
use crate::ast::Ast;
use crate::semantic::SemanticCache;

/// Run completion at a given position in a document.
///
/// This is the main entry point for `textDocument/completion` requests.
/// It classifies the cursor context and delegates to the appropriate
/// sub-module:
///
/// | Context               | Module           | sort_text prefix |
/// |-----------------------|------------------|------------------|
/// | Command name          | `command`        | `0_`             |
/// | Style name            | `style`          | `1_`             |
/// | Variable / ID ref     | `variable`       | `2_` / `3_`      |
/// | Arguments / keywords  | `argument`       | `1_`             |
pub fn run_completion(
    ast: &Ast,
    semantic: &SemanticCache,
    position: Position,
) -> Option<CompletionResponse> {
    let byte_offset = position_to_byte_offset(ast.source, position);

    // Find the node and ancestor chain at the cursor.
    // Note: `node_at_offset` always returns Some(...) as long as offset is
    // within the source span (the root node is the fallback).
    let _node = ast.node_at_offset(byte_offset)?;
    let scope = ast.scope_at_offset(byte_offset);

    let mut items: Vec<CompletionItem> = Vec::new();

    // Command-name completions.
    items.extend(command::complete_commands(ast, byte_offset, &scope));

    // Style-name completions (fix_style, compute_style, pair/bond/angle/...).
    items.extend(style::complete_styles(ast, byte_offset, &scope));

    // Variable / ID reference completions ($, ${, v_, c_, f_).
    items.extend(variable::complete_variables(ast, semantic, byte_offset, &scope));

    // Argument / parameter / keyword / expression completions.
    items.extend(argument::complete_arguments(ast, semantic, byte_offset, &scope));

    if items.is_empty() {
        return None;
    }

    Some(CompletionResponse::List(CompletionList {
        is_incomplete: false,
        items,
    }))
}

/// Convert an LSP Position (line, character) to a byte offset into
/// the source text. Handles multi-byte UTF-8 characters correctly
/// by iterating over `char_indices`.
fn position_to_byte_offset(source: &str, position: Position) -> usize {
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

    // Position beyond end → clamp to source length.
    source.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_to_byte_offset_start() {
        assert_eq!(
            position_to_byte_offset("hello world", Position { line: 0, character: 0 }),
            0
        );
    }

    #[test]
    fn test_position_to_byte_offset_mid() {
        assert_eq!(
            position_to_byte_offset("hello world", Position { line: 0, character: 6 }),
            6
        );
    }

    #[test]
    fn test_position_to_byte_offset_newline() {
        assert_eq!(
            position_to_byte_offset("a\nb\nc", Position { line: 1, character: 0 }),
            2
        );
    }

    #[test]
    fn test_position_to_byte_offset_beyond() {
        assert_eq!(
            position_to_byte_offset("abc", Position { line: 10, character: 0 }),
            3
        );
    }

    // ── Integration-style completion tests ──────────────────────────

    /// Helper: parse source and run completion at (line, char).
    fn complete_at(source: &str, line: u32, character: u32) -> Vec<String> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_lammps::LANGUAGE.into())
            .expect("load grammar");
        let tree = parser.parse(source, None).expect("parse");
        let ast = crate::ast::Ast::new(source, &tree);
        let semantic = crate::semantic::SemanticCache::build(&ast, &"test://input.in".to_string());
        let pos = Position { line, character };
        match run_completion(&ast, &semantic, pos) {
            Some(CompletionResponse::List(list)) => {
                list.items.iter().map(|i| i.label.clone()).collect()
            }
            _ => vec![],
        }
    }

    /// Helper: check that none of the completions are the forbidden word.
    fn assert_no_item(source: &str, line: u32, character: u32, forbidden: &str) {
        let items = complete_at(source, line, character);
        for item in &items {
            assert_ne!(
                item, forbidden,
                "completion '{}' should not appear at ({}, {}) in:\n{}\nitems: {:?}",
                forbidden, line, character, source, items
            );
        }
    }

    #[test]
    fn test_no_angle_completion_for_bare_all() {
        // Typing "all" at start of a fresh file.
        assert_no_item("all", 0, 3, "angle");
    }

    #[test]
    fn test_no_angle_completion_for_all_inside_atom_style() {
        // "all" is not a valid atom_style value, should not produce "angle".
        assert_no_item("atom_style all", 0, 14, "angle");
    }

    #[test]
    fn test_atom_style_partial_a_shows_atomic() {
        // "a" is a partial — "atomic" should appear since it starts with "a".
        let items = complete_at("atom_style a", 0, 12);
        assert!(
            items.contains(&"atomic".to_string()),
            "Expected 'atomic' to appear for 'atom_style a'; got {:?}",
            items
        );
        // "angle" is NOT a valid atom_style value — regression test for the bug.
        assert!(
            !items.contains(&"angle".to_string()),
            "Bug: 'angle' should NOT appear as an atom_style completion; got {:?}",
            items
        );
    }

    #[test]
    fn test_no_angle_completion_for_all_as_command() {
        // "all" as a command name (shouldn't trigger style completions).
        assert_no_item("all", 0, 3, "angle");
    }

    #[test]
    fn test_no_angle_completion_for_all_on_new_line() {
        // "all" on a new line after some other content.
        assert_no_item("units lj\nall", 1, 3, "angle");
    }

    #[test]
    fn test_no_angle_completion_for_all_in_special_bonds() {
        // "all" inside special_bonds (which mentions lg/coul/angle/dihedral in docs).
        assert_no_item("special_bonds all", 0, 17, "angle");
    }

    // ── fix-command regression tests ─────────────────────────────

    #[test]
    fn test_fix_command_no_spurious_command_names() {
        // 穷举 cursor 位置 — 跳过 pos 0（行首允许显示命令补全）
        for pos in 1..=6 {
            let items = complete_at("fix 1", 0, pos as u32);
            let has_wall = items.iter().any(|i| i == "wall/lj126");
            let has_atom = items.iter().any(|i| i.contains("atom_style"));
            if has_wall || has_atom {
                panic!("BUG pos {}: wall={} atom={} items={:?}", pos, has_wall, has_atom, items);
            }
        }
        for pos in 1..=10 {
            let items = complete_at("fix 1 all", 0, pos as u32);
            let has_atom = items.iter().any(|i| i.contains("atom_style"));
            let has_angle = items.iter().any(|i| i.contains("angle_style"));
            if has_atom || has_angle {
                panic!("BUG pos {}: atom={} angle={} items={:?}", pos, has_atom, has_angle, items);
            }
        }
        // 真实场景：已有完整 fix 命令的文件，新行键入 fix 1
        {
            // L7 is "fix 1" (after blank line L6)
            let source = "units metal\ndimension 3\natom_style atomic\n\
                          fix 1 all npt temp 300 300 0.1 iso 1.0 1.0 1.0\n\
                          fix 2 all nve\n\
                          compute myTemp all temp\n\nfix 1";
            for col in 1..=5 {
                let items = complete_at(source, 7, col as u32);
                let has_wall = items.iter().any(|i| i == "wall/lj126");
                assert!(!has_wall,
                    "REAL BUG col {}: wall/lj126 leaked! items={:?}", col, items);
            }
        }
    }

    #[test]
    fn test_fix_command_with_all_no_spurious() {
        // "fix 1 all" with cursor after "all" (pos 9 = end of input)
        let items = complete_at("fix 1 all", 0, 9);
        eprintln!("fix 1 all (pos 9) completions: {:?}", items);
        for forbidden in &["angle_style", "angle_coeff", "angle_write", "fix"] {
            assert!(
                !items.contains(&forbidden.to_string()),
                "Bug: '{}' should not appear for 'fix 1 all'; got {:?}",
                forbidden, items
            );
        }
    }

    #[test]
    fn test_fix_trailing_space_diagnostic() {
        // "fix 1 " with trailing space — cursor after the space
        let items = complete_at("fix 1 ", 0, 6);
        eprintln!("fix 1_ (pos 6) completions: {:?}", items);
    }

    #[test]
    fn test_fix_with_content_before() {
        // Simulate having content before the fix line
        let source = "units lj\nfix 1";
        let items = complete_at(source, 1, 5);
        eprintln!("after units\\nfix 1 (pos 1,5) completions: {:?}", items);
    }

    #[test]
    fn test_atom_style_filtering_works() {
        // "sp" is a partial — "sphere" and "spin" should appear.
        let items = complete_at("atom_style sp", 0, 13);
        assert!(
            items.contains(&"sphere".to_string()),
            "Expected 'sphere' for 'atom_style sp'; got {:?}",
            items
        );
        assert!(
            items.contains(&"spin".to_string()),
            "Expected 'spin' for 'atom_style sp'; got {:?}",
            items
        );
        // "angle" is not a valid atom_style — regression check.
        assert!(
            !items.contains(&"angle".to_string()),
            "Bug: 'angle' should NOT appear as atom_style; got {:?}",
            items
        );
    }

    // ── ID 补全回归测试 ──────────────────────────────────────

    #[test]
    fn test_fix_id_completion_from_previous_defs() {
        // fix 1 all nve 在文件前面定义 → fix_modify 的 ID 参数应补全 "1"
        let source = "fix 1 all nve\nfix_modify 1";
        let items = complete_at(source, 1, 12); // cursor at "1"
        eprintln!("fix_modify 1 completion items: {:?}", items);
        assert!(
            items.contains(&"1".to_string()),
            "Expected fix ID '1' to appear for fix_modify; got {:?}",
            items
        );
    }

    #[test]
    fn test_fix_id_completion_with_partial() {
        // fix 1 all nve, fix 2 all npt -> fix_modify 2| 应补全 "2"
        let source = "fix 1 all nve\nfix 2 all npt temp 300 300 0.1 iso 1 1 1\nfix_modify 2";
        let items = complete_at(source, 2, 12); // cursor at "2"
        eprintln!("fix_modify 2 completion items: {:?}", items);
        assert!(
            items.contains(&"2".to_string()),
            "Expected fix ID '2' to appear for fix_modify 2; got {:?}",
            items
        );
    }

    #[test]
    fn test_compute_id_completion_from_previous_defs() {
        // compute myTemp all temp → compute_modify 应补全 "myTemp"
        let source = "compute myTemp all temp\ncompute_modify m";
        let items = complete_at(source, 1, 15); // cursor at "m"
        eprintln!("compute_modify m completion items: {:?}", items);
        assert!(
            items.contains(&"myTemp".to_string()),
            "Expected compute ID 'myTemp'; got {:?}",
            items
        );
    }

    #[test]
    fn test_group_id_completion_includes_all() {
        // fix 命令的 group-ID 位置应补全 "all"（内置）+ 自定义 group
        let source = "group mobile type 1\nfix 1 a";
        let items = complete_at(source, 1, 6); // cursor at "a" (partial)
        assert!(
            items.contains(&"all".to_string()),
            "Expected built-in group 'all'; got {:?}",
            items
        );
        assert!(
            items.contains(&"mobile".to_string()),
            "Expected custom group 'mobile'; got {:?}",
            items
        );
    }

    #[test]
    fn test_group_id_completion_filters_partial() {
        // 只应补全匹配 "mo" 的 group
        let source = "group mobile type 1\ngroup fixed type 2\nfix 1 mo";
        let items = complete_at(source, 2, 7); // cursor at "o" (inside partial)
        eprintln!("fix 1 mo (pos 2,7) group-id completion items: {:?}", items);
        assert!(items.contains(&"mobile".to_string()));
        assert!(!items.contains(&"fixed".to_string()));
    }

    #[test]
    fn test_label_completion_from_previous_labels() {
        // jump SELF <label> → 应补全已定义 label
        let source = "label high_temp\nlabel low_temp\njump SELF h";
        let items = complete_at(source, 2, 10); // cursor at "h"
        eprintln!("jump SELF h label completion items: {:?}", items);
        assert!(
            items.contains(&"high_temp".to_string()),
            "Expected label 'high_temp'; got {:?}",
            items
        );
    }

    #[test]
    fn test_parameter_snippet_limit() {
        // fix 命令有 4 个参数: ID, group-ID, style, args
        // 当光标在 fix 1 all nvt 之后，只应补全当前+下个参数
        // 不应出现全部剩余参数的长列表
        let source = "fix 1 all nvt 300";
        let items = complete_at(source, 0, 15); // cursor at "3"
        eprintln!("fix 1 all nvt 3 completion items: {:?}", items);

        // 不应再显示 ID 和 group-ID 的参数提示
        let param_hints: Vec<&String> = items.iter()
            .filter(|i| *i == "ID" || *i == "group-ID")
            .collect();
        assert!(
            param_hints.is_empty(),
            "Should not show ID/group-ID hints at args position; got {:?}",
            items
        );
    }

    #[test]
    fn test_fix_1_does_not_trigger_styles() {
        // "fix 1" — user just typed fix ID, haven't reached group or style yet.
        // Fix style names like wall/lj126 must NOT appear.
        for pos in 1..=5 {
            let items = complete_at("fix 1", 0, pos as u32);
            let has_wall = items.iter().any(|i| i.contains("wall"));
            assert!(!has_wall,
                "BUG: fix style leaked at pos {}: items={:?}", pos, items);
        }
        // Also test with trailing space
        for pos in 1..=6 {
            let items = complete_at("fix 1 ", 0, pos as u32);
            let has_wall = items.iter().any(|i| i.contains("wall"));
            assert!(!has_wall,
                "BUG: fix style leaked with trailing space at pos {}: items={:?}", pos, items);
        }
    }

    #[test]
    fn test_full_input_no_style_leak_at_fix_id() {
        // 模拟 input.in 中有 fix 命令，新行键入 fix 1 不应触发 style
        let source = "units metal\ndimension 3\natom_style atomic\n\
                      fix 1 all npt temp 300 300 0.1 iso 1.0 1.0 1.0\n\
                      fix 2 all nve\n\
                      compute myTemp all temp\n\nfix 1";
        let last_line = 7u32;
        for col in 1..=5 {
            let items = complete_at(source, last_line, col as u32);
            let has_wall = items.iter().any(|i| i.contains("wall"));
            assert!(!has_wall,
                "BUG at line {} col {}: fix style leaked! items={:?}",
                last_line, col, items);
        }
    }

    #[test]
    fn test_unfix_id_completion() {
        // unfix ID → 应补全已定义的 fix ID
        let source = "fix myNVE all nve\nunfix my";
        let items = complete_at(source, 1, 7); // cursor at "y" (inside "my")
        assert!(
            items.contains(&"myNVE".to_string()),
            "Expected fix ID 'myNVE' for unfix; got {:?}",
            items
        );
    }
}
