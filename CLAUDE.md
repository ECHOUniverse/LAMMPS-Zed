# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

这是一个 Zed 编辑器的 **LAMMPS `.in` 输入脚本语法解析扩展**。基于 Tree-sitter 语法解析器，为 LAMMPS 分子动力学模拟的输入脚本提供语法高亮、代码结构等语言支持。

## 语言与交流

- **始终使用中文回复用户**
- 遇到不确定的项目决策时，直接询问用户，不自行猜测

## Zed 扩展架构

Zed 扩展使用 Tree-sitter 解析器提供语言支持。扩展目录结构：

```
lammps-extension/
  extension.toml          # 扩展清单（必选）
  Cargo.toml              # Rust 项目配置（需要 cdylib 输出）
  src/
    lib.rs                # Rust 扩展入口
  languages/
    lammps/
      config.toml         # 语言元数据定义
      highlights.scm      # 语法高亮查询规则
```

### 关键文件职责

- **extension.toml** — 扩展元数据（id, name, version, authors, repository），注册 Tree-sitter 语法仓库（`[grammars.lammps]`）
- **languages/lammps/config.toml** — 定义语言名称、语法关联、文件后缀（`.in`）、行注释符号（`#`）
- **languages/lammps/highlights.scm** — Tree-sitter 查询模式，将语法节点映射到高亮类别（`@keyword`, `@string`, `@number`, `@comment` 等）
- **src/lib.rs** — （可选）Rust 扩展 API，用于下载/管理语法解析器

## 现有资源

### tree-sitter-lammps (v0.0.8)

仓库地址: `https://github.com/chappertron/tree-sitter-lammps`（crates.io/documented on docs.rs）

已完成的语法覆盖：
- 基础命令结构（`command_name` + `args_under`）
- `fix`、`compute`、`variable` 特定解析（含 style、ID、group 字段）
- `shell` 命令、行注释（`#`）、行续接（`&`）
- 表达式系统：二元/一元运算符、函数调用、括号
- 变量展开：`${var}`、`$(expr)`、`v_`/`f_`/`c_` 下划线引用
- 字符串类型：双引号（`"..."`）、单引号原始字符串（`'...'`）、三引号（`"""..."""`）
- 热力学关键词（`thermo_kwarg`）、原子属性（`atom_property`）、布尔常量

已有高亮查询（`highlights.scm`）：
- `@keyword` — fix, compute, variable, command_name, thermo_kwarg, glob
- `@constant.builtin` — 布尔值
- `@number` — int, float
- `@function` — fix_style, compute_style
- `@property` — variable, fix_id, compute_id
- `@comment` — comment
- `@function.builtin` — 函数调用
- `@string` — string_content, sub_string_content

## 开发命令

```bash
# 验证扩展结构
# 将扩展目录复制/链接到 Zed 扩展目录进行本地测试
# macOS: ~/Library/Application Support/Zed/extensions/
ln -s "$(pwd)" ~/Library/Application\ Support/Zed/extensions/lammps

# 在 Zed 中打开开发模式查看扩展日志
# Cmd+Shift+P → "zed: open log"

# 使用 tree-sitter CLI 测试语法解析（如安装了 tree-sitter CLI）
tree-sitter parse examples/input.in
tree-sitter highlight examples/input.in

# Rust 构建（如有 src/lib.rs）
cargo build --release
```

## LAMMPS `.in` 文件语法要点

- 关键字从最左列开始，全部小写
- 参数用空格/Tab 分隔
- `#` 开始一行注释
- `&` 表示行续接
- 变量引用：`v_name`（变量）、`f_name`（fix）、`c_name`（compute）、`${name}`、`$(expr)`
- 命令类别：初始化（units, dimension, boundary, atom_style）、系统定义（read_data, lattice, region）、力场（pair_style, bond_style）、fix/compute、输出（thermo, dump）、控制（if, jump, variable）

## 发布扩展

1. 在本地完成开发和测试
2. Fork `zed-industries/extensions` 仓库
3. 将本仓库添加为 Git submodule（使用 HTTPS URL）
4. 在 `extensions.toml` 中添加新条目
5. 运行 `pnpm sort-extensions` 排序
6. 提交 PR 到 `zed-industries/extensions`

参考: https://zed.dev/docs/extensions/developing-extensions

## 重要约束

- 扩展 ID 必须全局唯一，提交后不可更改
- ID 和名称中不要包含 'zed'、'Zed' 或 'extension'
- 语言扩展不能捆绑语言服务器，应使用下载或用户环境检查方式
- 提交到 Zed 扩展仓库前，必须完成本地测试
- 扩展仓库必须公开
