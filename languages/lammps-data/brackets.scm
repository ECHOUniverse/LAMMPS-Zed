; 括号匹配 — 用于 Zed 的彩虹括号和括号跳转

; 双引号 - 字符串 (type labels)，排除彩虹色
(("\"" @open "\"" @close)
  (#set! rainbow.exclude))
