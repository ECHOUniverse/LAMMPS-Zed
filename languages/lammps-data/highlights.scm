; LAMMPS data file syntax highlighting for Zed

; === Section keywords (single word, Capitalized) ===
; Masses, Atoms, Bonds, Angles, Dihedrals, Impropers, etc.
((identifier) @keyword
  (#match? @keyword "^(Masses|Atoms|Bonds|Angles|Dihedrals|Impropers|Velocities|Ellipsoids|Lines|Triangles|Bodies)$"))

; === Multi-word section keywords — first word ===
; Pair, Bond, Angle, Dihedral, Improper, BondBond, Atom, etc.
((identifier) @keyword
  (#match? @keyword "^(Pair|PairIJ|Bond|Angle|Dihedral|Improper|BondBond|BondAngle|MiddleBondTorsion|EndBondTorsion|AngleTorsion|AngleAngleTorsion|BondBond13|AngleAngle|Atom)$"))

; === Multi-word section keywords — following words ===
; Coeffs, Labels, Types, Torsion
((identifier) @keyword
  (#match? @keyword "^(Coeffs|Labels|Types|Torsion|Coefs)$"))

; === Header count keywords ===
((identifier) @keyword
  (#match? @keyword "^(atoms|bonds|angles|dihedrals|impropers|ellipsoids|lines|triangles|bodies|types|xlo|xhi|ylo|yhi|zlo|zhi|xy|xz|yz|extra|special|per|bond|angle|dihedral|improper|atom)$"))

; === Numbers ===
[
  (int)
  (float)
] @number

; === Strings ===
(string) @string

; === Comments ===
(comment) @comment
