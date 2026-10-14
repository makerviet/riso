use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RulesFile {
    pub schema: String,
    pub game: String,
    pub name: String,
    pub version: String,
    pub source: String,
    pub sides: Sides,
    pub rounding: Rounding,
    pub inputs: Inputs,
    #[serde(default)]
    pub constants: IndexMap<String, serde_yaml_ng::Value>,
    pub values: Values,
    #[serde(default)]
    pub fouls: IndexMap<String, Foul>,
    #[serde(default)]
    pub cards: IndexMap<String, CardEffect>,
    pub score: String,
    pub ranking: Ranking,
    #[serde(default)]
    pub open_questions: Vec<OpenQuestion>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sides {
    pub ids: Vec<String>,
    pub teams: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Rounding {
    Ceil,
    Floor,
    Round,
    None,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Inputs {
    #[serde(default)]
    pub field: IndexMap<String, InputDef>,
    #[serde(default)]
    pub side: IndexMap<String, InputDef>,
    #[serde(default)]
    pub robot: IndexMap<String, InputDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InputDef {
    #[serde(rename = "type")]
    pub kind: InputKind,
    #[serde(default)]
    pub max: Option<i64>,
    #[serde(default)]
    pub values: Vec<String>,
    pub label: String,
    #[serde(default)]
    pub label_en: Option<String>,
    #[serde(default)]
    pub help: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InputKind {
    Count,
    Choice,
    Bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Values {
    #[serde(default)]
    pub robot: IndexMap<String, ValueDef>,
    #[serde(default)]
    pub field: IndexMap<String, ValueDef>,
    #[serde(default)]
    pub side: IndexMap<String, ValueDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ValueDef {
    Expr(String),
    Full {
        expr: String,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        label_en: Option<String>,
    },
}

impl ValueDef {
    pub fn expr(&self) -> &str {
        match self {
            Self::Expr(expr) | Self::Full { expr, .. } => expr,
        }
    }
    pub fn label(&self) -> Option<&str> {
        match self {
            Self::Expr(_) => None,
            Self::Full { label, .. } => label.as_deref(),
        }
    }
    pub fn label_en(&self) -> Option<&str> {
        match self {
            Self::Expr(_) => None,
            Self::Full { label_en, .. } => label_en.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Foul {
    pub label: String,
    #[serde(default)]
    pub label_en: Option<String>,
    pub points: String,
    pub goes_to: GoesTo,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum GoesTo {
    #[serde(rename = "opponent")]
    Opponent,
    #[serde(rename = "self")]
    SelfSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardEffect {
    TeamZero,
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ranking {
    pub aggregate: Aggregate,
    #[serde(default)]
    pub drop_lowest: Option<DropLowest>,
    #[serde(default)]
    pub keep_best: Option<usize>,
    #[serde(default)]
    pub tiebreakers: Vec<Tiebreaker>,
    #[serde(default)]
    pub final_tiebreak: FinalTiebreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Aggregate {
    Average,
    Sum,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DropLowest {
    pub count: usize,
    #[serde(default)]
    pub not_if: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tiebreaker {
    pub label: String,
    pub expr: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalTiebreak {
    #[default]
    Draw,
    TeamNumber,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OpenQuestion {
    pub id: String,
    pub used: String,
    pub source: String,
}
