//! Dựng `raso_rules::Observation` từ trạng thái mô phỏng theo `observe.yaml`.
//!
//! Mô phỏng chỉ *đo* (đếm vật phẩm trong vùng, đọc trạng thái leo); việc tính
//! điểm hoàn toàn thuộc về `rules.yaml` qua `raso_rules::score`.
use indexmap::IndexMap;
use raso_rules::schema::{InputDef, InputKind};
use raso_rules::{InputValue, Observation, Rules};
use serde::Deserialize;
use std::collections::BTreeMap;

pub trait ObservationSource {
    /// Loại vật phẩm duy nhất của gói (ví dụ `wildfire`).
    fn object_kind(&self) -> Option<String>;
    /// Số vật phẩm đang nằm trong target; `None` nếu target không có/đang tắt.
    fn count_in_target(&self, target: &str, released_by: Option<&str>) -> Option<i64>;
    /// `(liên minh, nhãn)` cho từng robot, nhãn ∈ none|contact|zone1|zone2|zone3.
    fn robot_brace_labels(&self) -> Vec<(String, String)>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Level {
    Field,
    Side,
    Robot,
}

impl Level {
    fn prefix(self) -> &'static str {
        match self {
            Level::Field => "field",
            Level::Side => "side",
            Level::Robot => "robot",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbeDef {
    #[serde(default)]
    count_objects: Option<String>,
    #[serde(default)]
    in_target: Option<String>,
    #[serde(default)]
    last_released_by: Option<String>,
    #[serde(default)]
    brace_state: Option<BTreeMap<String, String>>,
    #[serde(default)]
    not_simulated: Option<serde_yaml_ng::Value>,
}

#[derive(Debug, Clone)]
enum Probe {
    Count {
        object: String,
        target: String,
        released_by: Option<String>,
    },
    Brace(BTreeMap<String, String>),
    NotSimulated(InputValue),
}

#[derive(Debug, Clone)]
pub struct ObserveSpec {
    entries: Vec<(Level, String, Probe)>,
}

fn level_inputs(rules: &Rules, level: Level) -> &IndexMap<String, InputDef> {
    match level {
        Level::Field => &rules.file.inputs.field,
        Level::Side => &rules.file.inputs.side,
        Level::Robot => &rules.file.inputs.robot,
    }
}

fn yaml_to_input(v: &serde_yaml_ng::Value) -> Option<InputValue> {
    match v {
        serde_yaml_ng::Value::Bool(b) => Some(InputValue::Bool(*b)),
        serde_yaml_ng::Value::Number(n) => n.as_i64().map(InputValue::Int),
        serde_yaml_ng::Value::String(s) => Some(InputValue::Text(s.clone())),
        _ => None,
    }
}

/// Giá trị mặc định của ô `not_simulated` phải khớp kiểu ô trong rules.yaml,
/// nếu không biên bản mô phỏng sẽ bị `score` từ chối lúc hết trận.
fn value_fits(input: &InputDef, value: &InputValue) -> bool {
    match (input.kind, value) {
        (InputKind::Bool, InputValue::Bool(_)) => true,
        (InputKind::Count, InputValue::Int(n)) => *n >= 0 && input.max.is_none_or(|max| *n <= max),
        (InputKind::Choice, InputValue::Text(t)) => input.values.contains(t),
        _ => false,
    }
}

pub fn parse_observe(yaml: &str, rules: &Rules) -> Result<ObserveSpec, String> {
    let raw: BTreeMap<String, ProbeDef> =
        serde_yaml_ng::from_str(yaml).map_err(|e| format!("observe.yaml không hợp lệ: {e}"))?;
    let mut entries = Vec::new();
    for (key, def) in raw {
        let (level, name) = match key.split_once('.') {
            Some(("field", n)) => (Level::Field, n),
            Some(("side", n)) => (Level::Side, n),
            Some(("robot", n)) => (Level::Robot, n),
            _ => {
                return Err(format!(
                    "{key}: khoá phải có dạng field.<ô>, side.<ô> hoặc robot.<ô>"
                ));
            }
        };
        let input = level_inputs(rules, level)
            .get(name)
            .ok_or_else(|| format!("{key}: không có ô này trong rules.yaml"))?;
        let probe = match (&def.count_objects, &def.brace_state, &def.not_simulated) {
            (Some(object), None, None) => {
                let target = def
                    .in_target
                    .clone()
                    .ok_or_else(|| format!("{key}: count_objects cần in_target"))?;
                if input.kind != InputKind::Count || level == Level::Robot {
                    return Err(format!(
                        "{key}: count_objects chỉ dùng cho ô count cấp field/side"
                    ));
                }
                Probe::Count {
                    object: object.clone(),
                    target,
                    released_by: def.last_released_by.clone(),
                }
            }
            (None, Some(map), None) => {
                if level != Level::Robot || input.kind != InputKind::Choice {
                    return Err(format!(
                        "{key}: brace_state chỉ dùng cho ô choice cấp robot"
                    ));
                }
                for value in map.values() {
                    if !input.values.contains(value) {
                        return Err(format!("{key}: '{value}' không có trong values của ô"));
                    }
                }
                Probe::Brace(map.clone())
            }
            (None, None, Some(v)) => {
                let value = yaml_to_input(v)
                    .filter(|value| value_fits(input, value))
                    .ok_or_else(|| {
                        format!(
                            "{key}: not_simulated phải là giá trị hợp lệ của ô trong rules.yaml"
                        )
                    })?;
                Probe::NotSimulated(value)
            }
            _ => {
                return Err(format!(
                    "{key}: cần đúng một trong count_objects / brace_state / not_simulated"
                ));
            }
        };
        entries.push((level, name.to_string(), probe));
    }
    for level in [Level::Field, Level::Side, Level::Robot] {
        for name in level_inputs(rules, level).keys() {
            if !entries.iter().any(|(l, n, _)| *l == level && n == name) {
                return Err(format!(
                    "{}.{name}: ô của rules.yaml chưa có phép đo trong observe.yaml",
                    level.prefix()
                ));
            }
        }
    }
    Ok(ObserveSpec { entries })
}

/// Dựng biên bản từ mô phỏng. Biên bản luôn có đúng `sides.teams` robot mỗi
/// liên minh vì xuất phát từ `blank_observation`: robot thừa bị bỏ qua, robot
/// thiếu giữ nhãn `none`. `locked_robots` (chốt lúc còi kết thúc) thay cho
/// trạng thái leo đang sống.
pub fn build_observation(
    spec: &ObserveSpec,
    rules: &Rules,
    source: &dyn ObservationSource,
    locked_robots: Option<&[(String, String)]>,
) -> Observation {
    let mut obs = rules.blank_observation();
    obs.source = Some("simulation".into());
    let live_robots;
    let robots: &[(String, String)] = match locked_robots {
        Some(robots) => robots,
        None => {
            live_robots = source.robot_brace_labels();
            &live_robots
        }
    };
    let kind = source.object_kind();
    let count = |object: &str, target: &str, released_by: Option<&str>| {
        if kind.as_deref() == Some(object) {
            source.count_in_target(target, released_by)
        } else {
            None
        }
    };
    for (level, name, probe) in &spec.entries {
        let path = format!("{}.{name}", level.prefix());
        match (level, probe) {
            (_, Probe::NotSimulated(value)) => {
                obs.not_simulated.push(path);
                match level {
                    Level::Field => {
                        obs.field.insert(name.clone(), value.clone());
                    }
                    Level::Side => obs.sides.values_mut().for_each(|side| {
                        side.inputs.insert(name.clone(), value.clone());
                    }),
                    Level::Robot => obs
                        .sides
                        .values_mut()
                        .flat_map(|side| side.robots.iter_mut())
                        .for_each(|robot| {
                            robot.insert(name.clone(), value.clone());
                        }),
                }
            }
            (
                Level::Field,
                Probe::Count {
                    object,
                    target,
                    released_by,
                },
            ) => {
                let n = count(object, target, released_by.as_deref());
                if n.is_none() {
                    obs.not_simulated.push(path);
                }
                obs.field
                    .insert(name.clone(), InputValue::Int(n.unwrap_or(0)));
            }
            (
                Level::Side,
                Probe::Count {
                    object,
                    target,
                    released_by,
                },
            ) => {
                for (id, side) in obs.sides.iter_mut() {
                    let target = target.replace("{side}", id);
                    let released = released_by.as_ref().map(|r| r.replace("{side}", id));
                    let n = count(object, &target, released.as_deref());
                    if n.is_none() {
                        obs.not_simulated.push(path.clone());
                    }
                    side.inputs
                        .insert(name.clone(), InputValue::Int(n.unwrap_or(0)));
                }
            }
            (Level::Robot, Probe::Brace(map)) => {
                for (id, side) in obs.sides.iter_mut() {
                    let labels: Vec<&str> = robots
                        .iter()
                        .filter(|(side_id, _)| side_id == id)
                        .map(|(_, label)| label.as_str())
                        .collect();
                    for (slot, robot) in side.robots.iter_mut().enumerate() {
                        let label = labels.get(slot).copied().unwrap_or("none");
                        if let Some(value) = map.get(label) {
                            robot.insert(name.clone(), InputValue::Text(value.clone()));
                        }
                    }
                }
            }
            // parse_observe đã loại các tổ hợp còn lại.
            _ => {}
        }
    }
    obs.not_simulated.sort();
    obs.not_simulated.dedup();
    obs
}

#[cfg(test)]
mod tests {
    use super::*;
    use raso_rules::{NoParents, load, score};

    const RULES: &str = include_str!("../../../pkgs/games/fgc-2026/rules.yaml");
    const OBSERVE: &str = include_str!("../../../pkgs/games/fgc-2026/observe.yaml");

    struct Fake {
        red_su: i64,
        blue_su: i64,
        robots: Vec<(String, String)>,
    }

    impl ObservationSource for Fake {
        fn object_kind(&self) -> Option<String> {
            Some("wildfire".into())
        }
        fn count_in_target(&self, target: &str, released_by: Option<&str>) -> Option<i64> {
            match (target, released_by) {
                ("red-suppression-unit", Some("red")) => Some(self.red_su),
                ("blue-suppression-unit", Some("blue")) => Some(self.blue_su),
                ("extinguisher", _) => None, // đang tắt trong manifest
                _ => Some(0),
            }
        }
        fn robot_brace_labels(&self) -> Vec<(String, String)> {
            self.robots.clone()
        }
    }

    #[test]
    fn builds_a_valid_observation_and_pads_missing_robots() {
        let rules = load(RULES, &NoParents).unwrap();
        let spec = parse_observe(OBSERVE, &rules).unwrap();
        let fake = Fake {
            red_su: 100,
            blue_su: 0,
            robots: vec![
                ("red".into(), "zone2".into()),
                ("red".into(), "contact".into()),
            ],
        };
        let obs = build_observation(&spec, &rules, &fake, None);
        assert_eq!(obs.sides["red"].robots.len(), 3);
        assert_eq!(obs.sides["blue"].robots.len(), 3);
        assert_eq!(obs.source.as_deref(), Some("simulation"));
        assert!(
            obs.not_simulated
                .contains(&"field.ext_wildfire".to_string())
        );
        assert!(
            obs.not_simulated
                .contains(&"robot.lifted_by_partner".to_string())
        );
        let sheet = score(&rules, &obs).expect("biên bản mô phỏng phải hợp lệ");
        assert_eq!(sheet.sides["red"].score, 125); // ceil(100 * 1.25)
    }

    #[test]
    fn locked_robots_override_live_brace_state() {
        let rules = load(RULES, &NoParents).unwrap();
        let spec = parse_observe(OBSERVE, &rules).unwrap();
        let fake = Fake {
            red_su: 10,
            blue_su: 0,
            robots: vec![],
        };
        let locked = vec![("red".to_string(), "zone3".to_string())];
        let obs = build_observation(&spec, &rules, &fake, Some(&locked));
        assert_eq!(
            obs.sides["red"].robots[0]["climb"],
            raso_rules::InputValue::Text("ZONE_3".into())
        );
    }

    #[test]
    fn observe_must_cover_every_input_exactly_once() {
        let rules = load(RULES, &NoParents).unwrap();
        let missing = OBSERVE
            .lines()
            .filter(|l| !l.starts_with("robot.lifted_by_partner"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            parse_observe(&missing, &rules)
                .unwrap_err()
                .contains("robot.lifted_by_partner")
        );
        let extra = format!("{OBSERVE}\nside.bogus: {{ not_simulated: 0 }}\n");
        assert!(
            parse_observe(&extra, &rules)
                .unwrap_err()
                .contains("side.bogus")
        );
    }

    #[test]
    fn unknown_brace_label_mapping_is_rejected() {
        let rules = load(RULES, &NoParents).unwrap();
        let bad = OBSERVE.replace("zone3: ZONE_3", "zone3: ZONE_9");
        assert!(parse_observe(&bad, &rules).unwrap_err().contains("ZONE_9"));
    }

    #[test]
    #[ignore]
    fn benchmark_score_fgc_2026() {
        let rules = load(RULES, &NoParents).unwrap();
        let obs = rules.blank_observation();
        let started = std::time::Instant::now();
        for _ in 0..1_000 {
            score(&rules, &obs).unwrap();
        }
        // Tổng giây cho 1.000 lần = mili giây mỗi lần.
        let per_call_ms = started.elapsed().as_secs_f64();
        println!("score() fgc-2026: {per_call_ms:.4} ms/lần");
        assert!(per_call_ms < 1.0, "score() phải < 1 ms");
    }
}
