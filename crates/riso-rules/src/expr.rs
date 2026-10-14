//! Công thức: tiền xử lý chữ số → `Decimal`, hộp cát Rhai, hàm có sẵn.
use rhai::{AST, Array, Dynamic, Engine, EvalAltResult, Map};
use rust_decimal::{Decimal, MathematicalOps, RoundingStrategy};

pub type EvalResult<T> = Result<T, Box<EvalAltResult>>;

pub const MAX_OPERATIONS: u64 = 10_000;
pub const MAX_EXPR_DEPTH: usize = 64;
const FORBIDDEN: [&str; 9] = ["while", "loop", "for", "fn", "import", "eval", "print", "debug", "do"];
const KEYWORDS: [&str; 4] = ["if", "else", "true", "false"];

#[derive(Debug, Clone, PartialEq)]
pub struct Scan {
    /// Công thức sau khi đổi mọi chữ số thành `__d("…")`.
    pub rewritten: String,
    /// Chuỗi tên được tham chiếu (`["field", "coop"]`), bỏ tên hàm và từ khoá.
    pub refs: Vec<Vec<String>>,
    /// Mọi từ đơn xuất hiện (kể cả tên hàm) — dùng để chặn từ cấm.
    pub words: Vec<String>,
}

pub fn scan(src: &str) -> Scan {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len() + 16);
    let mut refs = Vec::new();
    let mut words = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i = (i + 1).min(chars.len());
            out.extend(&chars[start..i]);
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            let literal: String = chars[start..i].iter().filter(|ch| **ch != '_').collect();
            out.push_str("__d(\"");
            out.push_str(&literal);
            out.push_str("\")");
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let mut chain = Vec::new();
            loop {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                chain.push(chars[start..i].iter().collect::<String>());
                if i + 1 < chars.len() && chars[i] == '.' && (chars[i + 1].is_alphabetic() || chars[i + 1] == '_') {
                    i += 1;
                    continue;
                }
                break;
            }
            out.push_str(&chain.join("."));
            words.push(chain[0].clone());
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let is_call = j < chars.len() && chars[j] == '(';
            if !is_call && !KEYWORDS.contains(&chain[0].as_str()) {
                refs.push(chain);
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    Scan { rewritten: out, refs, words }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    Forbidden(String),
    Syntax(String),
}

#[derive(Debug, Clone)]
pub struct Compiled {
    pub source: String,
    pub ast: AST,
    pub refs: Vec<Vec<String>>,
}

pub fn compile(engine: &Engine, src: &str) -> Result<Compiled, CompileError> {
    let scanned = scan(src);
    if let Some(word) = scanned.words.iter().find(|w| FORBIDDEN.contains(&w.as_str())) {
        return Err(CompileError::Forbidden(word.clone()));
    }
    let ast = engine
        .compile_expression(&scanned.rewritten)
        .map_err(|error| CompileError::Syntax(error.to_string()))?;
    Ok(Compiled { source: src.to_string(), ast, refs: scanned.refs })
}

pub fn to_dec(v: &Dynamic) -> EvalResult<Decimal> {
    if let Ok(d) = v.as_decimal() {
        return Ok(d);
    }
    if let Ok(i) = v.as_int() {
        return Ok(Decimal::from(i));
    }
    if let Ok(b) = v.as_bool() {
        return Ok(if b { Decimal::ONE } else { Decimal::ZERO });
    }
    Err(format!("giá trị không phải số: {}", v.type_name()).into())
}

pub fn format_dynamic(v: &Dynamic) -> String {
    if let Ok(d) = v.as_decimal() {
        return d.normalize().to_string();
    }
    if let Ok(b) = v.as_bool() {
        return b.to_string();
    }
    if let Ok(i) = v.as_int() {
        return i.to_string();
    }
    v.to_string()
}

fn decimals(values: &Array) -> EvalResult<Vec<Decimal>> {
    values.iter().map(to_dec).collect()
}

fn table_key(v: &Dynamic) -> EvalResult<String> {
    if v.is_string() {
        return Ok(v.clone().into_string().unwrap_or_default());
    }
    Ok(to_dec(v)?.normalize().to_string())
}

fn stddev(values: &[Decimal]) -> EvalResult<Decimal> {
    if values.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let n = Decimal::from(values.len() as i64);
    let mean = values.iter().sum::<Decimal>() / n;
    let variance = values.iter().map(|x| (*x - mean) * (*x - mean)).sum::<Decimal>() / n;
    variance.sqrt().ok_or_else(|| "không tính được căn bậc hai".into())
}

