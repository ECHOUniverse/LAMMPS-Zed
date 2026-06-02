; LAMMPS input script syntax highlighting for Zed
; Based on tree-sitter-lammps grammar v0.0.8
; Reference: lammps_vscode (ThFriedrich/lammps_vscode) and LAMMPS docs (docs.lammps.org)

; ============================================================
; Comments
; ============================================================
(comment) @comment

; ============================================================
; Constants
; ============================================================

; Boolean values parsed as bool node (.true., .false.)
(bool) @constant.builtin

; Special built-in constants parsed as constant node (PI, version)
(constant) @constant.builtin

; Special numeric/atom constants that appear as word tokens
((word) @constant.builtin
  (#match? @constant.builtin "^(NULL|EDGE|INF)$"))

; Boolean synonyms used as logical flags in LAMMPS commands
((word) @constant.builtin
  (#match? @constant.builtin "^(on|off|yes|no)$"))

; ============================================================
; Numbers
; ============================================================
[
  (int)
  (float)
] @number

; ============================================================
; Strings
; ============================================================
[
  (string_content)
  (sub_string_content)
] @string

; ============================================================
; Keywords (generic catch-all first, specific overrides after)
; ============================================================

; Core commands with dedicated grammar nodes
; fix/compute/variable are consumed as anonymous tokens in dedicated
; syntax nodes (not command_name), so they need string literal matches
[
  "fix"
  "compute"
  "variable"
] @keyword

; Generic command names — catch-all for any unrecognized command.
; Specific command_name overrides come AFTER this, since tree-sitter
; resolves ties (same specificity) by last-match-wins.
(command
  (command_name) @keyword)

; Shell command keyword (anonymous token, not the shell command content)
"shell" @keyword

; Variable style keywords (equal, atom, loop, world, universe, etc.)
(variable_style) @keyword

; ============================================================
; Control Flow (command_name overrides — must be after generic catch-all)
; ============================================================

; if command — LAMMPS conditional branching
(command
  (command_name) @keyword.control
  (#eq? @keyword.control "if"))

; ============================================================
; Style Commands — pair_style, bond_style, angle_style,
;   dihedral_style, improper_style, kspace_style
; The first argument after the command name is the style/solver name.
; The grammar parses these as generic commands (no dedicated node),
; so we match by command_name text and capture the first word arg.
; These override both the generic @keyword on command_name and any
; @constant.builtin match on the style name word.
; ============================================================
(command
  (command_name) @_style_cmd
  (args_under . (word) @function)
  (#match? @_style_cmd "^(pair_style|bond_style|angle_style|dihedral_style|improper_style|kspace_style)$"))

; ============================================================
; Dump Commands
; dump ID group-ID style N file args
; — style name is the 3rd argument
; ============================================================
(command
  (command_name) @_dump_cmd
  (args_under . (word) . (word) . (word) @function)
  (#eq? @_dump_cmd "dump"))

; ============================================================
; Flow control keywords in argument position (then/elif/else)
; Must be after style-specific patterns so style names in args don't
; get matched as control keywords.
; ============================================================
((word) @keyword.control
  (#match? @keyword.control "^(then|elif|else)$"))

; ============================================================
; Functions
; ============================================================

; Fix and compute style names (dedicated grammar nodes)
[
  (fix_style)
  (compute_style)
] @function

; Built-in function calls in expressions — covers:
;   Math: sqrt, exp, ln, log, abs, sin, cos, tan, asin, acos, atan,
;         atan2, random, normal, ceil, floor, round, ramp, stagger,
;         logfreq, logfreq2, logfreq3, stride, stride2, vdisplace,
;         swiggle, cwiggle
;   Group/Region: count, mass, charge, xcm, vcm, fcm, bound, gyration,
;                 ke, angmom, torque, inertia, omega
;   Special: sum, min, max, ave, trap, slope, gmask, rmask, grmask,
;            next, is_file
;   Feature: is_available, is_active, is_defined
(func
  function: (identifier) @function.builtin)

; ============================================================
; Variables & Properties
; ============================================================

; Variable expansions: $x, ${x}, $(expr)
[
  (simple_expansion)
  (var_curly)
  (var_round)
] @variable

; Underscore-prefix references: v_name, c_name, f_name
(underscore_ident) @variable

; Indexed array references: v_myvar[1], c_pe[2], f_nve[3]
(indexed_ident) @variable

; Variable names — both definitions (variable name equal ...) and
; references (${name}, $name)
(variable) @variable

; Bare identifiers in expressions (variable refs without $ prefix)
; Must be after the func pattern so function names retain @function.builtin
(identifier) @variable

; ============================================================
; Fix and compute identifiers — must be AFTER generic (identifier)
; so that identifiers inside fix_id/compute_id override @variable.
; ============================================================
(fix_id (identifier) @property)
(compute_id (identifier) @property)

; ============================================================
; Types & Attributes
; ============================================================

; Atom properties (x, y, z, vx, vy, vz, mass, type, etc.)
(atom_property) @type

; Group identifiers
(group_id) @type

; ============================================================
; Special Strings
; ============================================================

; Thermo output keywords (step, atoms, temp, press, etc.)
(thermo_kwarg) @string.special

; Wildcard/glob patterns (*)
(glob) @string.special

; ============================================================
; Group 命令中的自定义 group ID
; ============================================================
(command
  (command_name) @_grp_cmd
  (args_under . (word) @type)
  (#eq? @_grp_cmd "group"))

; jump 命令中的标签引用（第2个参数）
(command
  (command_name) @_jmp_cmd
  (args_under . (word) . (word) @label)
  (#eq? @_jmp_cmd "jump"))

; label 命令中的标签名
(command
  (command_name) @_lbl_cmd
  (args_under . (word) @label)
  (#eq? @_lbl_cmd "label"))

; ============================================================
; Operators
; ============================================================
[
  "+"
  "-"
  "*"
  "/"
  "%"
  "^"
  "=="
  "!="
  "<"
  "<="
  ">"
  ">="
  "&&"
  "||"
  "|^"
  "!"
] @operator
