# LAMMPS for Zed

A [Zed](https://zed.dev) editor extension for LAMMPS `.in` input scripts, featuring **Tree-sitter syntax highlighting** and **LSP-powered intelligent language services**.

## Features

### Syntax Extension
- Syntax highlighting (9 node categories)
- Bracket matching: `()` `[]` `${}` + quotes `""` `''` `""""""`
- Document outline: fix, compute, variable definitions + separator comment anchors
- Variable tracking: definition/reference jumps, symbol renaming (locals.scm)
- Smart indentation: line continuation `&` + triple-quoted string content indentation
- Vim text objects: `af`/`if` whole command, `gc` consecutive comments
- Context-aware: disables autocomplete triggers inside strings/comments
- Shell injection: `shell` command content gets bash syntax highlighting
- Run button + 74 code snippets

### LSP Intelligent Services
- **Diagnostics**: Real-time checking of command spelling, variable references, include file existence, expression syntax
- **Completion**: Command names, style names, variable/ID (`$x`/`${x}`/`v_`/`c_`/`f_`), parameter keywords
- **Hover Documentation**: Detailed command/style documentation, variable definition line content
- **Go-to-Definition**: Jump to variable/fix/compute/label definitions
- **Find References**: Find all variable/fix/compute references
- **Rename**: Safe renaming of variable/fix/compute IDs
- **Document Symbols**: Hierarchical symbol list
- **Formatting**: Indentation and whitespace normalization

## Installation

### Method 1: Zed Extension Marketplace (Recommended)

```bash
# Zed → Cmd+Shift+X → Search "LAMMPS" → Install
```

### Method 2: Development Extension

```bash
# Prerequisites
# 1. Rust toolchain (1.85+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install wasm32 build target
rustup target add wasm32-wasip2

# 3. Clone the repository
git clone https://github.com/ECHOUniverse/LAMMPS-Zed.git
cd LAMMPS-Zed

# 4. Build and install LSP binary
cargo build --release --manifest-path lsp/Cargo.toml
cp lsp/target/release/lammps-lsp ~/.cargo/bin/

# 5. Install dev extension in Zed
# Cmd+Shift+P → "zed: install dev extension" → Select project directory
```

## Project Structure

```
LAMMPS-Zed/
  Cargo.toml                      # WASM extension crate
  extension.toml                  # Extension manifest
  src/lib.rs                      # LspAdapter implementation
  lsp/                            # LAMMPS LSP server (Rust)
    Cargo.toml
    build.rs                      # Compile-time command database generation
    src/                          # 25 Rust source files
    data/                         # Command metadata (9 TOML files)
  languages/lammps/               # Tree-sitter grammar definitions
    config.toml                   # Language metadata
    highlights.scm                # Syntax highlighting
    brackets.scm                  # Bracket matching
    outline.scm                   # Document outline
    locals.scm                    # Variable tracking
    indents.scm                   # Auto indentation
    textobjects.scm               # Vim text objects
    overrides.scm                 # Syntax scopes
    injections.scm                # Shell injection
    runnables.scm                 # Run button
  snippets/lammps.json            # 74 code snippets
  examples/input.in               # Example input script
```

## Syntax Highlighting Coverage

| Category | Matched Nodes |
|----------|--------------|
| `@keyword` | `fix`, `compute`, `command_name`, `shell`, `thermo_kwarg`, `glob`, `variable_style` |
| `@constant.builtin` | `bool` |
| `@number` | `int`, `float` |
| `@function` | `fix_style`, `compute_style` |
| `@function.builtin` | Function calls (`exp`, `sqrt`, etc.) |
| `@property` | `variable`, `fix_id`, `compute_id` |
| `@type` | `atom_property`, `group_id` |
| `@comment` | Comments |
| `@string` | `string_content`, `sub_string_content` |
| `@operator` | Operators |

## LSP Diagnostic Rules

| Code | Severity | Description |
|------|----------|-------------|
| E001 | Warning | Unknown command (with edit distance suggestion) |
| E002 | Error | Undefined variable/fix/compute reference |
| E003 | Error | Invalid include file path |
| E004 | Warning | Insufficient command arguments |
| W002 | Warning | Duplicate variable definition |
| W004 | Warning | Expression issue (missing operand/unknown function) |

## Dependencies

- [tree-sitter-lammps](https://github.com/ECHOUniverse/tree-sitter-lammps) v0.0.8
- [tower-lsp-server](https://crates.io/crates/tower-lsp-server) v0.23
- Rust toolchain 1.85+

## License

MIT
