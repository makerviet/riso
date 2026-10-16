//! RASO — bộ máy tính điểm khai báo bằng YAML.
pub mod error;
mod eval;
pub mod expr;
pub mod extends;
mod graph;
pub mod model;
#[cfg(not(target_arch = "wasm32"))]
pub mod pack;
mod rank;
pub mod schema;
#[cfg(feature = "wasm")]
pub mod wasm;

pub use error::{RuleError, ScoreError};
pub use eval::score;
pub use extends::{MapResolver, NoParents, PackResolver};
pub use model::{
    FoulLine, InputValue, LineItem, Observation, ScoreSheet, SideObservation, SideSheet, TeamEntry,
    TeamResult,
};
pub use rank::{MatchResult, StandingRow, Standings, rank};
pub use schema::RulesFile;

use rhai::{Dynamic, Engine, Map};

/// Luật đã kiểm tra và biên dịch, sẵn sàng tính điểm. `Send + Sync`.
#[derive(Debug)]
pub struct Rules {
    pub file: RulesFile,
    pub(crate) engine: Engine,
    pub(crate) plan: graph::Plan,
    pub(crate) constants: Vec<(String, Dynamic)>,
}

pub(crate) fn yaml_to_dynamic(v: &serde_yaml_ng::Value) -> Dynamic {
    use serde_yaml_ng::Value;
    match v {
        Value::Bool(b) => Dynamic::from_bool(*b),
        Value::Number(n) => n
            .to_string()
            .parse::<rust_decimal::Decimal>()
            .map(Dynamic::from_decimal)
            .unwrap_or(Dynamic::UNIT),
        Value::String(s) => Dynamic::from(s.clone()),
        Value::Sequence(items) => Dynamic::from_array(items.iter().map(yaml_to_dynamic).collect()),
        Value::Mapping(map) => {
            let mut out = Map::new();
            for (k, val) in map {
                let key = match k {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                out.insert(key.into(), yaml_to_dynamic(val));
            }
            Dynamic::from_map(out)
        }
        Value::Tagged(tagged) => yaml_to_dynamic(&tagged.value),
        Value::Null => Dynamic::UNIT,
    }
}

pub fn load(yaml: &str, resolver: &dyn PackResolver) -> Result<Rules, Vec<RuleError>> {
    let file = extends::parse_rules(yaml, resolver)?;
    let engine = expr::new_engine();
    let plan = graph::build_plan(&engine, &file, yaml)?;
    let constants = file
        .constants
        .iter()
        .map(|(k, v)| (k.clone(), yaml_to_dynamic(v)))
        .collect();
    Ok(Rules {
        file,
        engine,
        plan,
        constants,
    })
}

#[cfg(test)]
fn _assert_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<Rules>();
}
