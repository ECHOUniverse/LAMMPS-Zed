; 代码大纲 - 用于 Zed 的大纲视图 (Cmd+Shift+O)
; 显示 fix、compute、variable 定义结构和注释分节标题

; Variable 定义
(variable_def
  name: (variable) @name
  style: (variable_style) @context) @item

; Fix 语句
(fix
  fix_id: (fix_id) @name
  style: (fix_style) @context) @item

; Compute 语句
(compute
  compute_id: (compute_id) @name
  style: (compute_style) @context) @item

; 分节注释作为大纲标注 (e.g., # === 初始化设置 ===)
((comment) @annotation
  (#match? @annotation "^# =+"))
