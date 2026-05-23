; 语法作用域覆盖 — 定义字符串和注释的语义范围
; 配合 config.toml [overrides] 调整编辑器行为

; 字符串上下文（所有类型的字符串）
[
  (string)
  (raw_string)
  (triple_string)
  (string_content)
  (sub_string_content)
] @string

; 注释上下文
(comment) @comment
