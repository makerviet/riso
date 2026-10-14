use crate::Rules;
use crate::error::ScoreError;
use crate::expr::{format_dynamic, to_dec};
use crate::graph::Node;
use crate::model::*;
use crate::schema::{CardEffect, GoesTo, InputDef, InputKind, Rounding};
use indexmap::IndexMap;
use rhai::{Array, Dynamic, EvalAltResult, Map, Scope};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use std::collections::BTreeMap;

fn input_to_dynamic(
    def: &InputDef,
    value: Option<&InputValue>,
    path: &str,
    errors: &mut Vec<ScoreError>,
) -> Dynamic {
    let Some(value) = value else {
        errors.push(ScoreError::new(
            "OBS_MISSING",
            path,
            "thiếu ô này trong biên bản",
        ));
        return Dynamic::UNIT;
    };
    match (def.kind, value) {
        (InputKind::Count, InputValue::Int(n)) => {
            if *n < 0 || def.max.is_some_and(|m| *n > m) {
                errors.push(ScoreError::new(
                    "OBS_RANGE",
                    path,
                    format!(
                        "giá trị {n} ngoài khoảng 0..{}",
                        def.max.map(|m| m.to_string()).unwrap_or("∞".into())
                    ),
                ));
            }
            Dynamic::from_decimal(Decimal::from(*n))
        }
        (InputKind::Count, other) => {
            errors.push(ScoreError::new(
                "OBS_RANGE",
                path,
                format!("cần số nguyên ≥ 0, nhận {other:?}"),
            ));
            Dynamic::UNIT
        }
        (InputKind::Bool, InputValue::Bool(b)) => Dynamic::from_bool(*b),
        (InputKind::Bool, other) => {
            errors.push(ScoreError::new(
                "OBS_CHOICE",
                path,
                format!("cần true/false, nhận {other:?}"),
            ));
            Dynamic::UNIT
        }
        (InputKind::Choice, InputValue::Text(s)) if def.values.contains(s) => {
            Dynamic::from(s.clone())
        }
        (InputKind::Choice, other) => {
            errors.push(ScoreError::new(
                "OBS_CHOICE",
                path,
                format!("cần một trong {:?}, nhận {other:?}", def.values),
            ));
            Dynamic::UNIT
        }
    }
}

fn read_inputs(
    defs: &IndexMap<String, InputDef>,
    given: &BTreeMap<String, InputValue>,
    prefix: &str,
    errors: &mut Vec<ScoreError>,
) -> Vec<(String, Dynamic)> {
    for name in given.keys() {
        if !defs.contains_key(name) {
            errors.push(ScoreError::new(
                "OBS_UNKNOWN",
                format!("{prefix}.{name}"),
                "ô không có trong luật",
            ));
        }
    }
    defs.iter()
        .map(|(name, def)| {
            (
                name.clone(),
                input_to_dynamic(def, given.get(name), &format!("{prefix}.{name}"), errors),
            )
        })
        .collect()
}

fn eval_node(
    rules: &Rules,
    scope: &mut Scope,
    node: &Node,
    path: &str,
) -> Result<Dynamic, ScoreError> {
    rules
        .engine
        .eval_ast_with_scope::<Dynamic>(scope, &node.compiled.ast)
        .map_err(|error| {
            let code = if matches!(*error, EvalAltResult::ErrorTooManyOperations(_)) {
                "EVAL_LIMIT"
            } else {
                "EVAL"
            };
            ScoreError::new(
                code,
                path,
                format!("lỗi khi tính '{}': {error}", node.compiled.source),
            )
        })
}

fn push_constants(rules: &Rules, scope: &mut Scope) {
    for (name, value) in &rules.constants {
        scope.push_constant_dynamic(name.clone(), value.clone());
    }
}

fn map_of(pairs: &[(String, Dynamic)]) -> Map {
    pairs
        .iter()
        .map(|(k, v)| (k.as_str().into(), v.clone()))
        .collect()
}

