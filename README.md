# LAMMPS for Zed

[Zed](https://zed.dev) 编辑器的 LAMMPS `.in` 输入脚本语法高亮扩展，基于 Tree-sitter 语法解析器。

## 功能

- LAMMPS `.in` 输入脚本语法高亮
- 括号匹配（圆括号、方括号、`${}` 变量展开）
- 代码大纲（fix、compute、variable 定义结构）
- 支持注释、行续接、变量引用等完整语法特性

## 安装

### 开发扩展

```bash
# 1. 克隆仓库
git clone https://github.com/hanxu/LAMMPS-Zed.git
cd LAMMPS-Zed

# 2. 在 Zed 中安装开发扩展
# Cmd+Shift+P → "zed: install dev extension" → 选择项目目录
```

### 手动链接

```bash
ln -s "$(pwd)" ~/Library/Application\ Support/Zed/extensions/lammps
```

## 目录结构

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

## 语法高亮覆盖

| 高亮类别 | 匹配节点 |
|---------|---------|
| `@keyword` | `fix`, `compute`, `command_name`, `shell`, `thermo_kwarg`, `glob`, `variable_style` |
| `@constant.builtin` | `bool` |
| `@number` | `int`, `float` |
| `@function` | `fix_style`, `compute_style` |
| `@function.builtin` | 函数调用 (`exp`, `sqrt` 等) |
| `@property` | `variable`, `fix_id`, `compute_id` |
| `@type` | `atom_property`, `group_id` |
| `@comment` | 注释 |
| `@string` | `string_content`, `sub_string_content` |
| `@operator` | 二元和一元运算符 |

## 开发

```bash
# 安装 tree-sitter CLI
npm install -g tree-sitter-cli

# 测试语法解析
tree-sitter parse examples/input.in

# 测试高亮查询
tree-sitter query languages/lammps/highlights.scm examples/input.in
```

## 依赖

- [tree-sitter-lammps](https://github.com/ECHOUniverse/tree-sitter-lammps) v0.0.8（[crates.io](https://crates.io/crates/tree-sitter-lammps) 镜像）

## 许可

MIT
