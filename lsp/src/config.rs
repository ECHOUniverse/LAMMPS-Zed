use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspConfig {
    #[serde(default)]
    pub diagnostics: DiagnosticsConfig,
    #[serde(default)]
    pub completion: CompletionConfig,
    #[serde(default)]
    pub formatting: FormattingConfig,
    #[serde(default)]
    pub hover: HoverConfig,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            diagnostics: DiagnosticsConfig::default(),
            completion: CompletionConfig::default(),
            formatting: FormattingConfig::default(),
            hover: HoverConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_true")]
    pub unknown_command: bool,
    #[serde(default = "default_true")]
    pub undefined_variable: bool,
    #[serde(default = "default_true")]
    pub include_file: bool,
    #[serde(default = "default_true")]
    pub argument_count: bool,
    #[serde(default = "default_true")]
    pub expression_errors: bool,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            enable: true,
            unknown_command: true,
            undefined_variable: true,
            include_file: true,
            argument_count: true,
            expression_errors: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_true")]
    pub snippet_support: bool,
}

impl Default for CompletionConfig {
    fn default() -> Self {
        Self {
            enable: true,
            snippet_support: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingConfig {
    #[serde(default = "default_indent")]
    pub indent_size: u8,
    pub max_line_length: Option<usize>,
    /// Whether continuation lines align with the argument column (true)
    /// or use a simple indent of indent_size (false).
    #[serde(default = "default_true")]
    pub align_continuations: bool,
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            indent_size: 2,
            max_line_length: None,
            align_continuations: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverConfig {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_true")]
    pub doc_links: bool,
}

impl Default for HoverConfig {
    fn default() -> Self {
        Self {
            enable: true,
            doc_links: true,
        }
    }
}

fn default_true() -> bool { true }
fn default_indent() -> u8 { 2 }
