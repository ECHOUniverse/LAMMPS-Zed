use tree_sitter::{Parser, Tree, InputEdit, Point};

pub struct ParserState {
    pub source: String,
    pub tree: Tree,
    parser: Parser,
}

impl ParserState {
    pub fn new(source: &str) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_lammps::LANGUAGE.into())
            .expect("Failed to load tree-sitter-lammps grammar");

        let tree = parser.parse(source, None)
            .expect("Failed to parse document");

        Self {
            source: source.to_string(),
            tree,
            parser,
        }
    }

    pub fn apply_edit(
        &mut self,
        start_line: usize,
        start_char: usize,
        end_line: usize,
        end_char: usize,
        new_text: &str,
    ) {
        // Convert line/char to byte offsets
        let start_byte = self.line_char_to_byte(start_line, start_char);
        let old_end_byte = self.line_char_to_byte(end_line, end_char);

        // Apply edit to source text
        let old_text = &self.source[start_byte..old_end_byte];
        let _old_text_len = old_text.len();

        self.source.replace_range(start_byte..old_end_byte, new_text);

        // Build tree-sitter edit
        let edit = InputEdit {
            start_byte,
            old_end_byte,
            new_end_byte: start_byte + new_text.len(),
            start_position: Point::new(start_line, start_char),
            old_end_position: Point::new(end_line, end_char),
            new_end_position: Point::new(
                start_line,
                start_char + new_text.len(),
            ),
        };

        // Apply incremental edit
        self.tree.edit(&edit);

        // Re-parse
        if let Some(new_tree) = self.parser.parse(&self.source, Some(&self.tree)) {
            self.tree = new_tree;
        } else {
            // Fallback: full re-parse
            self.tree = self.parser.parse(&self.source, None)
                .expect("Failed to re-parse document");
        }

        // Invalidate semantic cache
        // (Will be managed in server.rs)
    }

    fn line_char_to_byte(&self, line: usize, character: usize) -> usize {
        let mut byte_offset = 0;
        let mut current_line = 0;
        let mut current_char = 0;

        for (i, ch) in self.source.char_indices() {
            if current_line == line && current_char == character {
                return i;
            }
            if ch == '\n' {
                current_line += 1;
                current_char = 0;
                byte_offset = i + 1;
            } else {
                current_char += 1;
            }
        }

        // If we've reached end of file, return the last byte
        if current_line == line {
            self.source.len()
        } else {
            byte_offset
        }
    }
}
