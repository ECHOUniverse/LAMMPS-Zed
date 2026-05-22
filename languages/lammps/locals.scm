; 变量作用域分析 - 用于 Zed 的变量定义/引用追踪、重命名符号、跳转定义
; 基于 Tree-sitter 标准 locals 查询

; === 作用域 ===

; 整个输入脚本作为全局作用域
(input_script) @local.scope

; === 变量定义 ===

(variable_def
  name: (variable) @local.definition)

; Fix 定义
(fix
  fix_id: (fix_id) @local.definition)

; Compute 定义
(compute
  compute_id: (compute_id) @local.definition)

; === 变量引用 ===

; 花括号展开 ${var}
(var_curly
  (variable) @local.reference)

; 下划线前缀引用 - v_var, c_var, f_var
(underscore_ident
  (variable) @local.reference)

(underscore_ident
  (fix_id) @local.reference)

(underscore_ident
  (compute_id) @local.reference)

; 简单展开 $x (单字符变量名)
(simple_expansion
  (variable) @local.reference)
