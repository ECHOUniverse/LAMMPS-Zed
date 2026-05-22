; LAMMPS input script syntax highlighting for Zed
; Based on tree-sitter-lammps grammar v0.0.8

; === Keywords ===

; Core command statements
[
 "fix"
 "compute"
] @keyword

; Generic command names (catch-all for any unrecognized command)
(command
  (command_name) @keyword)

; Shell commands
(shell) @keyword

; Thermo output keywords (thermo_style args)
(thermo_kwarg) @keyword

; Wildcard/glob patterns
(glob) @keyword

; Variable style keywords (equal, atom, loop, world, universe, etc.)
(variable_style) @keyword


; === Constants ===

; Boolean values (NULL → .true., .false.)
(bool) @constant.builtin


; === Numbers ===

[
 (int)
 (float)
] @number


; === Functions ===

; Fix and compute style names
[
 (fix_style)
 (compute_style)
] @function

; Built-in function calls (e.g., exp(), sqrt(), abs())
(func
  function: (identifier) @function.builtin)


; === Properties ===

; Variable names, fix IDs, compute IDs
[
 (variable)
 (fix_id)
 (compute_id)
] @property


; === Types ===

; Atom properties (x, y, z, vx, vy, vz, mass, type, etc.)
(atom_property) @type

; Group identifiers
(group_id) @type


; === Comments ===

(comment) @comment


; === Strings ===

[
 (string_content)
 (sub_string_content)
] @string


; === Operators ===

; Binary and unary operators (anonymous tokens from expressions)
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
