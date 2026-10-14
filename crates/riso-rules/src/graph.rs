use crate::error::RuleError;
use crate::expr::{CompileError, Compiled, compile};
use crate::schema::{RulesFile, ValueDef};
use indexmap::IndexMap;
use rhai::Engine;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub name: String,
    pub label: Option<String>,
    pub label_en: Option<String>,
    pub compiled: Compiled,
}

#[derive(Debug, Clone)]
pub(crate) struct FoulNode {
    pub id: String,
    pub node: Node,
}

#[derive(Debug, Clone)]
pub(crate) struct Plan {
    pub robot: Vec<Node>,
    pub field: Vec<Node>,
    pub side: Vec<Node>,
    pub fouls: Vec<FoulNode>,
    pub score: Node,
    #[allow(dead_code)] // dùng ở Task 5
    pub tiebreakers: Vec<Node>,
}

#[derive(Clone, Copy, PartialEq)]
enum Level {
    Robot,
    Field,
    Side,
    Foul,
    Score,
    Tiebreak,
}

struct Names<'a> {
    rules: &'a RulesFile,
}

impl Names<'_> {
    fn robot_names(&self) -> BTreeSet<&str> {
        self.rules.inputs.robot.keys().chain(self.rules.values.robot.keys()).map(String::as_str).collect()
    }
    fn field_names(&self) -> BTreeSet<&str> {
        self.rules.inputs.field.keys().chain(self.rules.values.field.keys()).map(String::as_str).collect()
    }
    fn side_inputs(&self) -> BTreeSet<&str> {
        self.rules.inputs.side.keys().map(String::as_str).collect()
    }
    fn side_names(&self) -> BTreeSet<&str> {
        self.rules.inputs.side.keys().chain(self.rules.values.side.keys()).map(String::as_str).collect()
    }
    fn constants(&self) -> BTreeSet<&str> {
        self.rules.constants.keys().map(String::as_str).collect()
    }
    fn everything(&self) -> BTreeSet<&str> {
        let mut all = self.robot_names();
        all.extend(self.field_names());
        all.extend(self.side_names());
        all.extend(self.constants());
        all
    }

    /// `Ok(())` nếu chuỗi tên hợp lệ ở cấp; `Err((code, message))` nếu không.
    fn check(&self, level: Level, chain: &[String]) -> Result<(), (&'static str, String)> {
        let root = chain[0].as_str();
        let member = chain.get(1).map(String::as_str);
        let shown = chain.join(".");
        let known = self.everything();
        let constants = self.constants();
        if constants.contains(root) {
            return Ok(());
        }
        let unknown = |m: &str| ("RULE_UNKNOWN_NAME", format!("không có tên '{m}' (trong '{shown}')"));
        let scope = || ("RULE_SCOPE", format!("'{shown}' không dùng được ở cấp này"));
        let member_in = |set: BTreeSet<&str>, m: Option<&str>| -> Result<(), (&'static str, String)> {
            match m {
                Some(m) if set.contains(m) => Ok(()),
                Some(m) if known.contains(m) => Err(scope()),
                Some(m) => Err(unknown(m)),
                None => Err(scope()),
            }
        };
        let is_side_id = self.rules.sides.ids.iter().any(|id| id == root);
        match level {
            Level::Tiebreak => {
                if root != "matches" {
                    return Err(if known.contains(root) { scope() } else { unknown(root) });
                }
                let mut set = self.side_names();
                set.insert("team_score");
                member_in(set, member)
            }
            Level::Robot => {
                if member.is_none() && self.robot_names().contains(root) {
                    return Ok(());
                }
                Err(if known.contains(root) || matches!(root, "robots" | "all_robots" | "field" | "opponent") || is_side_id { scope() } else { unknown(root) })
            }
            Level::Field => {
                if member.is_none() && self.field_names().contains(root) {
                    return Ok(());
                }
                if root == "all_robots" {
                    return member_in(self.robot_names(), member);
                }
                if is_side_id {
                    return member_in(self.side_inputs(), member);
                }
                Err(if known.contains(root) || matches!(root, "robots" | "field" | "opponent") { scope() } else { unknown(root) })
            }
            Level::Side | Level::Foul | Level::Score => {
                if member.is_none() && self.side_names().contains(root) {
                    return Ok(());
                }
                if member.is_none() && matches!(root, "fouls_received" | "fouls_committed") {
                    return if level == Level::Score {
                        Ok(())
                    } else if level == Level::Foul {
                        Err(("RULE_FOUL_REF", format!("điểm phạt không được dùng '{root}'")))
                    } else {
                        Err(scope())
                    };
                }
                match root {
                    "robots" => member_in(self.robot_names(), member),
                    "field" => member_in(self.field_names(), member),
                    "opponent" => member_in(self.side_inputs(), member),
                    _ if is_side_id => member_in(self.side_inputs(), member),
                    _ => Err(if known.contains(root) || root == "all_robots" { scope() } else { unknown(root) }),
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn compile_node(
    engine: &Engine,
    names: &Names,
    level: Level,
    path: &str,
    name: &str,
    def_expr: &str,
    label: Option<&str>,
    label_en: Option<&str>,
    errors: &mut Vec<RuleError>,
) -> Option<Node> {
    let compiled = match compile(engine, def_expr) {
        Ok(compiled) => compiled,
        Err(CompileError::Forbidden(word)) => {
            errors.push(RuleError::new("RULE_FORBIDDEN", path, format!("không được dùng '{word}' trong công thức")));
            return None;
        }
        Err(CompileError::Syntax(message)) => {
            errors.push(RuleError::new("RULE_SYNTAX", path, format!("công thức sai cú pháp: {message}")));
            return None;
        }
    };
    let mut ok = true;
    for chain in &compiled.refs {
        if let Err((code, message)) = names.check(level, chain) {
            errors.push(RuleError::new(code, path, message));
            ok = false;
        }
    }
    ok.then(|| Node {
        name: name.to_string(),
        label: label.map(str::to_string),
        label_en: label_en.map(str::to_string),
        compiled,
    })
}

/// Kahn: giữ thứ tự khai báo khi không có phụ thuộc.
fn topo(level_name: &str, nodes: Vec<Node>, errors: &mut Vec<RuleError>) -> Vec<Node> {
    let names: BTreeSet<String> = nodes.iter().map(|n| n.name.clone()).collect();
    let deps: Vec<BTreeSet<String>> = nodes
        .iter()
        .map(|n| {
            n.compiled
                .refs
                .iter()
                .filter(|chain| chain.len() == 1 && names.contains(&chain[0]) && chain[0] != n.name)
                .map(|chain| chain[0].clone())
                .collect()
        })
        .collect();
    let mut done: BTreeSet<String> = BTreeSet::new();
    let mut ordered = Vec::with_capacity(nodes.len());
    let mut remaining: Vec<usize> = (0..nodes.len()).collect();
    while !remaining.is_empty() {
        let Some(pos) = remaining.iter().position(|&i| deps[i].iter().all(|d| done.contains(d))) else {
            let stuck: Vec<String> = remaining.iter().map(|&i| nodes[i].name.clone()).collect();
            errors.push(RuleError::new(
                "RULE_CYCLE",
                format!("values.{level_name}.{}", stuck[0]),
                format!("phụ thuộc vòng giữa: {}", stuck.join(", ")),
            ));
            return Vec::new();
        };
        let index = remaining.remove(pos);
        done.insert(nodes[index].name.clone());
        ordered.push(nodes[index].clone());
    }
    for node in &nodes {
        if node.compiled.refs.iter().any(|c| c.len() == 1 && c[0] == node.name) {
            errors.push(RuleError::new("RULE_CYCLE", format!("values.{level_name}.{}", node.name), "giá trị tự tham chiếu"));
        }
    }
    ordered
}

fn level_nodes(
    engine: &Engine,
    names: &Names,
    level: Level,
    level_name: &str,
    defs: &IndexMap<String, ValueDef>,
    errors: &mut Vec<RuleError>,
) -> Vec<Node> {
    let nodes: Vec<Node> = defs
        .iter()
        .filter_map(|(name, def)| {
            compile_node(engine, names, level, &format!("values.{level_name}.{name}"), name, def.expr(), def.label(), def.label_en(), errors)
        })
        .collect();
    topo(level_name, nodes, errors)
}

pub(crate) fn build_plan(engine: &Engine, rules: &RulesFile, src: &str) -> Result<Plan, Vec<RuleError>> {
    let names = Names { rules };
    let mut errors = Vec::new();
    let robot = level_nodes(engine, &names, Level::Robot, "robot", &rules.values.robot, &mut errors);
    let field = level_nodes(engine, &names, Level::Field, "field", &rules.values.field, &mut errors);
    let side = level_nodes(engine, &names, Level::Side, "side", &rules.values.side, &mut errors);
    let fouls: Vec<FoulNode> = rules
        .fouls
        .iter()
        .filter_map(|(id, foul)| {
            compile_node(engine, &names, Level::Foul, &format!("fouls.{id}"), id, &foul.points, Some(&foul.label), foul.label_en.as_deref(), &mut errors)
                .map(|node| FoulNode { id: id.clone(), node })
        })
        .collect();
    let score = compile_node(engine, &names, Level::Score, "score", "score", &rules.score, None, None, &mut errors);
    let tiebreakers: Vec<Node> = rules
        .ranking
        .tiebreakers
        .iter()
        .enumerate()
        .filter_map(|(i, t)| compile_node(engine, &names, Level::Tiebreak, &format!("ranking.tiebreakers.{i}"), &t.label, &t.expr, Some(&t.label), None, &mut errors))
        .collect();
    if !errors.is_empty() {
        return Err(errors.into_iter().map(|e| e.located(src)).collect());
    }
    Ok(Plan { robot, field, side, fouls, score: score.expect("không lỗi thì có score"), tiebreakers })
}

#[cfg(test)]
mod tests {
    use crate::{NoParents, load};

    const BASE: &str = include_str!("../tests/fixtures/base.yaml");

    fn codes(yaml: &str) -> Vec<&'static str> {
        load(yaml, &NoParents).err().unwrap_or_default().iter().map(|e| e.code).collect()
    }

    #[test]
    fn base_fixture_loads_and_orders_values() {
        let rules = load(BASE, &NoParents).expect("hợp lệ");
        let side: Vec<&str> = rules.plan.side.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(side, vec!["ball_points_total", "park_bonus"]);
    }

    #[test]
    fn dependencies_are_sorted_topologically() {
        let yaml = BASE.replace(
            "    ball_points_total: { expr: \"balls * ball_points\", label: \"Điểm bóng\" }",
            "    doubled: \"ball_points_total * 2\"\n    ball_points_total: { expr: \"balls * ball_points\", label: \"Điểm bóng\" }",
        );
        let rules = load(&yaml, &NoParents).expect("hợp lệ");
        let side: Vec<&str> = rules.plan.side.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(side, vec!["ball_points_total", "doubled", "park_bonus"]);
    }

    #[test]
    fn cycle_is_rule_cycle() {
        let yaml = BASE.replace("\"balls * ball_points\"", "\"park_bonus + 1\"").replace(
            "\"5 * count(robots.parked)\"",
            "\"ball_points_total + 1\"",
        );
        assert_eq!(codes(&yaml), vec!["RULE_CYCLE"]);
    }

    #[test]
    fn unknown_name_is_reported_with_path_and_line() {
        let yaml = BASE.replace("\"balls * ball_points\"", "\"bals * ball_points\"");
        let errors = load(&yaml, &NoParents).unwrap_err();
        assert_eq!(errors[0].code, "RULE_UNKNOWN_NAME");
        assert_eq!(errors[0].path, "values.side.ball_points_total");
        assert!(errors[0].message.contains("bals"));
        assert!(errors[0].line.is_some());
    }

    #[test]
    fn robot_input_used_directly_at_side_level_is_rule_scope() {
        let yaml = BASE.replace("\"5 * count(robots.parked)\"", "\"5 * parked\"");
        assert_eq!(codes(&yaml), vec!["RULE_SCOPE"]);
    }

    #[test]
    fn opponent_can_only_read_inputs() {
        let ok = BASE.replace("\"5 * count(robots.parked)\"", "\"opponent.balls\"");
        assert!(load(&ok, &NoParents).is_ok());
        let bad = BASE.replace("\"5 * count(robots.parked)\"", "\"opponent.ball_points_total\"");
        assert_eq!(codes(&bad), vec!["RULE_SCOPE"]);
    }

    #[test]
    fn foul_points_cannot_reference_fouls_or_score() {
        let yaml = BASE.replace("points: \"3\"", "points: \"fouls_received * 0.1\"");
        assert_eq!(codes(&yaml), vec!["RULE_FOUL_REF"]);
    }

    #[test]
    fn score_may_use_fouls_received() {
        assert!(load(BASE, &NoParents).is_ok());
    }

    #[test]
    fn forbidden_syntax_in_value_is_rule_forbidden() {
        let yaml = BASE.replace("\"balls * ball_points\"", "\"print(balls)\"");
        assert_eq!(codes(&yaml), vec!["RULE_FORBIDDEN"]);
    }

    #[test]
    fn tiebreaker_must_use_matches() {
        let yaml = BASE.replace("max(matches.team_score)", "max(balls)");
        assert_eq!(codes(&yaml), vec!["RULE_SCOPE"]);
        let unknown = BASE.replace("max(matches.team_score)", "max(matches.nope)");
        assert_eq!(codes(&unknown), vec!["RULE_UNKNOWN_NAME"]);
    }
}
