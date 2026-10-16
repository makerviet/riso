//! Chạy bộ test `tests/scoring/*.yaml` của một đề. Không dùng trên wasm32.
use crate::error::RuleError;
use crate::model::Observation;
use crate::{PackResolver, load, score};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_yaml_ng::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct DirResolver {
    pub games_root: PathBuf,
}

impl PackResolver for DirResolver {
    fn rules_yaml(&self, game: &str) -> Option<String> {
        std::fs::read_to_string(self.games_root.join(game).join("rules.yaml")).ok()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TestCase {
    name: String,
    observation: Observation,
    #[serde(default)]
    expect: Option<Expect>,
    #[serde(default)]
    expect_error: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    #[serde(default)]
    field: BTreeMap<String, Value>,
    #[serde(default)]
    sides: BTreeMap<String, ExpectSide>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectSide {
    #[serde(default)]
    score: Option<i64>,
    #[serde(default)]
    values: BTreeMap<String, Value>,
    #[serde(default)]
    fouls_received: Option<Value>,
    #[serde(default)]
    teams: BTreeMap<String, i64>,
}

#[derive(Debug, Clone)]
pub struct CaseOutcome {
    pub name: String,
    pub file: String,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PackReport {
    pub game: String,
    pub cases: Vec<CaseOutcome>,
}

impl PackReport {
    pub fn ok(&self) -> bool {
        self.cases.iter().all(|c| c.failures.is_empty())
    }
}

fn same(expected: &Value, actual: &str) -> bool {
    let expected_text = match expected {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        other => format!("{other:?}"),
    };
    match (expected_text.parse::<Decimal>(), actual.parse::<Decimal>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => expected_text == actual,
    }
}

pub fn check_pack(dir: &Path) -> Result<PackReport, Vec<RuleError>> {
    let rules_path = dir.join("rules.yaml");
    let yaml = std::fs::read_to_string(&rules_path).map_err(|e| {
        vec![RuleError::new(
            "RULE_YAML",
            rules_path.display().to_string(),
            e.to_string(),
        )]
    })?;
    let resolver = DirResolver {
        games_root: dir.parent().unwrap_or(dir).to_path_buf(),
    };
    let rules = load(&yaml, &resolver)?;
    let tests_dir = dir.join("tests/scoring");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&tests_dir)
        .map(|it| {
            it.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    if files.is_empty() {
        return Err(vec![RuleError::new(
            "RULE_NO_TESTS",
            tests_dir.display().to_string(),
            "đề phải có ít nhất một file test trong tests/scoring",
        )]);
    }
    let mut cases = Vec::new();
    for file in files {
        let shown = file.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let case: TestCase = match serde_yaml_ng::from_str(&text) {
            Ok(case) => case,
            Err(e) => {
                cases.push(CaseOutcome {
                    name: shown.clone(),
                    file: shown,
                    failures: vec![format!("file test sai định dạng: {e}")],
                });
                continue;
            }
        };
        let mut failures = Vec::new();
        match (
            score(&rules, &case.observation),
            &case.expect_error,
            &case.expect,
        ) {
            (Err(errors), Some(code), _) => {
                if !errors.iter().any(|e| e.code == code) {
                    failures.push(format!(
                        "cần lỗi {code}, nhận {:?}",
                        errors.iter().map(|e| e.code).collect::<Vec<_>>()
                    ));
                }
            }
            (Ok(_), Some(code), _) => {
                failures.push(format!("cần lỗi {code} nhưng tính điểm thành công"))
            }
            (Err(errors), None, _) => failures.extend(errors.iter().map(|e| e.to_string())),
            (Ok(_), None, None) => failures.push("thiếu expect hoặc expect_error".into()),
            (Ok(sheet), None, Some(expect)) => {
                for (name, want) in &expect.field {
                    match sheet.field.iter().find(|v| &v.name == name) {
                        Some(got) if same(want, &got.value) => {}
                        Some(got) => {
                            failures.push(format!("field.{name}: cần {want:?}, nhận {}", got.value))
                        }
                        None => failures.push(format!("field.{name}: không có giá trị này")),
                    }
                }
                for (side, want) in &expect.sides {
                    let Some(got) = sheet.sides.get(side) else {
                        failures.push(format!("không có liên minh {side}"));
                        continue;
                    };
                    if let Some(s) = want.score {
                        if s != got.score {
                            failures.push(format!(
                                "{side}.score: cần {s}, nhận {} (thô {})",
                                got.score, got.raw_score
                            ));
                        }
                    }
                    for (name, value) in &want.values {
                        match got.values.iter().find(|v| &v.name == name) {
                            Some(item) if same(value, &item.value) => {}
                            Some(item) => failures
                                .push(format!("{side}.{name}: cần {value:?}, nhận {}", item.value)),
                            None => failures.push(format!("{side}.{name}: không có giá trị này")),
                        }
                    }
                    if let Some(fr) = &want.fouls_received {
                        if !same(fr, &got.fouls_received) {
                            failures.push(format!(
                                "{side}.fouls_received: cần {fr:?}, nhận {}",
                                got.fouls_received
                            ));
                        }
                    }
                    for (team, s) in &want.teams {
                        match got.teams.iter().find(|t| &t.id == team) {
                            Some(t) if t.team_score == *s => {}
                            Some(t) => failures.push(format!(
                                "{side}.teams.{team}: cần {s}, nhận {}",
                                t.team_score
                            )),
                            None => failures.push(format!("{side}.teams.{team}: không có đội")),
                        }
                    }
                }
            }
        }
        cases.push(CaseOutcome {
            name: case.name,
            file: shown,
            failures,
        });
    }
    Ok(PackReport {
        game: rules.file.game.clone(),
        cases,
    })
}
