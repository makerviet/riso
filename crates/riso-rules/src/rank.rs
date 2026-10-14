use crate::Rules;
use crate::error::ScoreError;
use crate::expr::to_dec;
use crate::model::ScoreSheet;
use crate::schema::{Aggregate, FinalTiebreak};
use rhai::{Array, Dynamic, Map, Scope};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchResult {
    pub match_id: String,
    pub sheet: ScoreSheet,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingRow {
    pub rank: usize,
    pub team: String,
    pub ranking_score: String,
    pub tiebreakers: Vec<String>,
    pub played: usize,
    pub counted_matches: Vec<String>,
    pub tied: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Standings {
    pub rows: Vec<StandingRow>,
}

struct Entry {
    match_id: String,
    team_score: Decimal,
    card: Option<String>,
    side_values: BTreeMap<String, Decimal>,
}

fn parse_value(v: &str) -> Decimal {
    match v {
        "true" => Decimal::ONE,
        "false" => Decimal::ZERO,
        other => other.parse().unwrap_or(Decimal::ZERO),
    }
}

pub fn rank(rules: &Rules, results: &[MatchResult]) -> Result<Standings, Vec<ScoreError>> {
    let ranking = &rules.file.ranking;
    let mut by_team: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
    for result in results {
        for side in result.sheet.sides.values() {
            let side_values: BTreeMap<String, Decimal> = side
                .values
                .iter()
                .map(|v| (v.name.clone(), parse_value(&v.value)))
                .collect();
            for team in &side.teams {
                by_team.entry(team.id.clone()).or_default().push(Entry {
                    match_id: result.match_id.clone(),
                    team_score: Decimal::from(team.team_score),
                    card: team.card.clone(),
                    side_values: side_values.clone(),
                });
            }
        }
    }

    let mut rows: Vec<(StandingRow, Decimal, Vec<Decimal>)> = Vec::new();
    for (team, entries) in by_team {
        let mut order: Vec<usize> = (0..entries.len()).collect();
        let counted: Vec<usize> = if let Some(k) = ranking.keep_best {
            order.sort_by(|a, b| {
                entries[*b]
                    .team_score
                    .cmp(&entries[*a].team_score)
                    .then(a.cmp(b))
            });
            order.into_iter().take(k).collect()
        } else if let Some(drop) = &ranking.drop_lowest {
            order.sort_by(|a, b| {
                entries[*a]
                    .team_score
                    .cmp(&entries[*b].team_score)
                    .then(a.cmp(b))
            });
            let mut dropped = Vec::new();
            if entries.len() > drop.count {
                for &i in &order {
                    if dropped.len() == drop.count {
                        break;
                    }
                    let protected = entries[i]
                        .card
                        .as_ref()
                        .is_some_and(|c| drop.not_if.contains(c));
                    if !protected {
                        dropped.push(i);
                    }
                }
            }
            (0..entries.len())
                .filter(|i| !dropped.contains(i))
                .collect()
        } else {
            order
        };
        let mut counted = counted;
        counted.sort();
        let overflow = || {
            vec![ScoreError::new(
                "EVAL",
                format!("ranking ({team})"),
                "tổng điểm xếp hạng vượt giới hạn số".to_string(),
            )]
        };
        let mut total = Decimal::ZERO;
        for &i in &counted {
            total = total
                .checked_add(entries[i].team_score)
                .ok_or_else(overflow)?;
        }
        let score = match ranking.aggregate {
            Aggregate::Sum => total,
            Aggregate::Average if counted.is_empty() => Decimal::ZERO,
            Aggregate::Average => total
                .checked_div(Decimal::from(counted.len() as i64))
                .ok_or_else(overflow)?,
        };

        let mut columns: BTreeMap<String, Array> = BTreeMap::new();
        for &i in &counted {
            columns
                .entry("team_score".into())
                .or_default()
                .push(Dynamic::from_decimal(entries[i].team_score));
            for (name, value) in &entries[i].side_values {
                columns
                    .entry(name.clone())
                    .or_default()
                    .push(Dynamic::from_decimal(*value));
            }
        }
        let matches: Map = columns
            .into_iter()
            .map(|(k, v)| (k.into(), Dynamic::from_array(v)))
            .collect();
        let mut tiebreak_values = Vec::new();
        for node in &rules.plan.tiebreakers {
            let mut scope = Scope::new();
            scope.push_constant("matches", matches.clone());
            let value = rules
                .engine
                .eval_ast_with_scope::<Dynamic>(&mut scope, &node.compiled.ast)
                .map_err(|e| {
                    let code = if matches!(*e, rhai::EvalAltResult::ErrorTooManyOperations(_)) {
                        "EVAL_LIMIT"
                    } else {
                        "EVAL"
                    };
                    vec![ScoreError::new(
                        code,
                        format!("ranking.tiebreakers ({})", node.name),
                        e.to_string(),
                    )]
                })?;
            tiebreak_values.push(to_dec(&value).map_err(|e| {
                vec![ScoreError::new(
                    "EVAL",
                    "ranking.tiebreakers",
                    e.to_string(),
                )]
            })?);
        }
        rows.push((
            StandingRow {
                rank: 0,
                team,
                ranking_score: score.normalize().to_string(),
                tiebreakers: tiebreak_values
                    .iter()
                    .map(|d| d.normalize().to_string())
                    .collect(),
                played: entries.len(),
                counted_matches: counted
                    .iter()
                    .map(|&i| entries[i].match_id.clone())
                    .collect(),
                tied: false,
            },
            score,
            tiebreak_values,
        ));
    }

    let primary = |a: &(StandingRow, Decimal, Vec<Decimal>),
                   b: &(StandingRow, Decimal, Vec<Decimal>)| {
        b.1.cmp(&a.1).then_with(|| {
            a.2.iter()
                .zip(b.2.iter())
                .map(|(x, y)| y.cmp(x))
                .find(|o| *o != Ordering::Equal)
                .unwrap_or(Ordering::Equal)
        })
    };
    let team_number = |a: &str, b: &str| match (a.parse::<i64>(), b.parse::<i64>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        _ => a.cmp(b),
    };
    rows.sort_by(|a, b| primary(a, b).then_with(|| team_number(&a.0.team, &b.0.team)));
    for i in 0..rows.len() {
        let equal_prev = i > 0 && primary(&rows[i - 1], &rows[i]) == Ordering::Equal;
        let equal_next = i + 1 < rows.len() && primary(&rows[i], &rows[i + 1]) == Ordering::Equal;
        let draw = ranking.final_tiebreak == FinalTiebreak::Draw;
        rows[i].0.rank = if draw && equal_prev {
            rows[i - 1].0.rank
        } else {
            i + 1
        };
        rows[i].0.tied = draw && (equal_prev || equal_next);
    }
    Ok(Standings {
        rows: rows.into_iter().map(|r| r.0).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::{NoParents, load, score};

    const BASE: &str = include_str!("../tests/fixtures/base.yaml");

    /// Một trận: đội `red_team` bên đỏ ghi `red_balls`, đội `blue_team` bên xanh ghi `blue_balls`.
    fn played(
        rules: &Rules,
        id: &str,
        red_team: (&str, Option<&str>),
        red_balls: i64,
        blue_team: (&str, Option<&str>),
        blue_balls: i64,
    ) -> MatchResult {
        let mut o = rules.blank_observation();
        let set = |o: &mut Observation, side: &str, team: (&str, Option<&str>), balls: i64| {
            let s = o.sides.get_mut(side).unwrap();
            s.inputs.insert("balls".into(), InputValue::Int(balls));
            s.teams = vec![TeamEntry {
                id: team.0.into(),
                card: team.1.map(str::to_string),
            }];
        };
        set(&mut o, "red", red_team, red_balls);
        set(&mut o, "blue", blue_team, blue_balls);
        MatchResult {
            match_id: id.into(),
            sheet: score(rules, &o).unwrap(),
        }
    }

    #[test]
    fn keep_best_sums_top_matches() {
        // base.yaml: aggregate sum, keep_best 2, tiebreaker max(team_score).
        let rules = load(BASE, &NoParents).unwrap();
        let results = vec![
            played(&rules, "m1", ("A", None), 10, ("B", None), 1),
            played(&rules, "m2", ("A", None), 1, ("B", None), 9),
            played(&rules, "m3", ("A", None), 5, ("B", None), 9),
        ];
        let standings = rank(&rules, &results).unwrap();
        let a = standings.rows.iter().find(|r| r.team == "A").unwrap();
        let b = standings.rows.iter().find(|r| r.team == "B").unwrap();
        assert_eq!(a.ranking_score, "30"); // 20 + 10
        assert_eq!(b.ranking_score, "36"); // 18 + 18
        assert_eq!(standings.rows[0].team, "B");
        assert_eq!(a.counted_matches, vec!["m1", "m3"]);
        assert_eq!(a.played, 3);
    }

    #[test]
    fn drop_lowest_skips_red_card_match_and_averages() {
        let yaml = BASE.replace(
            "  aggregate: sum\n  keep_best: 2",
            "  aggregate: average\n  drop_lowest: { count: 1, not_if: [red] }",
        );
        let rules = load(&yaml, &NoParents).unwrap();
        let results = vec![
            played(&rules, "m1", ("A", Some("red")), 1, ("B", None), 1),
            played(&rules, "m2", ("A", None), 3, ("B", None), 1),
            played(&rules, "m3", ("A", None), 5, ("B", None), 1),
        ];
        let standings = rank(&rules, &results).unwrap();
        let a = standings.rows.iter().find(|r| r.team == "A").unwrap();
        // m1 = 0 (thẻ đỏ) không được bỏ → bỏ m2 (6) → trung bình (0 + 10) / 2 = 5
        assert_eq!(a.counted_matches, vec!["m1", "m3"]);
        assert_eq!(a.ranking_score, "5");
    }

    #[test]
    fn single_match_is_not_dropped() {
        let yaml = BASE.replace(
            "  aggregate: sum\n  keep_best: 2",
            "  aggregate: average\n  drop_lowest: { count: 1 }",
        );
        let rules = load(&yaml, &NoParents).unwrap();
        let standings = rank(
            &rules,
            &[played(&rules, "m1", ("A", None), 4, ("B", None), 2)],
        )
        .unwrap();
        assert_eq!(
            standings
                .rows
                .iter()
                .find(|r| r.team == "A")
                .unwrap()
                .ranking_score,
            "8"
        );
    }

    #[test]
    fn tiebreaker_then_draw() {
        let rules = load(BASE, &NoParents).unwrap();
        let results = vec![
            played(&rules, "m1", ("A", None), 10, ("B", None), 5),
            played(&rules, "m2", ("A", None), 0, ("B", None), 5),
            played(&rules, "m3", ("C", None), 5, ("D", None), 5),
            played(&rules, "m4", ("C", None), 5, ("D", None), 5),
        ];
        let standings = rank(&rules, &results).unwrap();
        // A, B, C, D đều tổng 20; A có trận cao nhất 20 → hạng 1; B, C, D bằng nhau mọi tiêu chí.
        assert_eq!(standings.rows[0].team, "A");
        assert_eq!(standings.rows[0].rank, 1);
        assert!(standings.rows[1..].iter().all(|r| r.rank == 2 && r.tied));
    }
    #[test]
    fn team_number_final_tiebreak_orders_numerically_without_ties() {
        let yaml = BASE.replace(
            "  tiebreakers:",
            "  final_tiebreak: team_number\n  tiebreakers:",
        );
        let rules = load(&yaml, &NoParents).unwrap();
        let results = vec![
            played(&rules, "m1", ("10", None), 5, ("9", None), 5),
            played(&rules, "m2", ("10", None), 5, ("9", None), 5),
        ];
        let standings = rank(&rules, &results).unwrap();
        let order: Vec<_> = standings
            .rows
            .iter()
            .map(|r| (r.team.as_str(), r.rank, r.tied))
            .collect();
        assert_eq!(order, vec![("9", 1, false), ("10", 2, false)]);
    }
}
