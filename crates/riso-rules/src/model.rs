use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum InputValue {
    Bool(bool),
    Int(i64),
    /// Mọi thứ khác (chuỗi lựa chọn; số lẻ từ JSON được đọc thành `Text` để bị từ chối rõ ràng).
    Text(String),
}

/// Bộ đọc khoan dung: số lẻ (2.5) không làm hỏng việc parse cả biên bản mà
/// trở thành `Text("2.5")` để `score` trả `OBS_RANGE` có tên ô.
struct LenientVisitor;

impl Visitor<'_> for LenientVisitor {
    type Value = InputValue;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("bool, số hoặc chuỗi")
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<InputValue, E> {
        Ok(InputValue::Bool(v))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<InputValue, E> {
        Ok(InputValue::Int(v))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<InputValue, E> {
        Ok(i64::try_from(v).map(InputValue::Int).unwrap_or_else(|_| InputValue::Text(v.to_string())))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<InputValue, E> {
        Ok(InputValue::Text(v.to_string()))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<InputValue, E> {
        Ok(InputValue::Text(v.to_string()))
    }
}

impl<'de> Deserialize<'de> for InputValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(LenientVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamEntry {
    pub id: String,
    #[serde(default)]
    pub card: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SideObservation {
    #[serde(flatten)]
    pub inputs: BTreeMap<String, InputValue>,
    #[serde(default)]
    pub robots: Vec<BTreeMap<String, InputValue>>,
    #[serde(default)]
    pub fouls: BTreeMap<String, u32>,
    #[serde(default)]
    pub teams: Vec<TeamEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub game: String,
    #[serde(default)]
    pub rules_version: Option<String>,
    #[serde(default)]
    pub field: BTreeMap<String, InputValue>,
    pub sides: BTreeMap<String, SideObservation>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub not_simulated: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineItem {
    pub name: String,
    pub label: Option<String>,
    pub label_en: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoulLine {
    pub id: String,
    pub label: String,
    pub count: u32,
    pub points_each: String,
    pub total: String,
    pub goes_to: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamResult {
    pub id: String,
    pub card: Option<String>,
    pub team_score: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SideSheet {
    pub values: Vec<LineItem>,
    pub robots: Vec<Vec<LineItem>>,
    pub fouls: Vec<FoulLine>,
    pub fouls_received: String,
    pub fouls_committed: String,
    pub raw_score: String,
    pub score: i64,
    pub teams: Vec<TeamResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreSheet {
    pub game: String,
    pub rules_version: String,
    pub field: Vec<LineItem>,
    pub sides: BTreeMap<String, SideSheet>,
    pub not_simulated: Vec<String>,
    #[serde(default)]
    pub scoring_error: Option<String>,
}
