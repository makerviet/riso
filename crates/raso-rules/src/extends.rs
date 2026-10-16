use crate::error::RuleError;
use crate::schema::{InputKind, RulesFile};
use serde_yaml_ng::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EXTENDS_DEPTH: usize = 4;
pub const SCHEMA: &str = "raso.rules/v1";
/// Tên dành riêng cho phạm vi công thức (spec mục 4.3).
pub const RESERVED: [&str; 8] = [
    "robots",
    "all_robots",
    "field",
    "opponent",
    "matches",
    "fouls_received",
    "fouls_committed",
    "team_score",
];

pub trait PackResolver {
    fn rules_yaml(&self, game: &str) -> Option<String>;
}

pub struct MapResolver(pub BTreeMap<String, String>);

impl PackResolver for MapResolver {
    fn rules_yaml(&self, game: &str) -> Option<String> {
        self.0.get(game).cloned()
    }
}

pub struct NoParents;

impl PackResolver for NoParents {
    fn rules_yaml(&self, _game: &str) -> Option<String> {
        None
    }
}

fn parse_value(yaml: &str) -> Result<Value, Vec<RuleError>> {
    serde_yaml_ng::from_str::<Value>(yaml).map_err(|error| {
        let mut e = RuleError::new("RULE_YAML", "", format!("YAML không hợp lệ: {error}"));
        e.line = error.location().map(|l| l.line());
        vec![e]
    })
}

fn set_path(root: &mut Value, path: &str, value: Value) -> Result<(), RuleError> {
    let segments: Vec<&str> = path.split('.').collect();
    let mut node = root;
    for (index, segment) in segments.iter().enumerate() {
        let map = node.as_mapping_mut().ok_or_else(|| {
            RuleError::new(
                "RULE_EXTENDS",
                format!("override.{path}"),
                format!("'{segment}' không nằm trong một khối"),
            )
        })?;
        let key = Value::from(*segment);
        if index == segments.len() - 1 {
            map.insert(key, value);
            return Ok(());
        }
        if !map.contains_key(&key) {
            map.insert(key.clone(), Value::Mapping(Mapping::new()));
        }
        node = map.get_mut(&key).expect("vừa chèn");
    }
    Ok(())
}

fn resolve(
    yaml: &str,
    resolver: &dyn PackResolver,
    chain: &mut Vec<String>,
) -> Result<Value, Vec<RuleError>> {
    let mut value = parse_value(yaml)?;
    let map = value.as_mapping_mut().ok_or_else(|| {
        vec![RuleError::new(
            "RULE_YAML",
            "",
            "file luật phải là một khối khoá: giá trị",
        )]
    })?;
    let game = map
        .get("game")
        .and_then(Value::as_str)
        .unwrap_or("?")
        .to_string();
    if chain.contains(&game) {
        chain.push(game);
        return Err(vec![RuleError::new(
            "RULE_EXTENDS",
            "extends",
            format!("kế thừa vòng: {}", chain.join(" → ")),
        )]);
    }
    chain.push(game);
    let Some(parent_id) = map.remove("extends") else {
        map.remove("override");
        return Ok(value);
    };
    let parent_id = parent_id
        .as_str()
        .ok_or_else(|| {
            vec![RuleError::new(
                "RULE_EXTENDS",
                "extends",
                "extends phải là id đề",
            )]
        })?
        .to_string();
    if chain.len() > MAX_EXTENDS_DEPTH {
        return Err(vec![RuleError::new(
            "RULE_EXTENDS",
            "extends",
            format!("kế thừa quá {MAX_EXTENDS_DEPTH} tầng"),
        )]);
    }
    let parent_src = resolver.rules_yaml(&parent_id).ok_or_else(|| {
        vec![RuleError::new(
            "RULE_EXTENDS",
            "extends",
            format!("không tìm thấy đề cha '{parent_id}'"),
        )]
    })?;
    let mut merged = resolve(&parent_src, resolver, chain)?;
    let overrides = map.remove("override");
    let merged_map = merged.as_mapping_mut().expect("đề cha là khối");
    for (key, val) in map.iter() {
        merged_map.insert(key.clone(), val.clone());
    }
    if let Some(Value::Mapping(overrides)) = overrides {
        for (path, val) in overrides {
            let path = path.as_str().unwrap_or_default().to_string();
            set_path(&mut merged, &path, val).map_err(|e| vec![e])?;
        }
    }
    Ok(merged)
}

