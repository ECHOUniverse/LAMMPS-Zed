# LAMMPS Zed Extension - 功能增强规格

## 调研结论

### Zed 支持的 Tree-sitter Query 文件

| 文件 | 状态 | 说明 |
|------|------|------|
| `highlights.scm` | 已有 | 语法高亮，9 类节点映射 |
| `brackets.scm` | 增强 | 括号匹配 + 引号匹配（新增） |
| `outline.scm` | 已有 | 代码大纲（fix/compute/variable 定义 + 分隔注释） |
| `locals.scm` | 已有 | 变量定义/引用追踪，支持跳转定义和重命名 |
| `indents.scm` | 已有 | 三引号字符串内容缩进（行续接由 config.toml 处理） |
| `textobjects.scm` | **新增** | Vim 模式文本对象选择 |
| `overrides.scm` | **新增** | 上下文感知编辑器行为 |
| `injections.scm` | **新增** | shell 命令内嵌 bash 语法高亮 |
| `runnables.scm` | **新增** | 运行按钮检测 |
| `folds.scm` | Zed 不支持 | Zed 折叠基于缩进层级，无独立 query 文件 |

### 参考资料
- [Zed Language Extensions](https://zed.dev/docs/extensions/languages)
- [Zed Snippets](https://zed.dev/docs/extensions/snippets)
- [Zed Language Configuration](https://zed.dev/docs/configuring-languages)

### indents.scm 捕获协议

Zed 使用 `@indent` + `@end` 两个捕获（区别于 Helix 的 `@outdent` 和 nvim-treesitter 的 `@indent.end`）：

```scm
; 模式: (节点 "闭括号" @end) @indent
(array "]" @end) @indent
(object "}" @end) @indent

; 或分离写法
["{", "(", "["] @indent
["}", ")", "]"] @end
```

### textobjects.scm 捕获协议

| 捕获 | Vim 操作 | 语义 |
|------|----------|------|
| `@function.around` | `af` | 整条命令/小块 |
| `@function.inside` | `if` | 命令内部 |
| `@class.around` | `ac` | 整个区块 |
| `@class.inside` | `ic` | 区块内容 |
| `@comment.around` | `gc` | 连续注释 |

### overrides.scm 捕获协议

捕获名即为作用域名，配合 config.toml `[overrides.<name>]` 覆盖编辑器设置。`.inclusive` 后缀使范围包含前后空白。

### injections.scm 捕获协议

- `@injection.language` — 指定目标语言
- `@injection.content` — 注入的内容
- `(#set! injection.language "xxx")` — 硬编码目标语言

### runnables.scm 捕获协议

- `@run` — 运行按钮位置
- `(#set! tag xxx)` — 标签，与 tasks.json 档关联
- 非下划线前缀捕获作为 `ZED_CUSTOM_<capture>` 环境变量暴露

---

## 实施总结

### F1: brackets.scm — 增强括号匹配

**变更**：新增三种引号的括号匹配，均排除彩虹色高亮
- `"..."` 双引号 — 匹配后支持 `%` 跳转
- `'...'` 单引号 — 同上
- `"""..."""` 三引号 — 同上

### F2: textobjects.scm — 智能文本对象

**变更**：新增 6 种文本对象规则
- `fix`, `compute`, `variable_def`, `variable_del`, `shell`, `command` → `@function.around`
- `(comment)+` → `@comment.around`

### F3: overrides.scm + config.toml — 上下文感知编辑

**变更**：
- 定义 `@string` 和 `@comment` 两个语法作用域
- config.toml 中字符串/注释内禁用自动补全触发
- 配置 5 对 autoclose brackets，字符串内禁用引号自动闭合

### F4: injections.scm — Shell 命令高亮注入

**变更**：`shell` 命令内容获得 bash 语法高亮
- 使用 `(_) @injection.content` 捕获匿名子节点
- 注意 `word` 是 grammar 中的匿名 token，需用 `_` 匹配

### F5: runnables.scm — 可运行脚本检测

**变更**：整个文件标记为 runnable
- `(input_script) @run` 在文件顶部生成 Run 按钮
- 标签 `lammps-script` 关联 tasks.json

### F6: 扩充代码片段

**变更**：从 25 个扩展至 **74 个**

新增类别：
- Pair styles: lj/cut, eam, eam/alloy, buck, tersoff, airebo
- Fix styles: nph, deform, ave/time, print, setforce, spring, wall/lj126, wall/harmonic, recenter, momentum
- Compute styles: reduce, stress/atom, centro/atom, msd, vacf, rdf, ke/atom, pe/atom, coord/atom
- 控制命令: velocity, minimize, group, neighbor, neigh_modify, reset_timestep, change_box
- 文件 I/O: delete_atoms, replicate, write_data, write_restart, read_data, read_restart
- 系统设置: units, boundary, atom_style, create_box, create_atoms, mass, log, restart
- 可视化: dump image
- 控制流: loop template (label + jump)
