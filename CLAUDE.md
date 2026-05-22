# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

这是一个 Zed 编辑器的 **LAMMPS `.in` 输入脚本语法高亮扩展**，基于 Tree-sitter 语法解析器。

目前已实现：
- 语法高亮（highlights.scm）：9 类节点映射（keyword, constant, number, function, property, type, comment, string, operator）
- 括号匹配（brackets.scm）：`()`, `[]`, `${}`
- 代码大纲（outline.scm）：fix/compute/variable 定义

## 语言与交流

- **始终使用中文回复用户**
- 遇到不确定的项目决策时，直接询问用户，不自行猜测

## 项目结构

```
LAMMPS-Zed/
  extension.toml                  # 扩展清单
  languages/lammps/
    config.toml                   # 语言元数据
    highlights.scm                # 语法高亮规则
    brackets.scm                  # 括号匹配
    outline.scm                   # 代码大纲
  examples/
    input.in                      # 示例 LAMMPS 输入脚本
```

**注意**：本项目是纯 Tree-sitter 语法扩展，不需要 Cargo.toml 和 Rust 代码。Zed 根据 `extension.toml` 中 `[grammars]` 配置自动下载并编译 grammar 为 WASM。

## 关键文件职责

- **extension.toml** — 扩展元数据（id, name, version, authors, repository, languages），注册 grammar 仓库（`[grammars.lammps]`）
- **languages/lammps/config.toml** — 语言名称、语法关联、文件后缀 `path_suffixes = ["in"]`（**不带点号**）、行注释 `#`
- **languages/lammps/highlights.scm** — Tree-sitter 查询模式，将语法节点映射到高亮类别
- **languages/lammps/brackets.scm** — 括号匹配规则（`@open`/`@close`）
- **languages/lammps/outline.scm** — 代码大纲规则（`@item`/`@name`/`@context`）

## Grammar

使用 tree-sitter-lammps v0.0.8，fork 到 `https://github.com/ECHOUniverse/tree-sitter-lammps`（原 `chappertron/tree-sitter-lammps` 仓库已不可访问）。

### 可用节点类型

语法解析器支持的命名节点：
`command`, `command_name`, `fix`, `fix_id`, `fix_style`, `compute`, `compute_id`, `compute_style`, `variable`, `variable_def`, `variable_del`, `variable_style`, `shell`, `glob`, `thermo_kwarg`, `atom_property`, `group_id`, `bool`, `int`, `float`, `string_content`, `sub_string_content`, `comment`, `expression`, `binary_op`, `unary_op`, `func`, `identifier`, `args_under`, `argument_list`, `indexing`, `underscore_ident`, `var_curly`, `var_round`, `string`, `raw_string`, `triple_string`, `word`, `parens`

### 高亮查询当前映射

| 高亮类别 | 节点类型 |
|---------|---------|
| `@keyword` | `fix`, `compute`, `command_name`, `shell`, `thermo_kwarg`, `glob`, `variable_style` |
| `@constant.builtin` | `bool` |
| `@number` | `int`, `float` |
| `@function` | `fix_style`, `compute_style` |
| `@function.builtin` | `func → identifier` |
| `@property` | `variable`, `fix_id`, `compute_id` |
| `@type` | `atom_property`, `group_id` |
| `@comment` | `comment` |
| `@string` | `string_content`, `sub_string_content` |
| `@operator` | anonymous operator tokens |

## 开发调试

```bash
# 安装 tree-sitter CLI
npm install -g tree-sitter-cli

# 解析示例文件
cd /tmp/grammar-work && git clone https://github.com/ECHOUniverse/tree-sitter-lammps.git
cd tree-sitter-lammps && tree-sitter generate
tree-sitter parse /path/to/LAMMPS-Zed/examples/input.in

# 测试高亮查询
tree-sitter query /path/to/LAMMPS-Zed/languages/lammps/highlights.scm /path/to/LAMMPS-Zed/examples/input.in

# Zed 中安装开发扩展
# Cmd+Shift+P → "zed: install dev extension" → 选择项目目录

# 查看 Zed 日志
# Cmd+Shift+P → "zed: open log"
# 或在终端运行: tail -f ~/Library/Logs/Zed/Zed.log | grep -i lammps
```

## 踩坑记录

1. **`path_suffixes` 不带点号**：应写 `["in"]` 而非 `[".in"]`。Zed 对后缀的匹配去掉扩展名的点号。
2. **Grammar 仓库必须可访问**：`extension.toml` 中的 grammar `repository` 必须是 Zed 能 git clone 的 URL。原 `chappertron/tree-sitter-lammps` 已 404，需使用 fork 版本。
3. **`languages` 字段**：`extension.toml` 需要 `languages = ["languages/lammps"]` 显式注册语言目录。
4. **Grammar 编译缓存**：切换 grammar 仓库 URL 后，需删除 `grammars/` 目录让 Zed 重新 clone。
5. **`variable` 节点是标识符不是关键字**：在 highlights.scm 中应作为 `@property` 而非 `@keyword`。

## 发布扩展

1. 在本地完成开发和测试
2. Fork `zed-industries/extensions` 仓库
3. 将本仓库添加为 Git submodule
4. 在 `extensions.toml` 中添加新条目
5. 运行 `pnpm sort-extensions` 排序
6. 提交 PR 到 `zed-industries/extensions`

参考: https://zed.dev/docs/extensions/developing-extensions

## 重要约束

- 扩展 ID 必须全局唯一，提交后不可更改
- ID 和名称中不要包含 'zed'、'Zed' 或 'extension'
- 语言扩展不能捆绑语言服务器
- 提交到 Zed 扩展仓库前，必须完成本地测试
- 扩展仓库必须公开
