; 括号匹配 - 用于 Zed 的彩虹括号和括号跳转
; 匹配 LAMMPS 表达式中的括号和各种引号

; 圆括号 - 函数调用和表达式分组
("(" @open ")" @close)

; 方括号 - 数组索引
("[" @open "]" @close)

; 花括号 - 变量展开 ${var}
("${" @open "}" @close)

; 双引号字符串 - 排除彩虹色
(("\"" @open "\"" @close)
  (#set! rainbow.exclude))

; 单引号原始字符串 - 排除彩虹色
(("'" @open "'" @close)
  (#set! rainbow.exclude))

; 三引号多行字符串 - 排除彩虹色
(("\"\"\"" @open "\"\"\"" @close)
  (#set! rainbow.exclude))