fn structural_checks(rules: &RulesFile) -> Vec<RuleError> {
    let mut errors = Vec::new();
    if rules.schema != SCHEMA {
        errors.push(RuleError::new(
            "RULE_SCHEMA",
            "schema",
            format!("chỉ hỗ trợ '{SCHEMA}'"),
        ));
    }
    if rules.sides.ids.len() != 2 {
        errors.push(RuleError::new(
            "RULE_SCHEMA",
            "sides.ids",
            "v1 cần đúng 2 liên minh",
        ));
    }
    let mut seen_ids = BTreeSet::new();
    for id in &rules.sides.ids {
        if !seen_ids.insert(id) {
            errors.push(RuleError::new(
                "RULE_SCHEMA",
                "sides.ids",
                format!("id liên minh '{id}' bị trùng"),
            ));
        }
        if RESERVED.contains(&id.as_str()) {
            errors.push(RuleError::new(
                "RULE_SCHEMA",
                "sides.ids",
                format!("id liên minh '{id}' trùng tên dành riêng"),
            ));
        }
    }
    if rules.sides.teams == 0 {
        errors.push(RuleError::new(
            "RULE_SCHEMA",
            "sides.teams",
            "mỗi liên minh cần ít nhất 1 đội",
        ));
    }
    for (level, inputs) in [
        ("field", &rules.inputs.field),
        ("side", &rules.inputs.side),
        ("robot", &rules.inputs.robot),
    ] {
        for (name, def) in inputs {
            let path = format!("inputs.{level}.{name}");
            if def.kind == InputKind::Choice && def.values.is_empty() {
                errors.push(RuleError::new(
                    "RULE_SCHEMA",
                    path.clone(),
                    "ô choice cần danh sách values",
                ));
            }
            if def.max.is_some_and(|m| m < 0) {
                errors.push(RuleError::new("RULE_SCHEMA", path, "max phải ≥ 0"));
            }
        }
    }
    let mut seen = BTreeSet::new();
    let names = rules
        .inputs
        .field
        .keys()
        .map(|n| ("inputs.field", n))
        .chain(rules.inputs.side.keys().map(|n| ("inputs.side", n)))
        .chain(rules.inputs.robot.keys().map(|n| ("inputs.robot", n)))
        .chain(rules.values.robot.keys().map(|n| ("values.robot", n)))
        .chain(rules.values.field.keys().map(|n| ("values.field", n)))
        .chain(rules.values.side.keys().map(|n| ("values.side", n)))
        .chain(rules.constants.keys().map(|n| ("constants", n)));
    for (block, name) in names {
        let path = format!("{block}.{name}");
        if RESERVED.contains(&name.as_str()) || rules.sides.ids.contains(name) {
            errors.push(RuleError::new(
                "RULE_NAME",
                path,
                format!("'{name}' là tên dành riêng"),
            ));
        } else if !seen.insert(name.clone()) {
            errors.push(RuleError::new(
                "RULE_NAME",
                path,
                format!("tên '{name}' bị trùng"),
            ));
        }
    }
    if rules.cards.keys().any(|id| id.is_empty()) {
        errors.push(RuleError::new("RULE_SCHEMA", "cards", "tên thẻ rỗng"));
    }
    if let (Some(_), Some(_)) = (&rules.ranking.drop_lowest, rules.ranking.keep_best) {
        errors.push(RuleError::new(
            "RULE_SCHEMA",
            "ranking",
            "drop_lowest và keep_best không dùng cùng nhau",
        ));
    }
    errors
}

