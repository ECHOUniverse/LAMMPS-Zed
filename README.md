# LAMMPS for Zed

[Zed](https://zed.dev) 编辑器的 LAMMPS `.in` 输入脚本扩展，包含 **Tree-sitter 语法高亮** 和 **LSP 智能语言服务**。

## 功能

### 语法扩展
- 语法高亮（9 类节点映射）
- 括号匹配：`()` `[]` `${}` + 引号 `""` `''` `""""""`
- 代码大纲：fix、compute、variable 定义 + 分隔注释定位
- 变量追踪：定义/引用跳转、符号重命名（locals.scm）
- 智能缩进：行续接 `&` + 三引号字符串内容缩进
- Vim 文本对象：`af`/`if` 整条命令、`gc` 连续注释
- 上下文感知：字符串/注释内禁用自动补全触发
- Shell 注入：`shell` 命令内容获得 bash 语法高亮
- Run 按钮 + 74 个代码片段

### LSP 智能服务
- **诊断**：实时检查命令拼写、变量引用、include 文件存在、表达式语法
- **补全**：命令名、样式名、变量/ID（`$x`/`${x}`/`v_`/`c_`/`f_`）、参数关键词
- **悬停文档**：命令/样式详细文档、变量定义行内容
- **跳转定义**：变量/fix/compute/标签定义跳转
- **引用查找**：查找所有变量/fix/compute 引用
- **重命名**：安全重命名变量/fix/compute ID
- **代码大纲**：分层符号列表
- **格式化**：缩进和空白规范化

## 安装

### 方式 1：Zed 扩展市场（推荐）

```bash
# Zed → Cmd+Shift+X → 搜索 "LAMMPS" → 安装
```

### 方式 2：开发扩展

```bash
# 前置要求
# 1. Rust 工具链（1.85+）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 安装 wasm32 编译目标
rustup target add wasm32-wasip2

# 3. 克隆仓库
git clone https://github.com/ECHOUniverse/LAMMPS-Zed.git
cd LAMMPS-Zed

# 4. 编译并安装 LSP 二进制
cargo build --release --manifest-path lsp/Cargo.toml
cp lsp/target/release/lammps-lsp ~/.cargo/bin/

# 5. 在 Zed 中安装开发扩展
# Cmd+Shift+P → "zed: install dev extension" → 选择项目目录
```

## 目录结构

```
LAMMPS-Zed/
  Cargo.toml                      # WASM 扩展 crate
  extension.toml                  # 扩展清单
  src/lib.rs                      # LspAdapter 实现
  lsp/                            # LAMMPS LSP 服务器（Rust）
    Cargo.toml
    build.rs                      # 编译时命令数据库生成
    src/                          # 25 个 Rust 源文件
    data/                         # 命令元数据（9 个 TOML）
  languages/lammps/               # Tree-sitter 语法定义
    config.toml                   # 语言元数据
    highlights.scm                # 语法高亮
    brackets.scm                  # 括号匹配
    outline.scm                   # 代码大纲
    locals.scm                    # 变量追踪
    indents.scm                   # 自动缩进
    textobjects.scm               # Vim 文本对象
    overrides.scm                 # 语法作用域
    injections.scm                # Shell 注入
    runnables.scm                 # 运行按钮
  snippets/lammps.json            # 74 个代码片段
  examples/input.in               # 示例脚本
```

## 语法高亮覆盖

| 类别 | 匹配节点 |
|------|---------|
| `@keyword` | `fix`, `compute`, `command_name`, `shell`, `thermo_kwarg`, `glob`, `variable_style` |
| `@constant.builtin` | `bool` |
| `@number` | `int`, `float` |
| `@function` | `fix_style`, `compute_style` |
| `@function.builtin` | 函数调用 (`exp`, `sqrt` 等) |
| `@property` | `variable`, `fix_id`, `compute_id` |
| `@type` | `atom_property`, `group_id` |
| `@comment` | 注释 |
| `@string` | `string_content`, `sub_string_content` |
| `@operator` | 运算符 |

## LSP 诊断规则

| 代码 | 严重度 | 描述 |
|------|--------|------|
| E001 | Warning | 未知命令（含编辑距离建议） |
| E002 | Error | 未定义变量/fix/compute 引用 |
| E003 | Error | Include 文件路径无效 |
| E004 | Warning | 命令参数不足 |
| W002 | Warning | 重复变量定义 |
| W004 | Warning | 表达式问题（缺操作数/未知函数） |

## 依赖

- [tree-sitter-lammps](https://github.com/ECHOUniverse/tree-sitter-lammps) v0.0.8
- [tower-lsp-server](https://crates.io/crates/tower-lsp-server) v0.23
- Rust 工具链 1.85+

## 许可

MIT
