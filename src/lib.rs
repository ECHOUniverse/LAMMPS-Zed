use zed_extension_api as zed;

struct LammpsExtension {
    /// Lazily resolved path to the lammps-lsp binary.
    cached_binary_path: Option<String>,
}

impl zed::Extension for LammpsExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        // Strategy 1: Use user-configured binary path from Zed settings
        let binary_path = zed::settings::LspSettings::for_worktree(
            language_server_id.as_ref(),
            worktree,
        )
        .ok()
        .and_then(|settings| settings.binary)
        .and_then(|binary| binary.path);

        if let Some(path) = binary_path {
            self.cached_binary_path = Some(path);
        }

        // Strategy 2: Search for lammps-lsp on system PATH
        if self.cached_binary_path.is_none() {
            if let Some(path) = worktree.which("lammps-lsp") {
                self.cached_binary_path = Some(path);
            }
        }

        // Strategy 3: Error if not found
        match &self.cached_binary_path {
            Some(path) => Ok(zed::Command {
                command: path.clone(),
                args: vec!["--stdio".to_string()],
                env: Default::default(),
            }),
            None => Err(format!(
                "lammps-lsp binary not found. Install with 'cargo install lammps-lsp' \
                 or configure the path in Zed settings under lsp.lammps.binary.path"
            )),
        }
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<Option<serde_json::Value>> {
        let settings = zed::settings::LspSettings::for_worktree(
            language_server_id.as_ref(),
            worktree,
        )
        .ok()
        .and_then(|s| s.settings.clone())
        .unwrap_or_default();

        Ok(Some(serde_json::json!({
            "diagnostics": {
                "enable": settings.get("diagnostics.enable").unwrap_or(&serde_json::Value::Bool(true)),
                "unknown_command": settings.get("diagnostics.unknown_command").unwrap_or(&serde_json::Value::Bool(true)),
                "undefined_variable": settings.get("diagnostics.undefined_variable").unwrap_or(&serde_json::Value::Bool(true)),
                "include_file": settings.get("diagnostics.include_file").unwrap_or(&serde_json::Value::Bool(true)),
                "argument_count": settings.get("diagnostics.argument_count").unwrap_or(&serde_json::Value::Bool(true)),
                "expression_errors": settings.get("diagnostics.expression_errors").unwrap_or(&serde_json::Value::Bool(true)),
            },
            "completion": {
                "enable": settings.get("completion.enable").unwrap_or(&serde_json::Value::Bool(true)),
                "snippet_support": settings.get("completion.snippet_support").unwrap_or(&serde_json::Value::Bool(true)),
            },
            "formatting": {
                "indent_size": settings.get("formatting.indent_size").unwrap_or(&serde_json::json!(2)),
                "max_line_length": settings.get("formatting.max_line_length").unwrap_or(&serde_json::Value::Null),
            },
            "hover": {
                "enable": settings.get("hover.enable").unwrap_or(&serde_json::Value::Bool(true)),
                "doc_links": settings.get("hover.doc_links").unwrap_or(&serde_json::Value::Bool(true)),
            },
        })))
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> zed::Result<Option<serde_json::Value>> {
        Ok(Some(serde_json::json!({
            "lammps": {
                "lammps_doc_path": null,
            }
        })))
    }

    fn label_for_completion(
        &self,
        _language_server_id: &zed::LanguageServerId,
        completion: zed::lsp::Completion,
    ) -> Option<zed::CodeLabel> {
        let label = completion.label;

        Some(zed::CodeLabel {
            code: label.clone(),
            spans: vec![
                zed::CodeLabelSpan::code_range(0..label.len()),
            ],
            filter_range: (0..label.len()).into(),
        })
    }

    fn label_for_symbol(
        &self,
        _language_server_id: &zed::LanguageServerId,
        symbol: zed::lsp::Symbol,
    ) -> Option<zed::CodeLabel> {
        Some(zed::CodeLabel {
            code: symbol.name.clone(),
            spans: vec![
                zed::CodeLabelSpan::code_range(0..symbol.name.len()),
            ],
            filter_range: (0..symbol.name.len()).into(),
        })
    }
}

zed::register_extension!(LammpsExtension);
