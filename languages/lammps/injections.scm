; 语言注入 — 在 LAMMPS shell 命令中嵌入 bash 语法高亮
; shell 命令的参数内容作为 bash 脚本

(shell
  (_) @injection.content
  (#set! injection.language "bash"))
