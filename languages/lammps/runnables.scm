; 可运行代码检测 — 为 LAMMPS 输入脚本添加 Run 按钮
; 将整个文件标记为可运行的 LAMMPS 脚本
; 用户需在 tasks.json 中配置运行命令，例如：
; {
;   "label": "run lammps",
;   "command": "lmp -in $ZED_FILE",
;   "tags": ["lammps-script"]
; }

(input_script) @run
(#set! tag lammps-script)
