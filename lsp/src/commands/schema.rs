use serde::{Deserialize, Serialize};

/// Complete command database loaded at compile time.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandDatabase {
    pub general_commands: Vec<CommandDef>,
    pub pair_styles: Vec<StyleDef>,
    pub fix_styles: Vec<StyleDef>,
    pub compute_styles: Vec<StyleDef>,
    pub bond_styles: Vec<StyleDef>,
    pub angle_styles: Vec<StyleDef>,
    pub dihedral_styles: Vec<StyleDef>,
    pub improper_styles: Vec<StyleDef>,
    pub kspace_styles: Vec<StyleDef>,
}

/// Definition of a general LAMMPS command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDef {
    pub name: String,
    pub category: CommandCategory,
    pub parameters: Vec<Parameter>,
    pub doc_short: String,
    pub doc_full: String,
    pub lammps_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandCategory {
    Setup,
    Simulation,
    Output,
    Control,
    Input,
}

/// A single parameter of a LAMMPS command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: ParameterType,
    pub required: bool,
    pub default_value: Option<String>,
    pub doc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    Style,
    GroupId,
    FixId,
    ComputeId,
    Variable,
    VariableName,
    FileName,
    Label,
    Integer,
    Float,
    Boolean,
    String,
    Enum(Vec<String>),
    Expression,
    Keyword(String),
    Repeat(Box<ParameterType>),
}

/// Definition of a style (fix/compute/pair/bond/etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleDef {
    pub name: String,
    pub category: StyleCategory,
    pub since_version: Option<String>,
    pub doc_short: String,
    pub doc_full: String,
    pub required_args: Vec<Parameter>,
    pub optional_args: Vec<Parameter>,
    pub related_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StyleCategory {
    Pair,
    Fix,
    Compute,
    Bond,
    Angle,
    Dihedral,
    Improper,
    KSpace,
}
