; 文本对象 — 用于 Zed Vim 模式的语法感知选择
; af/if: 选择整条命令/命令内部
; ac/ic: 选择区块
; gc: 选择注释

; 整条 fix 命令
(fix) @function.around

; 整条 compute 命令
(compute) @function.around

; 整条 variable 定义
(variable_def) @function.around

; 整条 variable 删除
(variable_del) @function.around

; 整条 shell 命令
(shell) @function.around

; 通用命令
(command) @function.around

; 连续注释作为一个整体
(comment)+ @comment.around
