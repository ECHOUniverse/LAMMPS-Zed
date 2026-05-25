use zed_extension_api as zed;

struct LammpsExtension {
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
        // Strategy 1: User-configured binary path from Zed settings
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

        // Strategy 3: Auto-download from GitHub Releases
        if self.cached_binary_path.is_none() {
            if let Some(path) = try_download_binary() {
                self.cached_binary_path = Some(path);
            }
        }

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

fn try_download_binary() -> Option<String> {
    let (os, arch) = zed::current_platform();

    let os_str = match os {
        zed::Os::Mac => "darwin",
        zed::Os::Linux => "linux",
        zed::Os::Windows => "windows",
    };

    let arch_str = match arch {
        zed::Architecture::Aarch64 => "arm64",
        zed::Architecture::X8664 => "x64",
        zed::Architecture::X86 => "x86",
    };

    let ext = if matches!(os, zed::Os::Windows) { ".exe" } else { "" };
    let binary_name = format!("lammps-lsp-{}-{}{}", os_str, arch_str, ext);

    let release = zed::latest_github_release(
        "ECHOUniverse/LAMMPS-Zed",
        zed::GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        },
    )
    .ok()?;

    let asset = release.assets.iter().find(|a| a.name == binary_name)?;

    zed::download_file(
        &asset.download_url,
        &binary_name,
        zed::DownloadedFileType::Uncompressed,
    )
    .ok()?;

    zed::make_file_executable(&binary_name).ok()?;

    Some(binary_name)
}

zed::register_extension!(LammpsExtension);