fn line(node: &Node, value: &Dynamic) -> LineItem {
    LineItem {
        name: node.name.clone(),
        label: node.label.clone(),
        label_en: node.label_en.clone(),
        value: format_dynamic(value),
    }
}

fn input_line(name: &str, def: &InputDef, value: &Dynamic) -> LineItem {
    LineItem {
        name: name.to_string(),
        label: Some(def.label.clone()),
        label_en: def.label_en.clone(),
        value: format_dynamic(value),
    }
}

fn round(raw: Decimal, rounding: Rounding) -> Option<i64> {
    let r = match rounding {
        Rounding::Ceil => raw.ceil(),
        Rounding::Floor => raw.floor(),
        Rounding::Round => raw.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero),
        Rounding::None => raw.trunc(),
    };
    r.to_i64()
}

struct SideState {
    inputs: Vec<(String, Dynamic)>,
    robots: Vec<Vec<(String, Dynamic)>>,
    values: Vec<(String, Dynamic)>,
}

pub fn score(rules: &Rules, obs: &Observation) -> Result<ScoreSheet, Vec<ScoreError>> {
    let file = &rules.file;
    let plan = &rules.plan;
    let mut errors = Vec::new();
    if obs.game != file.game {
        return Err(vec![ScoreError::new(
            "OBS_GAME",
            "game",
            format!("biên bản của đề '{}', luật là '{}'", obs.game, file.game),
        )]);
    }
    for id in obs.sides.keys() {
        if !file.sides.ids.contains(id) {
            errors.push(ScoreError::new(
                "OBS_UNKNOWN",
                format!("sides.{id}"),
                "liên minh không có trong luật",
            ));
        }
    }
    let empty = SideObservation {
        inputs: BTreeMap::new(),
        robots: Vec::new(),
        fouls: BTreeMap::new(),
        teams: Vec::new(),
    };
    let field_inputs = read_inputs(&file.inputs.field, &obs.field, "field", &mut errors);
    let mut sides: Vec<(String, SideState)> = Vec::new();
    for id in &file.sides.ids {
        let side_obs = obs.sides.get(id).unwrap_or(&empty);
        if !obs.sides.contains_key(id) {
            errors.push(ScoreError::new(
                "OBS_MISSING",
                format!("sides.{id}"),
                "thiếu liên minh",
            ));
        }
        let inputs = read_inputs(
            &file.inputs.side,
            &side_obs.inputs,
            &format!("sides.{id}"),
            &mut errors,
        );
        if side_obs.robots.len() != file.sides.teams {
            errors.push(ScoreError::new(
                "OBS_ROBOTS",
                format!("sides.{id}.robots"),
                format!(
                    "cần {} robot, biên bản có {}",
                    file.sides.teams,
                    side_obs.robots.len()
                ),
            ));
        }
        let robots = side_obs
            .robots
            .iter()
            .enumerate()
            .map(|(i, r)| {
                read_inputs(
                    &file.inputs.robot,
                    r,
                    &format!("sides.{id}.robots.{i}"),
                    &mut errors,
                )
            })
            .collect();
        for foul in side_obs.fouls.keys() {
            if !file.fouls.contains_key(foul) {
                errors.push(ScoreError::new(
                    "OBS_UNKNOWN",
                    format!("sides.{id}.fouls.{foul}"),
                    "loại lỗi không có trong luật",
                ));
            }
        }
        for (i, team) in side_obs.teams.iter().enumerate() {
            if let Some(card) = &team.card
                && !file.cards.contains_key(card)
            {
                errors.push(ScoreError::new(
                    "OBS_CHOICE",
                    format!("sides.{id}.teams.{i}.card"),
                    format!("thẻ '{card}' không có trong luật"),
                ));
            }
        }
        sides.push((
            id.clone(),
            SideState {
                inputs,
                robots,
                values: Vec::new(),
            },
        ));
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    // 1. Cấp robot.
    for (id, state) in sides.iter_mut() {
        for (index, robot) in state.robots.iter_mut().enumerate() {
            let mut scope = Scope::new();
            push_constants(rules, &mut scope);
            for (name, value) in robot.iter() {
                scope.push_dynamic(name.clone(), value.clone());
            }
            for node in &plan.robot {
                let value = eval_node(
                    rules,
                    &mut scope,
                    node,
                    &format!("values.robot.{} ({id} #{})", node.name, index + 1),
                )
                .map_err(|e| vec![e])?;
                scope.push_dynamic(node.name.clone(), value.clone());
                robot.push((node.name.clone(), value));
            }
        }
    }

    // 2. Cấp field.
    let robot_columns = |robots: &[Vec<(String, Dynamic)>]| -> Map {
        let mut columns: BTreeMap<String, Array> = BTreeMap::new();
        for robot in robots {
            for (name, value) in robot {
                columns.entry(name.clone()).or_default().push(value.clone());
            }
        }
        columns
            .into_iter()
            .map(|(k, v)| (k.into(), Dynamic::from_array(v)))
            .collect()
    };
    let all_robots: Vec<Vec<(String, Dynamic)>> =
        sides.iter().flat_map(|(_, s)| s.robots.clone()).collect();
    let mut field_scope = Scope::new();
    push_constants(rules, &mut field_scope);
    for (name, value) in &field_inputs {
        field_scope.push_dynamic(name.clone(), value.clone());
    }
    field_scope.push_constant("all_robots", robot_columns(&all_robots));
    for (id, state) in &sides {
        field_scope.push_constant(id.clone(), map_of(&state.inputs));
    }
    let mut field_values = Vec::new();
    for node in &plan.field {
        let value = eval_node(
            rules,
            &mut field_scope,
            node,
            &format!("values.field.{}", node.name),
        )
        .map_err(|e| vec![e])?;
        field_scope.push_dynamic(node.name.clone(), value.clone());
        field_values.push((node.name.clone(), value));
    }
    let field_map: Map = map_of(&field_inputs)
        .into_iter()
        .chain(map_of(&field_values))
        .collect();

    // 3. Cấp side.
    let side_inputs: Vec<(String, Map)> = sides
        .iter()
        .map(|(id, s)| (id.clone(), map_of(&s.inputs)))
        .collect();
    let make_side_scope = |id: &str, state: &SideState| -> Scope<'static> {
        let mut scope = Scope::new();
        push_constants(rules, &mut scope);
        for (name, value) in state.inputs.iter().chain(state.values.iter()) {
            scope.push_dynamic(name.clone(), value.clone());
        }
        scope.push_constant("robots", robot_columns(&state.robots));
        scope.push_constant("field", field_map.clone());
        for (other, inputs) in &side_inputs {
            scope.push_constant(other.clone(), inputs.clone());
            if other != id {
                scope.push_constant("opponent", inputs.clone());
            }
        }
        scope
    };
    for (id, state) in sides.iter_mut() {
        let mut scope = make_side_scope(id, state);
        for node in &plan.side {
            let value = eval_node(
                rules,
                &mut scope,
                node,
                &format!("values.side.{} ({id})", node.name),
            )
            .map_err(|e| vec![e])?;
            scope.push_dynamic(node.name.clone(), value.clone());
            state.values.push((node.name.clone(), value));
        }
    }

    // 4. Phạt.
    let mut received: BTreeMap<String, Decimal> = file
        .sides
        .ids
        .iter()
        .map(|id| (id.clone(), Decimal::ZERO))
        .collect();
    let mut committed = received.clone();
    let mut foul_lines: BTreeMap<String, Vec<FoulLine>> = BTreeMap::new();
    for (id, state) in &sides {
        let side_obs = obs.sides.get(id).unwrap_or(&empty);
        let Some(opponent) = file.sides.ids.iter().find(|other| *other != id).cloned() else {
            return Err(vec![ScoreError::new(
                "EVAL",
                format!("sides.{id}"),
                "không tìm thấy liên minh đối thủ",
            )]);
        };
        let mut scope = make_side_scope(id, state);
        for foul in &plan.fouls {
            let def = &file.fouls[&foul.id];
            let count = side_obs.fouls.get(&foul.id).copied().unwrap_or(0);
            let each = to_dec(
                &eval_node(
                    rules,
                    &mut scope,
                    &foul.node,
                    &format!("fouls.{} ({id})", foul.id),
                )
                .map_err(|e| vec![e])?,
            )
            .map_err(|e| {
                vec![ScoreError::new(
                    "EVAL",
                    format!("fouls.{}", foul.id),
                    e.to_string(),
                )]
            })?;
            let overflow = || {
                vec![ScoreError::new(
                    "EVAL",
                    format!("fouls.{} ({id})", foul.id),
                    "tràn số khi tính điểm phạt",
                )]
            };
            let total = each
                .checked_mul(Decimal::from(count))
                .ok_or_else(overflow)?;
            let (target, key) = match def.goes_to {
                GoesTo::Opponent => (&mut received, &opponent),
                GoesTo::SelfSide => (&mut committed, id),
            };
            let slot = target.get_mut(key).ok_or_else(overflow)?;
            *slot = slot.checked_add(total).ok_or_else(overflow)?;
            foul_lines.entry(id.clone()).or_default().push(FoulLine {
                id: foul.id.clone(),
                label: def.label.clone(),
                count,
                points_each: each.normalize().to_string(),
                total: total.normalize().to_string(),
                goes_to: match def.goes_to {
                    GoesTo::Opponent => "opponent".into(),
                    GoesTo::SelfSide => "self".into(),
                },
            });
        }
    }

    // 5. Điểm cuối + thẻ.
    let mut sheet_sides = BTreeMap::new();
    for (id, state) in &sides {
        let mut scope = make_side_scope(id, state);
        scope.push_constant("fouls_received", received[id]);
        scope.push_constant("fouls_committed", committed[id]);
        let raw = to_dec(
            &eval_node(rules, &mut scope, &plan.score, &format!("score ({id})"))
                .map_err(|e| vec![e])?,
        )
        .map_err(|e| vec![ScoreError::new("EVAL", "score", e.to_string())])?;
        let final_score = round(raw, file.rounding).ok_or_else(|| {
            vec![ScoreError::new(
                "EVAL",
                format!("score ({id})"),
                "điểm vượt phạm vi số nguyên",
            )]
        })?;
        let side_obs = obs.sides.get(id).unwrap_or(&empty);
        let teams = side_obs
            .teams
            .iter()
            .map(|team| {
                let zero = team
                    .card
                    .as_ref()
                    .and_then(|c| file.cards.get(c))
                    .is_some_and(|e| *e == CardEffect::TeamZero);
                TeamResult {
                    id: team.id.clone(),
                    card: team.card.clone(),
                    team_score: if zero { 0 } else { final_score },
                }
            })
            .collect();
        let mut values: Vec<LineItem> = file
            .inputs
            .side
            .iter()
            .zip(state.inputs.iter())
            .map(|((n, d), (_, v))| input_line(n, d, v))
            .collect();
        values.extend(
            plan.side
                .iter()
                .zip(state.values.iter())
                .map(|(n, (_, v))| line(n, v)),
        );
        let robots = state
            .robots
            .iter()
            .map(|robot| {
                let mut items: Vec<LineItem> = file
                    .inputs
                    .robot
                    .iter()
                    .zip(robot.iter())
                    .map(|((n, d), (_, v))| input_line(n, d, v))
                    .collect();
                let offset = file.inputs.robot.len();
                items.extend(
                    plan.robot
                        .iter()
                        .zip(robot.iter().skip(offset))
                        .map(|(n, (_, v))| line(n, v)),
                );
                items
            })
            .collect();
        sheet_sides.insert(
            id.clone(),
            SideSheet {
                values,
                robots,
                fouls: foul_lines.remove(id).unwrap_or_default(),
                fouls_received: received[id].normalize().to_string(),
                fouls_committed: committed[id].normalize().to_string(),
                raw_score: raw.normalize().to_string(),
                score: final_score,
                teams,
            },
        );
    }
    let mut field: Vec<LineItem> = file
        .inputs
        .field
        .iter()
        .zip(field_inputs.iter())
        .map(|((n, d), (_, v))| input_line(n, d, v))
        .collect();
    field.extend(
        plan.field
            .iter()
            .zip(field_values.iter())
            .map(|(n, (_, v))| line(n, v)),
    );
    Ok(ScoreSheet {
        game: file.game.clone(),
        rules_version: file.version.clone(),
        field,
        sides: sheet_sides,
        not_simulated: obs.not_simulated.clone(),
        scoring_error: None,
    })
}

