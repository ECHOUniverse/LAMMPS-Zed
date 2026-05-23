# LAMMPS for Zed

[Zed](https://zed.dev) 编辑器的 LAMMPS `.in` 输入脚本语法高亮扩展，基于 Tree-sitter 语法解析器。

## 功能

- LAMMPS `.in` 输入脚本语法高亮（9 类节点映射）
- 括号匹配：`()` `[]` `${}` + 引号 `""` `''` `""""""`
- 代码大纲：fix、compute、variable 定义 + 分隔注释定位
- 变量追踪：定义/引用跳转、符号重命名（locals.scm）
- 智能缩进：行续接 `&` + 三引号字符串内容缩进
- Vim 文本对象：`af`/`if` 整条命令、`gc` 连续注释
- 上下文感知：字符串/注释内禁用自动补全触发
- Shell 注入：`shell` 命令内容获得 bash 语法高亮
- Run 按钮：标记为可运行 LAMMPS 脚本，配置 tasks.json 一键执行
- **74 个代码片段**：覆盖 pair/fix/compute/dump/thermo 等常用模拟命令

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
    config.toml                   # 语言元数据 + autoclose + overrides
    highlights.scm                # 语法高亮（9 类节点）
    brackets.scm                  # 括号匹配（括号 + 引号）
    outline.scm                   # 代码大纲
    locals.scm                    # 变量定义/引用追踪
    indents.scm                   # 自动缩进（行续接 + 三引号）
    textobjects.scm               # Vim 文本对象
    overrides.scm                 # 语法作用域（string/comment）
    injections.scm                # Shell 注入 bash 高亮
    runnables.scm                 # 运行按钮检测
  snippets/
    lammps.json                   # 代码片段（74 个）
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

# 克隆并编译 grammar（本地调试需进入 grammar 目录）
cd grammars/lammps && tree-sitter generate

# 测试语法解析
tree-sitter parse ../../examples/input.in

# 测试所有 query 文件
for f in ../../languages/lammps/*.scm; do
  echo "=== $(basename $f) ==="
  tree-sitter query "$f" ../../examples/input.in
done

# Zed 中安装开发扩展
# Cmd+Shift+P → "zed: install dev extension" → 选择项目目录

# 查看 Zed 日志
# Cmd+Shift+P → "zed: open log"
```

## 依赖

- [tree-sitter-lammps](https://github.com/ECHOUniverse/tree-sitter-lammps) v0.0.8（[crates.io](https://crates.io/crates/tree-sitter-lammps) 镜像）

## 许可

MIT