pub fn parse_rules(yaml: &str, resolver: &dyn PackResolver) -> Result<RulesFile, Vec<RuleError>> {
    let merged = resolve(yaml, resolver, &mut Vec::new())?;
    let rules: RulesFile = serde_yaml_ng::from_value(merged).map_err(|error| {
        vec![RuleError::new(
            "RULE_YAML",
            "",
            format!("cấu trúc luật không hợp lệ: {error}"),
        )]
    })?;
    let errors: Vec<RuleError> = structural_checks(&rules)
        .into_iter()
        .map(|e| e.located(yaml))
        .collect();
    if errors.is_empty() {
        Ok(rules)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const BASE: &str = include_str!("../tests/fixtures/base.yaml");
    const CHILD: &str = include_str!("../tests/fixtures/child.yaml");

    fn resolver() -> MapResolver {
        MapResolver(BTreeMap::from([(
            "base-game".to_string(),
            BASE.to_string(),
        )]))
    }

    #[test]
    fn base_parses() {
        let rules = parse_rules(BASE, &NoParents).expect("base hợp lệ");
        assert_eq!(rules.game, "base-game");
        assert_eq!(rules.sides.ids, vec!["red", "blue"]);
        assert_eq!(rules.inputs.side.keys().collect::<Vec<_>>(), vec!["balls"]);
    }

    #[test]
    fn child_inherits_and_overrides() {
        let rules = parse_rules(CHILD, &resolver()).expect("child hợp lệ");
        assert_eq!(rules.game, "child-game");
        assert_eq!(rules.version, "0.1.0");
        assert_eq!(
            rules.constants["ball_points"],
            serde_yaml_ng::Value::from(3)
        );
        assert_eq!(
            rules.values.side["park_bonus"].expr(),
            "7 * count(robots.parked)"
        );
        assert_eq!(
            rules.values.side["ball_points_total"].expr(),
            "balls * ball_points"
        );
        assert_eq!(
            rules.score,
            "ball_points_total + park_bonus + fouls_received"
        );
    }

    #[test]
    fn missing_parent_is_rule_extends() {
        let errors = parse_rules(CHILD, &NoParents).unwrap_err();
        assert_eq!(errors[0].code, "RULE_EXTENDS");
        assert!(errors[0].message.contains("base-game"));
    }

    #[test]
    fn extends_cycle_is_detected() {
        let a = CHILD
            .replace("extends: base-game", "extends: b")
            .replace("game: child-game", "game: a");
        let b = CHILD
            .replace("extends: base-game", "extends: a")
            .replace("game: child-game", "game: b");
        let resolver = MapResolver(BTreeMap::from([("a".into(), a.clone()), ("b".into(), b)]));
        let errors = parse_rules(&a, &resolver).unwrap_err();
        assert_eq!(errors[0].code, "RULE_EXTENDS");
        assert!(errors[0].message.contains("vòng"), "{}", errors[0].message);
    }

    #[test]
    fn yaml_syntax_error_has_line() {
        let errors =
            parse_rules("schema: raso.rules/v1\ngame: [unclosed\n", &NoParents).unwrap_err();
        assert_eq!(errors[0].code, "RULE_YAML");
        assert!(errors[0].line.is_some());
    }

    #[test]
    fn wrong_schema_and_side_count_are_rejected() {
        let wrong = BASE.replace("raso.rules/v1", "raso.rules/v9");
        assert_eq!(
            parse_rules(&wrong, &NoParents).unwrap_err()[0].code,
            "RULE_SCHEMA"
        );
        let three = BASE.replace("ids: [red, blue]", "ids: [red, blue, green]");
        assert_eq!(
            parse_rules(&three, &NoParents).unwrap_err()[0].code,
            "RULE_SCHEMA"
        );
    }

    #[test]
    fn choice_without_values_is_rejected() {
        let bad = BASE.replace(
            "parked: { type: bool, label: \"Đỗ\" }",
            "parked: { type: choice, label: \"Đỗ\" }",
        );
        let errors = parse_rules(&bad, &NoParents).unwrap_err();
        assert_eq!(errors[0].code, "RULE_SCHEMA");
        assert_eq!(errors[0].path, "inputs.robot.parked");
        assert_eq!(errors[0].line, Some(12));
    }

    #[test]
    fn duplicate_and_reserved_names_are_rejected() {
        let dup = BASE.replace("ball_points_total:", "balls:");
        assert_eq!(
            parse_rules(&dup, &NoParents).unwrap_err()[0].code,
            "RULE_NAME"
        );
        let reserved = BASE.replace("ball_points_total:", "opponent:");
        assert_eq!(
            parse_rules(&reserved, &NoParents).unwrap_err()[0].code,
            "RULE_NAME"
        );
    }

    #[test]
    fn line_of_finds_nested_keys() {
        assert_eq!(
            crate::error::line_of(BASE, "values.side.park_bonus"),
            Some(18)
        );
        assert_eq!(crate::error::line_of(BASE, "nope.nothing"), None);
    }

    #[test]
    fn extends_depth_over_limit_is_rejected() {
        let mut map = BTreeMap::new();
        for i in 0..5 {
            let src = CHILD
                .replace("extends: base-game", &format!("extends: g{}", i + 1))
                .replace("game: child-game", &format!("game: g{i}"));
            map.insert(format!("g{i}"), src);
        }
        map.insert(
            "g5".to_string(),
            BASE.replace("game: base-game", "game: g5"),
        );
        let errors = parse_rules(&map["g0"].clone(), &MapResolver(map)).unwrap_err();
        assert_eq!(errors[0].code, "RULE_EXTENDS");
        assert!(errors[0].message.contains("quá"), "{}", errors[0].message);
    }

    #[test]
    fn override_through_non_mapping_is_rejected() {
        let bad =
            CHILD.split("override:").next().unwrap().to_string() + "override:\n  score.x: 1\n";
        let errors = parse_rules(&bad, &resolver()).unwrap_err();
        assert_eq!(errors[0].code, "RULE_EXTENDS");
        assert_eq!(errors[0].path, "override.score.x");
    }

    #[test]
    fn unknown_top_level_field_is_rejected() {
        let bad = format!("{BASE}bogus: 1\n");
        assert_eq!(
            parse_rules(&bad, &NoParents).unwrap_err()[0].code,
            "RULE_YAML"
        );
    }

    #[test]
    fn duplicate_or_reserved_side_ids_rejected() {
        for ids in ["[red, red]", "[field, blue]"] {
            let yaml = BASE.replace("ids: [red, blue]", &format!("ids: {ids}"));
            let errors = parse_rules(&yaml, &NoParents).unwrap_err();
            assert!(
                errors
                    .iter()
                    .any(|e| e.code == "RULE_SCHEMA" && e.path == "sides.ids"),
                "{ids}: {errors:?}"
            );
        }
    }
}