impl Rules {
    pub fn blank_observation(&self) -> Observation {
        let blank = |defs: &IndexMap<String, InputDef>| -> BTreeMap<String, InputValue> {
            defs.iter()
                .map(|(name, def)| {
                    let v = match def.kind {
                        InputKind::Count => InputValue::Int(0),
                        InputKind::Bool => InputValue::Bool(false),
                        InputKind::Choice => InputValue::Text(def.values[0].clone()),
                    };
                    (name.clone(), v)
                })
                .collect()
        };
        Observation {
            game: self.file.game.clone(),
            rules_version: Some(self.file.version.clone()),
            field: blank(&self.file.inputs.field),
            sides: self
                .file
                .sides
                .ids
                .iter()
                .map(|id| {
                    (
                        id.clone(),
                        SideObservation {
                            inputs: blank(&self.file.inputs.side),
                            robots: (0..self.file.sides.teams)
                                .map(|_| blank(&self.file.inputs.robot))
                                .collect(),
                            fouls: BTreeMap::new(),
                            teams: Vec::new(),
                        },
                    )
                })
                .collect(),
            source: None,
            not_simulated: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::*;
    use crate::{NoParents, load, score};
    use std::collections::BTreeMap;

    const BASE: &str = include_str!("../tests/fixtures/base.yaml");

    fn obs(
        red_balls: i64,
        blue_balls: i64,
        red_parked: [bool; 2],
        blue_parked: [bool; 2],
    ) -> Observation {
        let side = |balls: i64, parked: [bool; 2]| SideObservation {
            inputs: BTreeMap::from([("balls".into(), InputValue::Int(balls))]),
            robots: parked
                .iter()
                .map(|p| BTreeMap::from([("parked".into(), InputValue::Bool(*p))]))
                .collect(),
            fouls: BTreeMap::new(),
            teams: Vec::new(),
        };
        Observation {
            game: "base-game".into(),
            rules_version: None,
            field: BTreeMap::new(),
            sides: BTreeMap::from([
                ("red".into(), side(red_balls, red_parked)),
                ("blue".into(), side(blue_balls, blue_parked)),
            ]),
            source: None,
            not_simulated: Vec::new(),
        }
    }

    #[test]
    fn computes_side_values_and_score() {
        let rules = load(BASE, &NoParents).unwrap();
        let sheet = score(&rules, &obs(10, 3, [true, false], [false, false])).unwrap();
        assert_eq!(sheet.sides["red"].score, 25); // 10*2 + 5*1
        assert_eq!(sheet.sides["blue"].score, 6);
        let red_values: BTreeMap<_, _> = sheet.sides["red"]
            .values
            .iter()
            .map(|v| (v.name.as_str(), v.value.as_str()))
            .collect();
        assert_eq!(red_values["ball_points_total"], "20");
        assert_eq!(red_values["park_bonus"], "5");
    }

    #[test]
    fn fouls_go_to_opponent() {
        let rules = load(BASE, &NoParents).unwrap();
        let mut o = obs(10, 3, [false, false], [false, false]);
        o.sides
            .get_mut("red")
            .unwrap()
            .fouls
            .insert("minor".into(), 2);
        let sheet = score(&rules, &o).unwrap();
        assert_eq!(sheet.sides["blue"].fouls_received, "6");
        assert_eq!(sheet.sides["blue"].score, 12);
        assert_eq!(sheet.sides["red"].score, 20);
        assert_eq!(sheet.sides["red"].fouls[0].total, "6");
    }

    #[test]
    fn red_card_zeroes_only_that_team() {
        let rules = load(BASE, &NoParents).unwrap();
        let mut o = obs(10, 0, [false, false], [false, false]);
        o.sides.get_mut("red").unwrap().teams = vec![
            TeamEntry {
                id: "VIE".into(),
                card: Some("red".into()),
            },
            TeamEntry {
                id: "LAO".into(),
                card: Some("yellow".into()),
            },
        ];
        let sheet = score(&rules, &o).unwrap();
        assert_eq!(sheet.sides["red"].score, 20);
        assert_eq!(sheet.sides["red"].teams[0].team_score, 0);
        assert_eq!(sheet.sides["red"].teams[1].team_score, 20);
    }

    #[test]
    fn observation_errors_name_the_input() {
        let rules = load(BASE, &NoParents).unwrap();
        let codes = |o: &Observation| {
            score(&rules, o)
                .unwrap_err()
                .iter()
                .map(|e| (e.code, e.path.clone()))
                .collect::<Vec<_>>()
        };

        let negative = obs(-1, 0, [false, false], [false, false]);
        assert_eq!(
            codes(&negative),
            vec![("OBS_RANGE", "sides.red.balls".to_string())]
        );

        let too_many = obs(51, 0, [false, false], [false, false]);
        assert_eq!(
            codes(&too_many),
            vec![("OBS_RANGE", "sides.red.balls".to_string())]
        );

        let mut missing_robot = obs(1, 1, [false, false], [false, false]);
        missing_robot.sides.get_mut("blue").unwrap().robots.pop();
        assert_eq!(
            codes(&missing_robot),
            vec![("OBS_ROBOTS", "sides.blue.robots".to_string())]
        );

        let mut missing_input = obs(1, 1, [false, false], [false, false]);
        missing_input.sides.get_mut("red").unwrap().inputs.clear();
        assert_eq!(
            codes(&missing_input),
            vec![("OBS_MISSING", "sides.red.balls".to_string())]
        );

        let mut unknown = obs(1, 1, [false, false], [false, false]);
        unknown
            .sides
            .get_mut("red")
            .unwrap()
            .inputs
            .insert("bals".into(), InputValue::Int(1));
        assert_eq!(
            codes(&unknown),
            vec![("OBS_UNKNOWN", "sides.red.bals".to_string())]
        );

        let mut wrong_type = obs(1, 1, [false, false], [false, false]);
        wrong_type
            .sides
            .get_mut("red")
            .unwrap()
            .inputs
            .insert("balls".into(), InputValue::Text("2.5".into()));
        assert_eq!(
            codes(&wrong_type),
            vec![("OBS_RANGE", "sides.red.balls".to_string())]
        );

        let mut bad_card = obs(1, 1, [false, false], [false, false]);
        bad_card.sides.get_mut("red").unwrap().teams = vec![TeamEntry {
            id: "X".into(),
            card: Some("purple".into()),
        }];
        assert_eq!(
            codes(&bad_card),
            vec![("OBS_CHOICE", "sides.red.teams.0.card".to_string())]
        );

        let mut bad_foul = obs(1, 1, [false, false], [false, false]);
        bad_foul
            .sides
            .get_mut("red")
            .unwrap()
            .fouls
            .insert("huge".into(), 1);
        assert_eq!(
            codes(&bad_foul),
            vec![("OBS_UNKNOWN", "sides.red.fouls.huge".to_string())]
        );

        let mut wrong_game = obs(1, 1, [false, false], [false, false]);
        wrong_game.game = "other".into();
        assert_eq!(codes(&wrong_game)[0].0, "OBS_GAME");
    }

    #[test]
    fn json_fractional_count_is_rejected_not_rounded() {
        let rules = load(BASE, &NoParents).unwrap();
        let json = r#"{"game":"base-game","sides":{"red":{"balls":2.5,"robots":[{"parked":false},{"parked":false}]},"blue":{"balls":1,"robots":[{"parked":false},{"parked":false}]}}}"#;
        let o: Observation = serde_json::from_str(json).unwrap();
        let errors = score(&rules, &o).unwrap_err();
        assert_eq!(errors[0].code, "OBS_RANGE");
    }

    #[test]
    fn division_by_zero_is_eval_error_with_value_name() {
        let yaml = BASE.replace("\"balls * ball_points\"", "\"balls / 0\"");
        let rules = load(&yaml, &NoParents).unwrap();
        let errors = score(&rules, &obs(1, 1, [false, false], [false, false])).unwrap_err();
        assert_eq!(errors[0].code, "EVAL");
        assert!(
            errors[0].path.contains("ball_points_total"),
            "{}",
            errors[0].path
        );
    }

    #[test]
    fn blank_observation_is_valid() {
        let rules = load(BASE, &NoParents).unwrap();
        let blank = rules.blank_observation();
        assert_eq!(blank.sides["red"].robots.len(), 2);
        let sheet = score(&rules, &blank).unwrap();
        assert_eq!(sheet.sides["red"].score, 0);
    }

    #[test]
    fn rounding_applies_to_final_score() {
        let yaml = BASE.replace("\"balls * ball_points\"", "\"balls * 0.3\"");
        let rules = load(&yaml, &NoParents).unwrap();
        let sheet = score(&rules, &obs(1, 0, [false, false], [false, false])).unwrap();
        assert_eq!(sheet.sides["red"].raw_score, "0.3");
        assert_eq!(sheet.sides["red"].score, 1);
    }

    #[test]
    fn overflowing_foul_is_error_not_panic() {
        let yaml = BASE.replace("points: \"3\"", "points: \"50000000000000000000000\"");
        let rules = load(&yaml, &NoParents).unwrap();
        let mut o = obs(1, 1, [false, false], [false, false]);
        o.sides
            .get_mut("red")
            .unwrap()
            .fouls
            .insert("minor".into(), 4_000_000);
        let errors = score(&rules, &o).unwrap_err();
        assert_eq!(errors[0].code, "EVAL");
        assert!(
            errors[0].path.starts_with("fouls.minor"),
            "{}",
            errors[0].path
        );
    }

    #[test]
    fn score_beyond_i64_is_error_not_saturated() {
        let yaml = BASE.replace(
            "\"balls * ball_points\"",
            "\"balls * 50000000000000000000\"",
        );
        let rules = load(&yaml, &NoParents).unwrap();
        let errors = score(&rules, &obs(1, 0, [false, false], [false, false])).unwrap_err();
        assert_eq!(errors[0].code, "EVAL");
        assert!(errors[0].path.starts_with("score"), "{}", errors[0].path);
    }

    #[test]
    fn yaml_null_cell_is_named_error() {
        let rules = load(BASE, &NoParents).unwrap();
        let yaml = "game: base-game\nsides:\n  red:\n    balls:\n    robots: [{parked: false}, {parked: false}]\n  blue:\n    balls: 1\n    robots: [{parked: false}, {parked: false}]\n";
        let o: Observation = serde_yaml_ng::from_str(yaml).unwrap();
        let errors = score(&rules, &o).unwrap_err();
        assert_eq!(
            (errors[0].code, errors[0].path.as_str()),
            ("OBS_RANGE", "sides.red.balls")
        );
    }

    #[test]
    fn self_goes_to_fouls_are_committed() {
        let yaml = BASE
            .replace("goes_to: opponent", "goes_to: self")
            .replace("+ fouls_received\"", "- fouls_committed\"");
        let rules = load(&yaml, &NoParents).unwrap();
        let mut o = obs(10, 3, [false, false], [false, false]);
        o.sides
            .get_mut("red")
            .unwrap()
            .fouls
            .insert("minor".into(), 2);
        let sheet = score(&rules, &o).unwrap();
        assert_eq!(sheet.sides["red"].fouls_committed, "6");
        assert_eq!(sheet.sides["red"].fouls_received, "0");
        assert_eq!(sheet.sides["red"].score, 14);
        assert_eq!(sheet.sides["blue"].score, 6);
        assert_eq!(sheet.sides["red"].fouls[0].goes_to, "self");
    }
}