pub fn new_engine() -> Engine {
    let mut engine = Engine::new();
    for symbol in FORBIDDEN {
        engine.disable_symbol(symbol);
    }
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_EXPR_DEPTH);
    engine.set_max_call_levels(8);
    engine.set_max_array_size(10_000);
    engine.set_max_map_size(1_000);
    engine.set_max_string_size(1_000);
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});

    engine.register_fn("__d", |s: &str| -> EvalResult<Decimal> {
        s.parse::<Decimal>().map_err(|e| format!("số không hợp lệ '{s}': {e}").into())
    });
    engine.register_fn("ceil", |x: Decimal| x.ceil());
    engine.register_fn("floor", |x: Decimal| x.floor());
    engine.register_fn("round", |x: Decimal| {
        x.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
    });
    engine.register_fn("abs", |x: Decimal| x.abs());
    engine.register_fn("min", |a: Decimal, b: Decimal| a.min(b));
    engine.register_fn("max", |a: Decimal, b: Decimal| a.max(b));
    engine.register_fn("min", |a: Array| -> EvalResult<Decimal> {
        decimals(&a)?.into_iter().min().ok_or_else(|| "min của mảng rỗng".into())
    });
    engine.register_fn("max", |a: Array| -> EvalResult<Decimal> {
        decimals(&a)?.into_iter().max().ok_or_else(|| "max của mảng rỗng".into())
    });
    engine.register_fn("sum", |a: Array| -> EvalResult<Decimal> { Ok(decimals(&a)?.into_iter().sum()) });
    engine.register_fn("count", |a: Array| -> EvalResult<Decimal> {
        Ok(Decimal::from(decimals(&a)?.into_iter().filter(|x| !x.is_zero()).count() as i64))
    });
    engine.register_fn("len", |a: Array| Decimal::from(a.len() as i64));
    engine.register_fn("avg", |a: Array| -> EvalResult<Decimal> {
        let values = decimals(&a)?;
        if values.is_empty() {
            return Ok(Decimal::ZERO);
        }
        Ok(values.iter().sum::<Decimal>() / Decimal::from(values.len() as i64))
    });
    engine.register_fn("stddev", |a: Array| -> EvalResult<Decimal> { stddev(&decimals(&a)?) });
    engine.register_fn("lookup", |key: Dynamic, table: Map| -> EvalResult<Dynamic> {
        let k = table_key(&key)?;
        table
            .get(k.as_str())
            .cloned()
            .ok_or_else(|| format!("lookup: bảng không có khoá '{k}'").into())
    });
    engine.register_fn("steps", |n: Dynamic, table: Map| -> EvalResult<Decimal> {
        let n = to_dec(&n)?;
        let mut best: Option<(Decimal, Decimal)> = None;
        for (key, value) in &table {
            let threshold: Decimal = key
                .parse()
                .map_err(|_| format!("steps: khoá '{key}' không phải số"))?;
            if threshold <= n && best.is_none_or(|(t, _)| threshold > t) {
                best = Some((threshold, to_dec(value)?));
            }
        }
        Ok(best.map(|(_, v)| v).unwrap_or(Decimal::ZERO))
    });
    engine
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::{Map, Scope};

    fn eval(src: &str) -> Dynamic {
        let engine = new_engine();
        let compiled = compile(&engine, src).expect("compile");
        engine
            .eval_ast_with_scope::<Dynamic>(&mut Scope::new(), &compiled.ast)
            .expect("eval")
    }

    fn eval_with(scope: &mut Scope, src: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        let engine = new_engine();
        let compiled = compile(&engine, src).expect("compile");
        engine.eval_ast_with_scope::<Dynamic>(scope, &compiled.ast)
    }

    fn dec(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    #[test]
    fn ceil_of_twenty_times_one_point_zero_five_is_exactly_21() {
        assert_eq!(to_dec(&eval("ceil(20 * 1.05)")).unwrap(), dec("21"));
    }

    #[test]
    fn fifty_balls_with_one_zone1_robot_is_55_not_56() {
        // Máy tính JS chính thức ra 56 vì 50 * 1.1 = 55.00000000000001.
        assert_eq!(to_dec(&eval("ceil(50 * (1 + 0.10))")).unwrap(), dec("55"));
    }

    #[test]
    fn division_is_decimal_not_integer() {
        let third = to_dec(&eval("1 / 3")).unwrap();
        assert!(third > dec("0.333") && third < dec("0.334"), "{third}");
    }

    #[test]
    fn scan_rewrites_numbers_and_collects_references() {
        let s = scan("su_wildfire * climb_mult + field.coop + count(robots.in_zone3) + 0.05");
        assert_eq!(
            s.rewritten,
            "su_wildfire * climb_mult + field.coop + count(robots.in_zone3) + __d(\"0.05\")"
        );
        let refs: Vec<String> = s.refs.iter().map(|r| r.join(".")).collect();
        assert_eq!(refs, vec!["su_wildfire", "climb_mult", "field.coop", "robots.in_zone3"]);
        assert!(s.words.contains(&"count".to_string()));
    }

    #[test]
    fn scan_skips_strings_keywords_and_digits_inside_identifiers() {
        let s = scan("if climb == \"ZONE_3\" { 1 } else { 0 }");
        assert_eq!(s.rewritten, "if climb == \"ZONE_3\" { __d(\"1\") } else { __d(\"0\") }");
        let refs: Vec<String> = s.refs.iter().map(|r| r.join(".")).collect();
        assert_eq!(refs, vec!["climb"]);
        let z = scan("ZONE_3 + x2");
        assert_eq!(z.rewritten, "ZONE_3 + x2");
    }

    #[test]
    fn forbidden_constructs_are_rejected() {
        let engine = new_engine();
        for src in ["while true { 1 }", "print(1)", "eval(\"1\")", "loop { 1 }"] {
            assert!(
                matches!(compile(&engine, src), Err(CompileError::Forbidden(_))),
                "{src} phải bị cấm"
            );
        }
    }

    #[test]
    fn syntax_errors_are_reported() {
        let engine = new_engine();
        assert!(matches!(compile(&engine, "1 +"), Err(CompileError::Syntax(_))));
    }

    #[test]
    fn aggregate_builtins() {
        assert_eq!(to_dec(&eval("sum([1, 2.5, true])")).unwrap(), dec("4.5"));
        assert_eq!(to_dec(&eval("count([0, 1, true, false, 3])")).unwrap(), dec("3"));
        assert_eq!(to_dec(&eval("len([0, 0, 0])")).unwrap(), dec("3"));
        assert_eq!(to_dec(&eval("avg([1, 2, 3, 4])")).unwrap(), dec("2.5"));
        assert_eq!(to_dec(&eval("max(2, 7)")).unwrap(), dec("7"));
        assert_eq!(to_dec(&eval("min([4, 2, 9])")).unwrap(), dec("2"));
        let sigma = to_dec(&eval("stddev([30, 40, 50])")).unwrap();
        assert!((sigma - dec("8.16496580927726")).abs() < dec("0.0000001"), "{sigma}");
        assert_eq!(to_dec(&eval("stddev([40, 40, 40])")).unwrap(), Decimal::ZERO);
    }

    #[test]
    fn rounding_is_half_away_from_zero() {
        assert_eq!(to_dec(&eval("round(2.5)")).unwrap(), dec("3"));
        assert_eq!(to_dec(&eval("round(-2.5)")).unwrap(), dec("-3"));
        assert_eq!(to_dec(&eval("floor(2.9)")).unwrap(), dec("2"));
        assert_eq!(to_dec(&eval("abs(-4)")).unwrap(), dec("4"));
    }

    #[test]
    fn lookup_and_steps_use_tables() {
        let mut table = Map::new();
        table.insert("ZONE_3".into(), Dynamic::from_decimal(dec("0.30")));
        let mut coop = Map::new();
        coop.insert("4".into(), Dynamic::from_decimal(dec("10")));
        coop.insert("5".into(), Dynamic::from_decimal(dec("25")));
        coop.insert("6".into(), Dynamic::from_decimal(dec("40")));
        let mut scope = Scope::new();
        scope.push_constant("climb_increment", table);
        scope.push_constant("coop_bonus", coop);
        scope.push("climb", "ZONE_3".to_string());
        assert_eq!(
            to_dec(&eval_with(&mut scope, "lookup(climb, climb_increment)").unwrap()).unwrap(),
            dec("0.30")
        );
        assert_eq!(to_dec(&eval_with(&mut scope, "steps(3, coop_bonus)").unwrap()).unwrap(), Decimal::ZERO);
        assert_eq!(to_dec(&eval_with(&mut scope, "steps(4, coop_bonus)").unwrap()).unwrap(), dec("10"));
        assert_eq!(to_dec(&eval_with(&mut scope, "steps(6, coop_bonus)").unwrap()).unwrap(), dec("40"));
        assert_eq!(to_dec(&eval_with(&mut scope, "steps(9, coop_bonus)").unwrap()).unwrap(), dec("40"));
        let missing = eval_with(&mut scope, "lookup(\"NOPE\", climb_increment)").unwrap_err();
        assert!(missing.to_string().contains("NOPE"), "{missing}");
    }

    #[test]
    fn if_expression_returns_decimal() {
        assert_eq!(to_dec(&eval("if 4 >= 4 { 10 } else { 0.5 }")).unwrap(), dec("10"));
    }

    #[test]
    fn engine_limits_are_set() {
        let engine = new_engine();
        assert_eq!(engine.max_operations(), MAX_OPERATIONS);
    }

    #[test]
    fn format_dynamic_normalizes_decimals() {
        assert_eq!(format_dynamic(&Dynamic::from_decimal(dec("1.500"))), "1.5");
        assert_eq!(format_dynamic(&Dynamic::from_bool(true)), "true");
        assert_eq!(format_dynamic(&Dynamic::from("ZONE_1".to_string())), "ZONE_1");
    }
}
