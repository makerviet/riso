# RASO `raso-rules` — Kế hoạch thực hiện (Giai đoạn 1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Một bộ máy tính điểm khai báo bằng `rules.yaml`, dùng chung cho trận mô phỏng (server Rust) và trận thật (trang trọng tài chạy WASM), với hai đề mẫu FGC 2026 và FGC 2025.

**Architecture:** Crate Rust thuần `crates/raso-rules` (đọc YAML → kế hoạch tính theo cấp robot → field → side → phạt → điểm cuối; công thức là biểu thức Rhai được tiền xử lý để mọi chữ số là `Decimal`). Server gọi crate trực tiếp, đo sân thành "biên bản" (`Observation`) theo `observe.yaml`. Trang `/referee/[gameId]` gọi bản WASM của cùng crate.

**Tech Stack:** Rust 2024, rhai 1.25 (`decimal`, `sync`), rust_decimal 1.43 (`maths`), serde_yaml_ng 0.10, indexmap 2, wasm-bindgen 0.2 + wasm-pack; SvelteKit 5 (runes), Hono (Cloudflare Worker), vitest.

**Spec:** `docs/superpowers/specs/2026-10-15-raso-rules-design.md` — đọc cùng kế hoạch này.

## Global Constraints

- Schema luật: `schema: raso.rules/v1`; đúng 2 liên minh (`sides.ids` dài 2).
- Mọi số trong công thức và ô biên bản là `rust_decimal::Decimal`; chữ số ngoài chuỗi được đổi thành `__d("…")` trước khi biên dịch. **Không** bật feature `no_float` của Rhai (Cargo gộp feature với server, sẽ tắt số thực của `arena.rhai`/`robot.rhai`).
- Rhai features đúng: `["decimal", "sync"]`; target wasm32 thêm `"wasm-bindgen"` và `getrandom = { version = "0.2", features = ["js"] }`.
- Hộp cát: cấm `while`, `loop`, `for`, `fn`, `import`, `eval`, `print`, `debug`, `do`; tối đa 10.000 phép toán/công thức; độ sâu biểu thức tối đa 64.
- Ô `count` là số nguyên `i64` ≥ 0 và ≤ `max`.
- `<side_id>.<tên>` và `opponent.<tên>` chỉ truy cập **ô biên bản cấp side**, không truy cập giá trị tính.
- `round()` làm tròn nửa xa số 0 (2,5 → 3), không làm tròn kiểu ngân hàng.
- Giao thức FGS1: section mới tag **14** `SCORE_SHEET` (JSON UTF-8), minor version **5 → 6**; section 9 `SCORE` giữ nguyên định dạng.
- Thông báo lỗi cho người viết luật bằng tiếng Việt, có mã lỗi cố định (spec mục 10).
- Commit kết thúc bằng hai dòng:
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` và `Claude-Session: https://claude.ai/code/session_017BLJKL45w4XQThvcefcCsK`.
- Mỗi test quan trọng phải được chứng minh **có thể đỏ**: sau khi xanh, cố tình làm hỏng code tương ứng, chạy lại thấy đỏ, rồi hoàn tác (bài học dự án NEO_WallE). Mỗi task có bước "Phá thử".

## Review Focus

1. **Luật có công thức tham chiếu sai cấp** (ví dụ dùng ô `robot` trực tiếp ở cấp `side`, hoặc `opponent.su_points` là giá trị tính) → phải bị `load` từ chối với `RULE_SCOPE`, không được lọt đến lúc tính rồi báo lỗi khó hiểu. Test ở Task 3.
2. **Biên bản thiếu robot / thừa robot / thiếu ô / ô lạ** (trọng tài bấm thiếu, mô phỏng có ít hơn 3 người chơi) → trả `OBS_ROBOTS`/`OBS_MISSING`/`OBS_UNKNOWN` có tên ô; server phải tự đệm robot thiếu bằng `blank_observation`. Test ở Task 4 và Task 9.
3. **Số thực JS lọt vào chuỗi tính** (trang trọng tài gửi `0.1+0.2` hoặc số lẻ cho ô `count`) → `count` nhận số không nguyên phải lỗi `OBS_RANGE`, không làm tròn ngầm. Test ở Task 4.
4. **Công thức điểm phạt tham chiếu `fouls_received`/`score`** (vòng lặp ngầm qua hai liên minh) → `RULE_FOUL_REF`. Test ở Task 3.
5. **Đề có `extends` trỏ tới đề không tồn tại hoặc tạo vòng** → `RULE_EXTENDS` với tên đề, không treo/không tràn ngăn xếp. Test ở Task 2.

---

## Bản đồ file

| File | Trách nhiệm |
|---|---|
| `crates/raso-rules/Cargo.toml` | Crate, features `cli`, `wasm` |
| `crates/raso-rules/src/lib.rs` | `Rules`, `load`, `score`, `rank`, re-export |
| `crates/raso-rules/src/error.rs` | `RuleError`, `ScoreError`, mã lỗi, `line_of` |
| `crates/raso-rules/src/expr.rs` | `scan` (tiền xử lý + trích tên), `new_engine`, hàm có sẵn, `compile` |
| `crates/raso-rules/src/schema.rs` | Kiểu `RulesFile` (serde) |
| `crates/raso-rules/src/extends.rs` | `PackResolver`, `resolve` (kế thừa + override) |
| `crates/raso-rules/src/graph.rs` | Kiểm tra phạm vi tên, sắp topo, `Plan` |
| `crates/raso-rules/src/model.rs` | `Observation`, `ScoreSheet` và các kiểu con |
| `crates/raso-rules/src/eval.rs` | `score()` |
| `crates/raso-rules/src/rank.rs` | `rank()` |
| `crates/raso-rules/src/pack.rs` | `DirResolver`, `check_pack` (không biên dịch cho wasm32) |
| `crates/raso-rules/src/bin/raso.rs` | CLI |
| `crates/raso-rules/src/wasm.rs` | `RulesHandle` (wasm-bindgen) |
| `crates/raso-rules/tests/packs.rs` | Chạy test của mọi đề trong `pkgs/games` |
| `pkgs/games/index.json` | Danh sách đề có luật |
| `pkgs/games/fgc-2026/{rules.yaml,observe.yaml,tests/scoring/*.yaml}` | Đề FGC 2026 |
| `pkgs/games/fgc-2025/{manifest.json,rules.yaml,tests/scoring/*.yaml}` | Đề FGC 2025 (chỉ luật) |
| `server/src/game/scoring_bridge.rs` | `ObserveSpec`, `ObservationSource`, `build_observation` |
| `server/src/game/sphere_runtime/scoring.rs` | Thay bằng `count_in_target`, `robot_brace_labels` |
| `server/src/game/match_registry.rs` | Gọi `score` mỗi 30 tick, chốt robot lúc còi, kết quả cuối |
| `server/src/game/match_registry/protocol.rs` | Section 14 |
| `api/src/routes/rules.ts` | `GET /api/rules`, `GET /api/rules/:id` |
| `api/src/routes/game-packs.ts` | `runtime` trả `rules`, `observe` |
| `web/src/routes/(public)/referee/...` | Trang trọng tài |
| `web/src/lib/raso.ts` | Nạp WASM + tải luật |
| `web/src/routes/(game)/match/test-match/match-protocol.ts`, `ScoreSheetPanel.svelte` | Giải mã + hiển thị section 14 |
| `.github/workflows/ci.yml` | Job `rules` |

---

### Task 1: Crate `raso-rules` và lõi biểu thức

**Files:**
- Create: `crates/raso-rules/Cargo.toml`
- Create: `crates/raso-rules/src/lib.rs`
- Create: `crates/raso-rules/src/expr.rs`
- Modify: `.gitignore` (thêm `crates/raso-rules/target/`, `crates/raso-rules/pkg/`)

**Interfaces:**
- Produces:
  - `pub struct Scan { pub rewritten: String, pub refs: Vec<Vec<String>>, pub words: Vec<String> }`
  - `pub fn scan(src: &str) -> Scan`
  - `pub fn new_engine() -> rhai::Engine`
  - `pub enum CompileError { Forbidden(String), Syntax(String) }`
  - `pub struct Compiled { pub source: String, pub ast: rhai::AST, pub refs: Vec<Vec<String>> }`
  - `pub fn compile(engine: &Engine, src: &str) -> Result<Compiled, CompileError>`
  - `pub fn to_dec(v: &Dynamic) -> Result<Decimal, Box<EvalAltResult>>`
  - `pub fn format_dynamic(v: &Dynamic) -> String`

- [ ] **Step 1: Tạo crate**

`crates/raso-rules/Cargo.toml`:

```toml
[package]
name = "raso-rules"
version = "0.1.0"
edition = "2024"
license = "MIT"
description = "RASO — bộ máy tính điểm khai báo bằng YAML cho giải robot"

[lib]
crate-type = ["rlib", "cdylib"]

[[bin]]
name = "raso"
path = "src/bin/raso.rs"
required-features = ["cli"]

[features]
default = []
cli = []
wasm = ["dep:wasm-bindgen"]

[dependencies]
rhai = { version = "1.25.1", features = ["decimal", "sync"] }
rust_decimal = { version = "1.43", features = ["maths"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml_ng = "0.10"
indexmap = { version = "2", features = ["serde"] }
wasm-bindgen = { version = "0.2", optional = true }

[target.'cfg(target_arch = "wasm32")'.dependencies]
rhai = { version = "1.25.1", features = ["decimal", "sync", "wasm-bindgen"] }
getrandom = { version = "0.2", features = ["js"] }

[profile.release]
lto = "thin"
opt-level = "s"
```

`crates/raso-rules/src/lib.rs` (tạm thời):

```rust
//! RASO — bộ máy tính điểm khai báo bằng YAML.
pub mod expr;
```

Thêm vào `.gitignore` ở gốc repo:

```
crates/raso-rules/target/
crates/raso-rules/pkg/
```

- [ ] **Step 2: Viết test thất bại cho `scan`, số thập phân và hộp cát**

Tạo `crates/raso-rules/src/expr.rs` chỉ với phần test (code chính viết ở Step 4):

```rust
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
```

- [ ] **Step 3: Chạy test để thấy thất bại**

Run: `cd crates/raso-rules && cargo test --lib expr`
Expected: lỗi biên dịch `cannot find function scan` / `new_engine`.

- [ ] **Step 4: Viết phần code của `expr.rs`** (đặt phía trên khối `#[cfg(test)]`)

```rust
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
```

Cập nhật `lib.rs`:

```rust
//! RASO — bộ máy tính điểm khai báo bằng YAML.
pub mod expr;
```

- [ ] **Step 5: Chạy test**

Run: `cd crates/raso-rules && cargo test --lib expr`
Expected: 13 test PASS.

Nếu `min`/`max` dạng `(Decimal, Decimal)` xung đột với hàm có sẵn của Rhai thì test `aggregate_builtins` sẽ đỏ; khi đó đổi tên đăng ký thành `min`/`max` chỉ cho `Array` và thêm `min2`/`max2`, đồng thời cập nhật spec mục 4.3.

- [ ] **Step 6: Phá thử**

Lần lượt: (a) bỏ dòng `out.push_str("__d(\"")…` (để chữ số nguyên dạng float) → `ceil_of_twenty…` hoặc `division_is_decimal…` phải đỏ; (b) đổi `MidpointAwayFromZero` thành `MidpointNearestEven` → `rounding_is_half_away_from_zero` đỏ; (c) xoá `"print"` khỏi `FORBIDDEN` → `forbidden_constructs_are_rejected` đỏ. Hoàn tác từng lần.

- [ ] **Step 7: Commit**

```bash
git add crates/raso-rules .gitignore
git commit -m "feat(rules): lõi biểu thức Decimal + hộp cát cho raso-rules"
```

---

### Task 2: Schema `rules.yaml`, kế thừa và lỗi có số dòng

**Files:**
- Create: `crates/raso-rules/src/error.rs`
- Create: `crates/raso-rules/src/schema.rs`
- Create: `crates/raso-rules/src/extends.rs`
- Create: `crates/raso-rules/tests/fixtures/base.yaml`, `crates/raso-rules/tests/fixtures/child.yaml`
- Modify: `crates/raso-rules/src/lib.rs`

**Interfaces:**
- Consumes: không.
- Produces:
  - `pub struct RuleError { pub code: &'static str, pub path: String, pub line: Option<usize>, pub message: String }` (derive `Debug, Clone, PartialEq, Serialize`)
  - `pub struct ScoreError { pub code: &'static str, pub path: String, pub message: String }` (cùng derive)
  - `pub fn line_of(src: &str, path: &str) -> Option<usize>` (1-based)
  - `pub struct RulesFile { schema, game, name, version, source, sides: Sides, rounding: Rounding, inputs: Inputs, constants: IndexMap<String, serde_yaml_ng::Value>, values: Values, fouls: IndexMap<String, Foul>, cards: IndexMap<String, CardEffect>, score: String, ranking: Ranking, open_questions: Vec<OpenQuestion> }`
  - Các kiểu con: `Sides { ids: Vec<String>, teams: usize }`, `Rounding { Ceil, Floor, Round, None }`, `Inputs { field, side, robot: IndexMap<String, InputDef> }`, `InputDef { kind: InputKind, max: Option<i64>, values: Vec<String>, label: String, label_en: Option<String>, help: Option<String> }`, `InputKind { Count, Choice, Bool }`, `Values { robot, field, side: IndexMap<String, ValueDef> }`, `ValueDef` (untagged `Expr(String)` | `Full { expr, label, label_en }`) với `fn expr(&self) -> &str`, `fn label(&self) -> Option<&str>`, `fn label_en(&self) -> Option<&str>`, `Foul { label, label_en, points: String, goes_to: GoesTo }`, `GoesTo { Opponent, SelfSide }`, `CardEffect { TeamZero, None }`, `Ranking { aggregate: Aggregate, drop_lowest: Option<DropLowest>, keep_best: Option<usize>, tiebreakers: Vec<Tiebreaker>, final_tiebreak: FinalTiebreak }`, `Aggregate { Average, Sum }`, `DropLowest { count: usize, not_if: Vec<String> }`, `Tiebreaker { label, expr }`, `FinalTiebreak { Draw, TeamNumber }`, `OpenQuestion { id, used, source }`.
  - `pub trait PackResolver { fn rules_yaml(&self, game: &str) -> Option<String>; }`
  - `pub struct MapResolver(pub BTreeMap<String, String>)` impl `PackResolver`
  - `pub struct NoParents;` impl `PackResolver` (luôn `None`)
  - `pub fn parse_rules(yaml: &str, resolver: &dyn PackResolver) -> Result<RulesFile, Vec<RuleError>>` — kế thừa + deserialize + kiểm tra cấu trúc (chưa kiểm tra công thức).

- [ ] **Step 1: Viết fixture kế thừa**

`crates/raso-rules/tests/fixtures/base.yaml`:

```yaml
schema: raso.rules/v1
game: base-game
name: Base
version: 1.0.0
source: "fixture"
sides: { ids: [red, blue], teams: 2 }
rounding: ceil
inputs:
  side:
    balls: { type: count, max: 50, label: "Bóng" }
  robot:
    parked: { type: bool, label: "Đỗ" }
constants:
  ball_points: 2
values:
  side:
    ball_points_total: { expr: "balls * ball_points", label: "Điểm bóng" }
    park_bonus: { expr: "5 * count(robots.parked)", label: "Đỗ xe" }
score: "ball_points_total + park_bonus + fouls_received"
fouls:
  minor: { label: "Lỗi", points: "3", goes_to: opponent }
cards: { red: team_zero, yellow: none }
ranking:
  aggregate: sum
  keep_best: 2
  tiebreakers:
    - { label: "Trận cao nhất", expr: "max(matches.team_score)" }
```

`crates/raso-rules/tests/fixtures/child.yaml`:

```yaml
schema: raso.rules/v1
extends: base-game
game: child-game
name: Child
version: 0.1.0
source: "fixture con"
override:
  constants.ball_points: 3
  values.side.park_bonus: { expr: "7 * count(robots.parked)", label: "Đỗ xe (con)" }
```

- [ ] **Step 2: Viết test thất bại** — tạo `crates/raso-rules/src/extends.rs` với khối test:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const BASE: &str = include_str!("../tests/fixtures/base.yaml");
    const CHILD: &str = include_str!("../tests/fixtures/child.yaml");

    fn resolver() -> MapResolver {
        MapResolver(BTreeMap::from([("base-game".to_string(), BASE.to_string())]))
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
        assert_eq!(rules.constants["ball_points"], serde_yaml_ng::Value::from(3));
        assert_eq!(rules.values.side["park_bonus"].expr(), "7 * count(robots.parked)");
        assert_eq!(rules.values.side["ball_points_total"].expr(), "balls * ball_points");
        assert_eq!(rules.score, "ball_points_total + park_bonus + fouls_received");
    }

    #[test]
    fn missing_parent_is_rule_extends() {
        let errors = parse_rules(CHILD, &NoParents).unwrap_err();
        assert_eq!(errors[0].code, "RULE_EXTENDS");
        assert!(errors[0].message.contains("base-game"));
    }

    #[test]
    fn extends_cycle_is_detected() {
        let a = CHILD.replace("extends: base-game", "extends: b").replace("game: child-game", "game: a");
        let b = CHILD.replace("extends: base-game", "extends: a").replace("game: child-game", "game: b");
        let resolver = MapResolver(BTreeMap::from([("a".into(), a.clone()), ("b".into(), b)]));
        let errors = parse_rules(&a, &resolver).unwrap_err();
        assert_eq!(errors[0].code, "RULE_EXTENDS");
        assert!(errors[0].message.contains("vòng"), "{}", errors[0].message);
    }

    #[test]
    fn yaml_syntax_error_has_line() {
        let errors = parse_rules("schema: raso.rules/v1\ngame: [unclosed\n", &NoParents).unwrap_err();
        assert_eq!(errors[0].code, "RULE_YAML");
        assert!(errors[0].line.is_some());
    }

    #[test]
    fn wrong_schema_and_side_count_are_rejected() {
        let wrong = BASE.replace("raso.rules/v1", "raso.rules/v9");
        assert_eq!(parse_rules(&wrong, &NoParents).unwrap_err()[0].code, "RULE_SCHEMA");
        let three = BASE.replace("ids: [red, blue]", "ids: [red, blue, green]");
        assert_eq!(parse_rules(&three, &NoParents).unwrap_err()[0].code, "RULE_SCHEMA");
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
    }

    #[test]
    fn duplicate_and_reserved_names_are_rejected() {
        let dup = BASE.replace("ball_points_total:", "balls:");
        assert_eq!(parse_rules(&dup, &NoParents).unwrap_err()[0].code, "RULE_NAME");
        let reserved = BASE.replace("ball_points_total:", "opponent:");
        assert_eq!(parse_rules(&reserved, &NoParents).unwrap_err()[0].code, "RULE_NAME");
    }

    #[test]
    fn line_of_finds_nested_keys() {
        assert_eq!(crate::error::line_of(BASE, "values.side.park_bonus"), Some(18));
        assert_eq!(crate::error::line_of(BASE, "nope.nothing"), None);
    }
}
```

Lưu ý: số dòng `18` ứng với fixture ở Step 1 (dòng `    park_bonus:`); nếu sửa fixture thì sửa số này.

- [ ] **Step 3: Chạy để thấy thất bại**

Run: `cd crates/raso-rules && cargo test --lib extends`
Expected: lỗi biên dịch (thiếu `parse_rules`, `MapResolver`…).

- [ ] **Step 4: Viết `error.rs`**

```rust
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuleError {
    pub code: &'static str,
    pub path: String,
    pub line: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScoreError {
    pub code: &'static str,
    pub path: String,
    pub message: String,
}

impl RuleError {
    pub fn new(code: &'static str, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self { code, path: path.into(), line: None, message: message.into() }
    }

    pub fn located(mut self, src: &str) -> Self {
        if self.line.is_none() {
            self.line = line_of(src, &self.path);
        }
        self
    }
}

impl ScoreError {
    pub fn new(code: &'static str, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self { code, path: path.into(), message: message.into() }
    }
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(f, "[{}] {} (dòng {}): {}", self.code, self.path, line, self.message),
            None => write!(f, "[{}] {}: {}", self.code, self.path, self.message),
        }
    }
}

impl std::fmt::Display for ScoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.code, self.path, self.message)
    }
}

/// Tìm dòng (1-based) của khoá theo đường dẫn chấm trong YAML dạng khối.
/// Chỉ dùng để chỉ chỗ cho người viết luật; không tìm thấy thì trả `None`.
pub fn line_of(src: &str, path: &str) -> Option<usize> {
    let lines: Vec<&str> = src.lines().collect();
    let mut from = 0;
    let mut min_indent = 0;
    let mut found = None;
    for segment in path.split('.') {
        let mut hit = None;
        for (index, line) in lines.iter().enumerate().skip(from) {
            let indent = line.len() - line.trim_start().len();
            let trimmed = line.trim_start();
            if found.is_some() && !trimmed.is_empty() && !trimmed.starts_with('#') && indent < min_indent {
                break;
            }
            if indent >= min_indent
                && (trimmed.starts_with(&format!("{segment}:")) || trimmed.starts_with(&format!("\"{segment}\":")))
            {
                hit = Some((index, indent));
                break;
            }
        }
        let (index, indent) = hit?;
        found = Some(index + 1);
        from = index + 1;
        min_indent = indent + 1;
    }
    found
}
```

- [ ] **Step 5: Viết `schema.rs`**

```rust
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
```

- [ ] **Step 6: Viết phần code của `extends.rs`** (phía trên khối test)

```rust
use crate::error::RuleError;
use crate::schema::{InputKind, RulesFile};
use serde_yaml_ng::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EXTENDS_DEPTH: usize = 4;
pub const SCHEMA: &str = "raso.rules/v1";
/// Tên dành riêng cho phạm vi công thức (spec mục 4.3).
pub const RESERVED: [&str; 8] = [
    "robots", "all_robots", "field", "opponent", "matches", "fouls_received", "fouls_committed", "team_score",
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
            RuleError::new("RULE_EXTENDS", format!("override.{path}"), format!("'{segment}' không nằm trong một khối"))
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

fn resolve(yaml: &str, resolver: &dyn PackResolver, chain: &mut Vec<String>) -> Result<Value, Vec<RuleError>> {
    let mut value = parse_value(yaml)?;
    let map = value
        .as_mapping_mut()
        .ok_or_else(|| vec![RuleError::new("RULE_YAML", "", "file luật phải là một khối khoá: giá trị")])?;
    let game = map.get("game").and_then(Value::as_str).unwrap_or("?").to_string();
    if chain.contains(&game) {
        chain.push(game);
        return Err(vec![RuleError::new("RULE_EXTENDS", "extends", format!("kế thừa vòng: {}", chain.join(" → ")))]);
    }
    chain.push(game);
    let Some(parent_id) = map.remove("extends") else {
        map.remove("override");
        return Ok(value);
    };
    let parent_id = parent_id
        .as_str()
        .ok_or_else(|| vec![RuleError::new("RULE_EXTENDS", "extends", "extends phải là id đề")])?
        .to_string();
    if chain.len() > MAX_EXTENDS_DEPTH {
        return Err(vec![RuleError::new("RULE_EXTENDS", "extends", format!("kế thừa quá {MAX_EXTENDS_DEPTH} tầng"))]);
    }
    if chain.contains(&parent_id) {
        chain.push(parent_id);
        return Err(vec![RuleError::new("RULE_EXTENDS", "extends", format!("kế thừa vòng: {}", chain.join(" → ")))]);
    }
    let parent_src = resolver.rules_yaml(&parent_id).ok_or_else(|| {
        vec![RuleError::new("RULE_EXTENDS", "extends", format!("không tìm thấy đề cha '{parent_id}'"))]
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
        errors.push(RuleError::new("RULE_SCHEMA", "schema", format!("chỉ hỗ trợ '{SCHEMA}'")));
    }
    if rules.sides.ids.len() != 2 {
        errors.push(RuleError::new("RULE_SCHEMA", "sides.ids", "v1 cần đúng 2 liên minh"));
    }
    if rules.sides.teams == 0 {
        errors.push(RuleError::new("RULE_SCHEMA", "sides.teams", "mỗi liên minh cần ít nhất 1 đội"));
    }
    for (level, inputs) in [("field", &rules.inputs.field), ("side", &rules.inputs.side), ("robot", &rules.inputs.robot)] {
        for (name, def) in inputs {
            let path = format!("inputs.{level}.{name}");
            if def.kind == InputKind::Choice && def.values.is_empty() {
                errors.push(RuleError::new("RULE_SCHEMA", path.clone(), "ô choice cần danh sách values"));
            }
            if def.max.is_some_and(|m| m < 0) {
                errors.push(RuleError::new("RULE_SCHEMA", path, "max phải ≥ 0"));
            }
        }
    }
    let mut seen = BTreeSet::new();
    let names = rules
        .inputs.field.keys().map(|n| ("inputs.field", n))
        .chain(rules.inputs.side.keys().map(|n| ("inputs.side", n)))
        .chain(rules.inputs.robot.keys().map(|n| ("inputs.robot", n)))
        .chain(rules.values.robot.keys().map(|n| ("values.robot", n)))
        .chain(rules.values.field.keys().map(|n| ("values.field", n)))
        .chain(rules.values.side.keys().map(|n| ("values.side", n)))
        .chain(rules.constants.keys().map(|n| ("constants", n)));
    for (block, name) in names {
        let path = format!("{block}.{name}");
        if RESERVED.contains(&name.as_str()) || rules.sides.ids.contains(name) {
            errors.push(RuleError::new("RULE_NAME", path, format!("'{name}' là tên dành riêng")));
        } else if !seen.insert(name.clone()) {
            errors.push(RuleError::new("RULE_NAME", path, format!("tên '{name}' bị trùng")));
        }
    }
    for (id, _) in &rules.cards {
        if id.is_empty() {
            errors.push(RuleError::new("RULE_SCHEMA", "cards", "tên thẻ rỗng"));
        }
    }
    if let (Some(_), Some(_)) = (&rules.ranking.drop_lowest, rules.ranking.keep_best) {
        errors.push(RuleError::new("RULE_SCHEMA", "ranking", "drop_lowest và keep_best không dùng cùng nhau"));
    }
    errors
}

pub fn parse_rules(yaml: &str, resolver: &dyn PackResolver) -> Result<RulesFile, Vec<RuleError>> {
    let merged = resolve(yaml, resolver, &mut Vec::new())?;
    let rules: RulesFile = serde_yaml_ng::from_value(merged)
        .map_err(|error| vec![RuleError::new("RULE_YAML", "", format!("cấu trúc luật không hợp lệ: {error}"))])?;
    let errors: Vec<RuleError> = structural_checks(&rules).into_iter().map(|e| e.located(yaml)).collect();
    if errors.is_empty() { Ok(rules) } else { Err(errors) }
}
```

Cập nhật `lib.rs`:

```rust
//! RASO — bộ máy tính điểm khai báo bằng YAML.
pub mod error;
pub mod expr;
pub mod extends;
pub mod schema;

pub use error::{RuleError, ScoreError};
pub use extends::{MapResolver, NoParents, PackResolver};
```

- [ ] **Step 7: Chạy test**

Run: `cd crates/raso-rules && cargo test --lib`
Expected: tất cả test của `expr` và `extends` PASS.

- [ ] **Step 8: Phá thử**

(a) Trong `resolve`, bỏ khối `if let Some(Value::Mapping(overrides))` → `child_inherits_and_overrides` đỏ. (b) Bỏ kiểm tra `chain.contains(&parent_id)` → `extends_cycle_is_detected` phải đỏ (lỗi là "quá 4 tầng" chứ không chứa "vòng"). Hoàn tác.

- [ ] **Step 9: Commit**

```bash
git add crates/raso-rules
git commit -m "feat(rules): schema rules.yaml, kế thừa extends/override, lỗi có số dòng"
```

---

### Task 3: Phạm vi tên, sắp topo và biên dịch kế hoạch tính (`load`)

**Files:**
- Create: `crates/raso-rules/src/graph.rs`
- Modify: `crates/raso-rules/src/lib.rs`

**Interfaces:**
- Consumes: `expr::{new_engine, compile, Compiled, CompileError}`, `schema::*`, `extends::parse_rules`, `error::RuleError`.
- Produces:
  - `pub(crate) struct Node { pub name: String, pub label: Option<String>, pub label_en: Option<String>, pub compiled: Compiled }`
  - `pub(crate) struct FoulNode { pub id: String, pub node: Node }`
  - `pub(crate) struct Plan { pub robot: Vec<Node>, pub field: Vec<Node>, pub side: Vec<Node>, pub fouls: Vec<FoulNode>, pub score: Node, pub tiebreakers: Vec<Node> }`
  - `pub(crate) fn build_plan(engine: &Engine, rules: &RulesFile, src: &str) -> Result<Plan, Vec<RuleError>>`
  - `pub struct Rules { pub file: RulesFile, pub(crate) engine: Engine, pub(crate) plan: Plan, pub(crate) constants: Vec<(String, Dynamic)> }`
  - `pub fn load(yaml: &str, resolver: &dyn PackResolver) -> Result<Rules, Vec<RuleError>>`
  - `pub(crate) fn yaml_to_dynamic(v: &serde_yaml_ng::Value) -> Dynamic`

Quy tắc phạm vi (spec mục 4.3), dùng nguyên văn trong code:

| Cấp | Tên đơn được phép | Chuỗi `gốc.thành_phần` được phép |
|---|---|---|
| robot | ô robot, giá trị robot, hằng số | `hằng_số.khoá` |
| field | ô field, giá trị field, hằng số | `all_robots.<ô/giá trị robot>`, `<side_id>.<ô side>`, `hằng_số.khoá` |
| side, foul | ô side, giá trị side, hằng số | `robots.<ô/giá trị robot>`, `field.<ô/giá trị field>`, `<side_id>.<ô side>`, `opponent.<ô side>`, `hằng_số.khoá` |
| score | như side + `fouls_received`, `fouls_committed` | như side |
| tiebreaker | — | `matches.<ô side / giá trị side / team_score>` |

- [ ] **Step 1: Viết test thất bại** — tạo `crates/raso-rules/src/graph.rs` với khối test:

```rust
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
```

- [ ] **Step 2: Chạy để thấy thất bại**

Run: `cd crates/raso-rules && cargo test --lib graph`
Expected: lỗi biên dịch (thiếu `load`).

- [ ] **Step 3: Viết code `graph.rs`** (phía trên khối test)

```rust
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
    for (index, node) in nodes.iter().enumerate() {
        if node.compiled.refs.iter().any(|c| c.len() == 1 && c[0] == node.name) {
            errors.push(RuleError::new("RULE_CYCLE", format!("values.{level_name}.{}", node.name), "giá trị tự tham chiếu"));
            let _ = index;
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
```

Ghi chú cho người thực thi: trong `topo`, khối kiểm tra tự tham chiếu cuối hàm là thừa nếu tự tham chiếu đã bị loại khỏi `deps`; giữ lại để báo lỗi rõ "giá trị tự tham chiếu" thay vì để lọt. Xoá biến `index` thừa nếu clippy cảnh báo.

- [ ] **Step 4: Thêm `Rules` và `load` vào `lib.rs`**

```rust
//! RASO — bộ máy tính điểm khai báo bằng YAML.
pub mod error;
pub mod expr;
pub mod extends;
mod graph;
pub mod schema;

pub use error::{RuleError, ScoreError};
pub use extends::{MapResolver, NoParents, PackResolver};
pub use schema::RulesFile;

use rhai::{Dynamic, Engine, Map};

/// Luật đã kiểm tra và biên dịch, sẵn sàng tính điểm. `Send + Sync`.
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
    let constants = file.constants.iter().map(|(k, v)| (k.clone(), yaml_to_dynamic(v))).collect();
    Ok(Rules { file, engine, plan, constants })
}

#[cfg(test)]
fn _assert_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<Rules>();
}
```

- [ ] **Step 5: Chạy test**

Run: `cd crates/raso-rules && cargo test --lib`
Expected: PASS toàn bộ (expr + extends + graph).

- [ ] **Step 6: Phá thử**

(a) Trong `check`, nhánh `"opponent"` đổi `self.side_inputs()` thành `self.side_names()` → `opponent_can_only_read_inputs` đỏ. (b) Ở `Level::Foul`, đổi `RULE_FOUL_REF` thành `Ok(())` → `foul_points_cannot_reference_fouls_or_score` đỏ. (c) Trong `topo`, thay `position(...)` bằng `Some(0)` → `dependencies_are_sorted_topologically` đỏ. Hoàn tác.

- [ ] **Step 7: Commit**

```bash
git add crates/raso-rules
git commit -m "feat(rules): kiểm tra phạm vi tên, sắp topo, load() biên dịch kế hoạch tính"
```

---

### Task 4: Biên bản, bảng điểm và `score()`

**Files:**
- Create: `crates/raso-rules/src/model.rs`
- Create: `crates/raso-rules/src/eval.rs`
- Modify: `crates/raso-rules/src/lib.rs`

**Interfaces:**
- Consumes: `Rules`, `graph::{Plan, Node}`, `expr::{to_dec, format_dynamic}`, `schema::*`.
- Produces:
  - `pub enum InputValue { Bool(bool), Int(i64), Text(String) }` (serde untagged)
  - `pub struct TeamEntry { pub id: String, #[serde(default)] pub card: Option<String> }`
  - `pub struct SideObservation { #[serde(flatten)] pub inputs: BTreeMap<String, InputValue>, #[serde(default)] pub robots: Vec<BTreeMap<String, InputValue>>, #[serde(default)] pub fouls: BTreeMap<String, u32>, #[serde(default)] pub teams: Vec<TeamEntry> }`
  - `pub struct Observation { pub game: String, #[serde(default)] pub rules_version: Option<String>, #[serde(default)] pub field: BTreeMap<String, InputValue>, pub sides: BTreeMap<String, SideObservation>, #[serde(default)] pub source: Option<String>, #[serde(default)] pub not_simulated: Vec<String> }`
  - `pub struct LineItem { pub name: String, pub label: Option<String>, pub label_en: Option<String>, pub value: String }`
  - `pub struct FoulLine { pub id: String, pub label: String, pub count: u32, pub points_each: String, pub total: String, pub goes_to: String }`
  - `pub struct TeamResult { pub id: String, pub card: Option<String>, pub team_score: i64 }`
  - `pub struct SideSheet { pub values: Vec<LineItem>, pub robots: Vec<Vec<LineItem>>, pub fouls: Vec<FoulLine>, pub fouls_received: String, pub fouls_committed: String, pub raw_score: String, pub score: i64, pub teams: Vec<TeamResult> }`
  - `pub struct ScoreSheet { pub game: String, pub rules_version: String, pub field: Vec<LineItem>, pub sides: BTreeMap<String, SideSheet>, pub not_simulated: Vec<String>, #[serde(default)] pub scoring_error: Option<String> }`
  - Tất cả derive `Debug, Clone, PartialEq, Serialize, Deserialize`.
  - `pub fn score(rules: &Rules, obs: &Observation) -> Result<ScoreSheet, Vec<ScoreError>>`
  - `impl Rules { pub fn blank_observation(&self) -> Observation }` — mọi `count` = 0, `choice` = giá trị đầu, `bool` = false, đủ `sides.teams` robot, `teams` rỗng, `fouls` rỗng.

Thứ tự tính (spec mục 4.3): robot (từng robot) → field → side (từng liên minh) → phạt (`goes_to`) → `score` → làm tròn → thẻ.

- [ ] **Step 1: Viết test thất bại** — tạo `crates/raso-rules/src/eval.rs` với khối test:

```rust
#[cfg(test)]
mod tests {
    use crate::model::*;
    use crate::{NoParents, load, score};
    use std::collections::BTreeMap;

    const BASE: &str = include_str!("../tests/fixtures/base.yaml");

    fn obs(red_balls: i64, blue_balls: i64, red_parked: [bool; 2], blue_parked: [bool; 2]) -> Observation {
        let side = |balls: i64, parked: [bool; 2]| SideObservation {
            inputs: BTreeMap::from([("balls".into(), InputValue::Int(balls))]),
            robots: parked.iter().map(|p| BTreeMap::from([("parked".into(), InputValue::Bool(*p))])).collect(),
            fouls: BTreeMap::new(),
            teams: Vec::new(),
        };
        Observation {
            game: "base-game".into(),
            rules_version: None,
            field: BTreeMap::new(),
            sides: BTreeMap::from([("red".into(), side(red_balls, red_parked)), ("blue".into(), side(blue_balls, blue_parked))]),
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
        let red_values: BTreeMap<_, _> = sheet.sides["red"].values.iter().map(|v| (v.name.as_str(), v.value.as_str())).collect();
        assert_eq!(red_values["ball_points_total"], "20");
        assert_eq!(red_values["park_bonus"], "5");
    }

    #[test]
    fn fouls_go_to_opponent() {
        let rules = load(BASE, &NoParents).unwrap();
        let mut o = obs(10, 3, [false, false], [false, false]);
        o.sides.get_mut("red").unwrap().fouls.insert("minor".into(), 2);
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
            TeamEntry { id: "VIE".into(), card: Some("red".into()) },
            TeamEntry { id: "LAO".into(), card: Some("yellow".into()) },
        ];
        let sheet = score(&rules, &o).unwrap();
        assert_eq!(sheet.sides["red"].score, 20);
        assert_eq!(sheet.sides["red"].teams[0].team_score, 0);
        assert_eq!(sheet.sides["red"].teams[1].team_score, 20);
    }

    #[test]
    fn observation_errors_name_the_input() {
        let rules = load(BASE, &NoParents).unwrap();
        let codes = |o: &Observation| score(&rules, o).unwrap_err().iter().map(|e| (e.code, e.path.clone())).collect::<Vec<_>>();

        let negative = obs(-1, 0, [false, false], [false, false]);
        assert_eq!(codes(&negative), vec![("OBS_RANGE", "sides.red.balls".to_string())]);

        let too_many = obs(51, 0, [false, false], [false, false]);
        assert_eq!(codes(&too_many), vec![("OBS_RANGE", "sides.red.balls".to_string())]);

        let mut missing_robot = obs(1, 1, [false, false], [false, false]);
        missing_robot.sides.get_mut("blue").unwrap().robots.pop();
        assert_eq!(codes(&missing_robot), vec![("OBS_ROBOTS", "sides.blue.robots".to_string())]);

        let mut missing_input = obs(1, 1, [false, false], [false, false]);
        missing_input.sides.get_mut("red").unwrap().inputs.clear();
        assert_eq!(codes(&missing_input), vec![("OBS_MISSING", "sides.red.balls".to_string())]);

        let mut unknown = obs(1, 1, [false, false], [false, false]);
        unknown.sides.get_mut("red").unwrap().inputs.insert("bals".into(), InputValue::Int(1));
        assert_eq!(codes(&unknown), vec![("OBS_UNKNOWN", "sides.red.bals".to_string())]);

        let mut wrong_type = obs(1, 1, [false, false], [false, false]);
        wrong_type.sides.get_mut("red").unwrap().inputs.insert("balls".into(), InputValue::Text("2.5".into()));
        assert_eq!(codes(&wrong_type), vec![("OBS_RANGE", "sides.red.balls".to_string())]);

        let mut bad_card = obs(1, 1, [false, false], [false, false]);
        bad_card.sides.get_mut("red").unwrap().teams = vec![TeamEntry { id: "X".into(), card: Some("purple".into()) }];
        assert_eq!(codes(&bad_card), vec![("OBS_CHOICE", "sides.red.teams.0.card".to_string())]);

        let mut bad_foul = obs(1, 1, [false, false], [false, false]);
        bad_foul.sides.get_mut("red").unwrap().fouls.insert("huge".into(), 1);
        assert_eq!(codes(&bad_foul), vec![("OBS_UNKNOWN", "sides.red.fouls.huge".to_string())]);

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
        assert!(errors[0].path.contains("ball_points_total"), "{}", errors[0].path);
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
}
```

- [ ] **Step 2: Chạy để thấy thất bại**

Run: `cd crates/raso-rules && cargo test --lib eval`
Expected: lỗi biên dịch (thiếu `model`, `score`).

- [ ] **Step 3: Viết `model.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputValue {
    Bool(bool),
    Int(i64),
    /// Mọi thứ khác (chuỗi lựa chọn; số lẻ từ JSON được đọc thành `Text` để bị từ chối rõ ràng).
    Text(String),
}

impl<'de> Deserialize<'de> for InputValueLenient {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        Ok(InputValueLenient(match v {
            serde_json::Value::Bool(b) => InputValue::Bool(b),
            serde_json::Value::Number(n) => match n.as_i64() {
                Some(i) => InputValue::Int(i),
                None => InputValue::Text(n.to_string()),
            },
            serde_json::Value::String(s) => InputValue::Text(s),
            other => InputValue::Text(other.to_string()),
        }))
    }
}

/// Bộ đọc khoan dung: số lẻ (2.5) không làm hỏng việc parse cả biên bản mà
/// trở thành `Text("2.5")` để `score` trả `OBS_RANGE` có tên ô.
#[derive(Debug, Clone, PartialEq)]
pub struct InputValueLenient(pub InputValue);

fn lenient_map<'de, D: serde::Deserializer<'de>>(d: D) -> Result<BTreeMap<String, InputValue>, D::Error> {
    let raw: BTreeMap<String, InputValueLenient> = BTreeMap::deserialize(d)?;
    Ok(raw.into_iter().map(|(k, v)| (k, v.0)).collect())
}

fn lenient_vec<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<BTreeMap<String, InputValue>>, D::Error> {
    let raw: Vec<BTreeMap<String, InputValueLenient>> = Vec::deserialize(d)?;
    Ok(raw.into_iter().map(|m| m.into_iter().map(|(k, v)| (k, v.0)).collect()).collect())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamEntry {
    pub id: String,
    #[serde(default)]
    pub card: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SideObservation {
    #[serde(flatten, deserialize_with = "lenient_map")]
    pub inputs: BTreeMap<String, InputValue>,
    #[serde(default, deserialize_with = "lenient_vec")]
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
    #[serde(default, deserialize_with = "lenient_map")]
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
```

Ghi chú: serde không cho `deserialize_with` cùng `flatten` trên mọi phiên bản. Nếu `cargo build` báo lỗi ở `SideObservation.inputs`, thay bằng `#[serde(flatten)] raw_inputs: BTreeMap<String, serde_json::Value>` + hàm `inputs()` chuyển đổi khoan dung, và đổi mọi chỗ dùng `inputs` tương ứng; test `json_fractional_count_is_rejected_not_rounded` là thước đo đúng. Xoá `#[derive(Deserialize)]` thừa nếu trùng với impl tay của `InputValueLenient`.

- [ ] **Step 4: Viết `eval.rs`** (phía trên khối test)

```rust
use crate::Rules;
use crate::error::ScoreError;
use crate::expr::{format_dynamic, to_dec};
use crate::graph::Node;
use crate::model::*;
use crate::schema::{CardEffect, GoesTo, InputDef, InputKind, Rounding};
use indexmap::IndexMap;
use rhai::{Array, Dynamic, EvalAltResult, Map, Scope};
use rust_decimal::{Decimal, RoundingStrategy};
use rust_decimal::prelude::ToPrimitive;
use std::collections::BTreeMap;

fn input_to_dynamic(def: &InputDef, value: Option<&InputValue>, path: &str, errors: &mut Vec<ScoreError>) -> Dynamic {
    let Some(value) = value else {
        errors.push(ScoreError::new("OBS_MISSING", path, "thiếu ô này trong biên bản"));
        return Dynamic::UNIT;
    };
    match (def.kind, value) {
        (InputKind::Count, InputValue::Int(n)) => {
            if *n < 0 || def.max.is_some_and(|m| *n > m) {
                errors.push(ScoreError::new(
                    "OBS_RANGE",
                    path,
                    format!("giá trị {n} ngoài khoảng 0..{}", def.max.map(|m| m.to_string()).unwrap_or("∞".into())),
                ));
            }
            Dynamic::from_decimal(Decimal::from(*n))
        }
        (InputKind::Count, other) => {
            errors.push(ScoreError::new("OBS_RANGE", path, format!("cần số nguyên ≥ 0, nhận {other:?}")));
            Dynamic::UNIT
        }
        (InputKind::Bool, InputValue::Bool(b)) => Dynamic::from_bool(*b),
        (InputKind::Bool, other) => {
            errors.push(ScoreError::new("OBS_CHOICE", path, format!("cần true/false, nhận {other:?}")));
            Dynamic::UNIT
        }
        (InputKind::Choice, InputValue::Text(s)) if def.values.contains(s) => Dynamic::from(s.clone()),
        (InputKind::Choice, other) => {
            errors.push(ScoreError::new("OBS_CHOICE", path, format!("cần một trong {:?}, nhận {other:?}", def.values)));
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
            errors.push(ScoreError::new("OBS_UNKNOWN", format!("{prefix}.{name}"), "ô không có trong luật"));
        }
    }
    defs.iter()
        .map(|(name, def)| (name.clone(), input_to_dynamic(def, given.get(name), &format!("{prefix}.{name}"), errors)))
        .collect()
}

fn eval_node(rules: &Rules, scope: &mut Scope, node: &Node, path: &str) -> Result<Dynamic, ScoreError> {
    rules.engine.eval_ast_with_scope::<Dynamic>(scope, &node.compiled.ast).map_err(|error| {
        let code = if matches!(*error, EvalAltResult::ErrorTooManyOperations(_)) { "EVAL_LIMIT" } else { "EVAL" };
        ScoreError::new(code, path, format!("lỗi khi tính '{}': {error}", node.compiled.source))
    })
}

fn push_constants(rules: &Rules, scope: &mut Scope) {
    for (name, value) in &rules.constants {
        scope.push_constant_dynamic(name.clone(), value.clone());
    }
}

fn map_of(pairs: &[(String, Dynamic)]) -> Map {
    pairs.iter().map(|(k, v)| (k.as_str().into(), v.clone())).collect()
}

fn line(node: &Node, value: &Dynamic) -> LineItem {
    LineItem { name: node.name.clone(), label: node.label.clone(), label_en: node.label_en.clone(), value: format_dynamic(value) }
}

fn input_line(name: &str, def: &InputDef, value: &Dynamic) -> LineItem {
    LineItem { name: name.to_string(), label: Some(def.label.clone()), label_en: def.label_en.clone(), value: format_dynamic(value) }
}

fn round(raw: Decimal, rounding: Rounding) -> i64 {
    let r = match rounding {
        Rounding::Ceil => raw.ceil(),
        Rounding::Floor => raw.floor(),
        Rounding::Round => raw.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero),
        Rounding::None => raw.trunc(),
    };
    r.to_i64().unwrap_or(i64::MAX)
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
        return Err(vec![ScoreError::new("OBS_GAME", "game", format!("biên bản của đề '{}', luật là '{}'", obs.game, file.game))]);
    }
    for id in obs.sides.keys() {
        if !file.sides.ids.contains(id) {
            errors.push(ScoreError::new("OBS_UNKNOWN", format!("sides.{id}"), "liên minh không có trong luật"));
        }
    }
    let empty = SideObservation { inputs: BTreeMap::new(), robots: Vec::new(), fouls: BTreeMap::new(), teams: Vec::new() };
    let field_inputs = read_inputs(&file.inputs.field, &obs.field, "field", &mut errors);
    let mut sides: Vec<(String, SideState)> = Vec::new();
    for id in &file.sides.ids {
        let side_obs = obs.sides.get(id).unwrap_or(&empty);
        if !obs.sides.contains_key(id) {
            errors.push(ScoreError::new("OBS_MISSING", format!("sides.{id}"), "thiếu liên minh"));
        }
        let inputs = read_inputs(&file.inputs.side, &side_obs.inputs, &format!("sides.{id}"), &mut errors);
        if side_obs.robots.len() != file.sides.teams {
            errors.push(ScoreError::new(
                "OBS_ROBOTS",
                format!("sides.{id}.robots"),
                format!("cần {} robot, biên bản có {}", file.sides.teams, side_obs.robots.len()),
            ));
        }
        let robots = side_obs
            .robots
            .iter()
            .enumerate()
            .map(|(i, r)| read_inputs(&file.inputs.robot, r, &format!("sides.{id}.robots.{i}"), &mut errors))
            .collect();
        for foul in side_obs.fouls.keys() {
            if !file.fouls.contains_key(foul) {
                errors.push(ScoreError::new("OBS_UNKNOWN", format!("sides.{id}.fouls.{foul}"), "loại lỗi không có trong luật"));
            }
        }
        for (i, team) in side_obs.teams.iter().enumerate() {
            if let Some(card) = &team.card {
                if !file.cards.contains_key(card) {
                    errors.push(ScoreError::new("OBS_CHOICE", format!("sides.{id}.teams.{i}.card"), format!("thẻ '{card}' không có trong luật")));
                }
            }
        }
        sides.push((id.clone(), SideState { inputs, robots, values: Vec::new() }));
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
                let value = eval_node(rules, &mut scope, node, &format!("values.robot.{} ({id} #{})", node.name, index + 1)).map_err(|e| vec![e])?;
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
        columns.into_iter().map(|(k, v)| (k.into(), Dynamic::from_array(v))).collect()
    };
    let all_robots: Vec<Vec<(String, Dynamic)>> = sides.iter().flat_map(|(_, s)| s.robots.clone()).collect();
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
        let value = eval_node(rules, &mut field_scope, node, &format!("values.field.{}", node.name)).map_err(|e| vec![e])?;
        field_scope.push_dynamic(node.name.clone(), value.clone());
        field_values.push((node.name.clone(), value));
    }
    let field_map: Map = map_of(&field_inputs).into_iter().chain(map_of(&field_values)).collect();

    // 3. Cấp side.
    let side_inputs: Vec<(String, Map)> = sides.iter().map(|(id, s)| (id.clone(), map_of(&s.inputs))).collect();
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
            let value = eval_node(rules, &mut scope, node, &format!("values.side.{} ({id})", node.name)).map_err(|e| vec![e])?;
            scope.push_dynamic(node.name.clone(), value.clone());
            state.values.push((node.name.clone(), value));
        }
    }

    // 4. Phạt.
    let mut received: BTreeMap<String, Decimal> = file.sides.ids.iter().map(|id| (id.clone(), Decimal::ZERO)).collect();
    let mut committed = received.clone();
    let mut foul_lines: BTreeMap<String, Vec<FoulLine>> = BTreeMap::new();
    for (id, state) in &sides {
        let side_obs = obs.sides.get(id).unwrap_or(&empty);
        let opponent = file.sides.ids.iter().find(|other| *other != id).expect("2 liên minh").clone();
        let mut scope = make_side_scope(id, state);
        for foul in &plan.fouls {
            let def = &file.fouls[&foul.id];
            let count = side_obs.fouls.get(&foul.id).copied().unwrap_or(0);
            let each = to_dec(&eval_node(rules, &mut scope, &foul.node, &format!("fouls.{} ({id})", foul.id)).map_err(|e| vec![e])?)
                .map_err(|e| vec![ScoreError::new("EVAL", format!("fouls.{}", foul.id), e.to_string())])?;
            let total = each * Decimal::from(count);
            match def.goes_to {
                GoesTo::Opponent => *received.get_mut(&opponent).unwrap() += total,
                GoesTo::SelfSide => *committed.get_mut(id).unwrap() += total,
            }
            foul_lines.entry(id.clone()).or_default().push(FoulLine {
                id: foul.id.clone(),
                label: def.label.clone(),
                count,
                points_each: each.normalize().to_string(),
                total: total.normalize().to_string(),
                goes_to: match def.goes_to { GoesTo::Opponent => "opponent".into(), GoesTo::SelfSide => "self".into() },
            });
        }
    }

    // 5. Điểm cuối + thẻ.
    let mut sheet_sides = BTreeMap::new();
    for (id, state) in &sides {
        let mut scope = make_side_scope(id, state);
        scope.push_constant("fouls_received", received[id]);
        scope.push_constant("fouls_committed", committed[id]);
        let raw = to_dec(&eval_node(rules, &mut scope, &plan.score, &format!("score ({id})")).map_err(|e| vec![e])?)
            .map_err(|e| vec![ScoreError::new("EVAL", "score", e.to_string())])?;
        let final_score = round(raw, file.rounding);
        let side_obs = obs.sides.get(id).unwrap_or(&empty);
        let teams = side_obs
            .teams
            .iter()
            .map(|team| {
                let zero = team.card.as_ref().and_then(|c| file.cards.get(c)).is_some_and(|e| *e == CardEffect::TeamZero);
                TeamResult { id: team.id.clone(), card: team.card.clone(), team_score: if zero { 0 } else { final_score } }
            })
            .collect();
        let mut values: Vec<LineItem> = file.inputs.side.iter().zip(state.inputs.iter()).map(|((n, d), (_, v))| input_line(n, d, v)).collect();
        values.extend(plan.side.iter().zip(state.values.iter()).map(|(n, (_, v))| line(n, v)));
        let robots = state
            .robots
            .iter()
            .map(|robot| {
                let mut items: Vec<LineItem> = file.inputs.robot.iter().zip(robot.iter()).map(|((n, d), (_, v))| input_line(n, d, v)).collect();
                let offset = file.inputs.robot.len();
                items.extend(plan.robot.iter().zip(robot.iter().skip(offset)).map(|(n, (_, v))| line(n, v)));
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
    let mut field: Vec<LineItem> = file.inputs.field.iter().zip(field_inputs.iter()).map(|((n, d), (_, v))| input_line(n, d, v)).collect();
    field.extend(plan.field.iter().zip(field_values.iter()).map(|(n, (_, v))| line(n, v)));
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
                            robots: (0..self.file.sides.teams).map(|_| blank(&self.file.inputs.robot)).collect(),
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
```

Lưu ý: `robot.push(...)` ở bước 1 nối giá trị tính sau các ô biên bản; việc dựng `LineItem` ở bước 5 dựa vào thứ tự đó (ô trước, giá trị sau). Nếu đổi thứ tự thì sửa cả hai chỗ.

Cập nhật `lib.rs`: thêm `mod eval; pub mod model;` và

```rust
pub use eval::score;
pub use model::{FoulLine, InputValue, LineItem, Observation, ScoreSheet, SideObservation, SideSheet, TeamEntry, TeamResult};
```

- [ ] **Step 5: Chạy test**

Run: `cd crates/raso-rules && cargo test --lib`
Expected: PASS toàn bộ.

- [ ] **Step 6: Phá thử**

(a) Đổi `GoesTo::Opponent => *received.get_mut(&opponent)` thành `received.get_mut(id)` → `fouls_go_to_opponent` đỏ. (b) Trong `input_to_dynamic`, bỏ điều kiện `def.max.is_some_and(...)` → `observation_errors_name_the_input` đỏ ở ca `too_many`. (c) Đổi `Rounding::Ceil => raw.ceil()` thành `raw.floor()` → `rounding_applies_to_final_score` đỏ. Hoàn tác.

- [ ] **Step 7: Commit**

```bash
git add crates/raso-rules
git commit -m "feat(rules): biên bản, bảng điểm chi tiết và score() với phạt, thẻ, làm tròn"
```

---
### Task 5: Xếp hạng `rank()`

**Files:**
- Create: `crates/raso-rules/src/rank.rs`
- Modify: `crates/raso-rules/src/lib.rs`

**Interfaces:**
- Consumes: `Rules`, `model::{ScoreSheet, TeamResult}`, `graph::Plan.tiebreakers`, `expr::to_dec`, `schema::{Aggregate, FinalTiebreak}`.
- Produces:
  - `pub struct MatchResult { pub match_id: String, pub sheet: ScoreSheet }`
  - `pub struct StandingRow { pub rank: usize, pub team: String, pub ranking_score: String, pub tiebreakers: Vec<String>, pub played: usize, pub counted_matches: Vec<String>, pub tied: bool }`
  - `pub struct Standings { pub rows: Vec<StandingRow> }`
  - `pub fn rank(rules: &Rules, results: &[MatchResult]) -> Result<Standings, Vec<ScoreError>>`
  - Tất cả derive `Debug, Clone, PartialEq, Serialize, Deserialize`.

Quy tắc (spec mục 4.5):
- Giá trị trận của đội = `team_score` trong `SideSheet.teams`.
- `keep_best: k` → giữ k trận cao nhất.
- `drop_lowest { count, not_if }` → chỉ bỏ khi số trận > `count`; duyệt từ thấp lên, bỏ qua trận có thẻ thuộc `not_if`.
- `aggregate: average` → tổng/số trận được tính (0 trận → 0); `sum` → tổng.
- Tiebreaker tính trên `matches.<tên>` chỉ gồm các trận được tính; so sánh giảm dần.
- `final_tiebreak: draw` → hàng bằng nhau mọi tiêu chí dùng chung số hạng và `tied = true`; `team_number` → so `team` tăng dần (số nếu parse được, không thì chuỗi).

- [ ] **Step 1: Viết test thất bại** — tạo `crates/raso-rules/src/rank.rs` với khối test:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::{NoParents, load, score};
    use std::collections::BTreeMap;

    const BASE: &str = include_str!("../tests/fixtures/base.yaml");

    /// Một trận: đội `red_team` bên đỏ ghi `red_balls`, đội `blue_team` bên xanh ghi `blue_balls`.
    fn played(rules: &Rules, id: &str, red_team: (&str, Option<&str>), red_balls: i64, blue_team: (&str, Option<&str>), blue_balls: i64) -> MatchResult {
        let mut o = rules.blank_observation();
        let set = |o: &mut Observation, side: &str, team: (&str, Option<&str>), balls: i64| {
            let s = o.sides.get_mut(side).unwrap();
            s.inputs.insert("balls".into(), InputValue::Int(balls));
            s.teams = vec![TeamEntry { id: team.0.into(), card: team.1.map(str::to_string) }];
        };
        set(&mut o, "red", red_team, red_balls);
        set(&mut o, "blue", blue_team, blue_balls);
        MatchResult { match_id: id.into(), sheet: score(rules, &o).unwrap() }
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
        let yaml = BASE.replace("  aggregate: sum\n  keep_best: 2", "  aggregate: average\n  drop_lowest: { count: 1, not_if: [red] }");
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
        let yaml = BASE.replace("  aggregate: sum\n  keep_best: 2", "  aggregate: average\n  drop_lowest: { count: 1 }");
        let rules = load(&yaml, &NoParents).unwrap();
        let standings = rank(&rules, &[played(&rules, "m1", ("A", None), 4, ("B", None), 2)]).unwrap();
        assert_eq!(standings.rows.iter().find(|r| r.team == "A").unwrap().ranking_score, "8");
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
}
```

- [ ] **Step 2: Chạy để thấy thất bại**

Run: `cd crates/raso-rules && cargo test --lib rank`
Expected: lỗi biên dịch.

- [ ] **Step 3: Viết code `rank.rs`** (phía trên khối test)

```rust
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
            let side_values: BTreeMap<String, Decimal> = side.values.iter().map(|v| (v.name.clone(), parse_value(&v.value))).collect();
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
            order.sort_by(|a, b| entries[*b].team_score.cmp(&entries[*a].team_score).then(a.cmp(b)));
            order.into_iter().take(k).collect()
        } else if let Some(drop) = &ranking.drop_lowest {
            order.sort_by(|a, b| entries[*a].team_score.cmp(&entries[*b].team_score).then(a.cmp(b)));
            let mut dropped = Vec::new();
            if entries.len() > drop.count {
                for &i in &order {
                    if dropped.len() == drop.count {
                        break;
                    }
                    let protected = entries[i].card.as_ref().is_some_and(|c| drop.not_if.contains(c));
                    if !protected {
                        dropped.push(i);
                    }
                }
            }
            (0..entries.len()).filter(|i| !dropped.contains(i)).collect()
        } else {
            order
        };
        let mut counted = counted;
        counted.sort();
        let total: Decimal = counted.iter().map(|&i| entries[i].team_score).sum();
        let score = match ranking.aggregate {
            Aggregate::Sum => total,
            Aggregate::Average if counted.is_empty() => Decimal::ZERO,
            Aggregate::Average => total / Decimal::from(counted.len() as i64),
        };

        let mut columns: BTreeMap<String, Array> = BTreeMap::new();
        for &i in &counted {
            columns.entry("team_score".into()).or_default().push(Dynamic::from_decimal(entries[i].team_score));
            for (name, value) in &entries[i].side_values {
                columns.entry(name.clone()).or_default().push(Dynamic::from_decimal(*value));
            }
        }
        let matches: Map = columns.into_iter().map(|(k, v)| (k.into(), Dynamic::from_array(v))).collect();
        let mut tiebreak_values = Vec::new();
        for node in &rules.plan.tiebreakers {
            let mut scope = Scope::new();
            scope.push_constant("matches", matches.clone());
            let value = rules
                .engine
                .eval_ast_with_scope::<Dynamic>(&mut scope, &node.compiled.ast)
                .map_err(|e| vec![ScoreError::new("EVAL", format!("ranking.tiebreakers ({})", node.name), e.to_string())])?;
            tiebreak_values.push(to_dec(&value).map_err(|e| vec![ScoreError::new("EVAL", "ranking.tiebreakers", e.to_string())])?);
        }
        rows.push((
            StandingRow {
                rank: 0,
                team,
                ranking_score: score.normalize().to_string(),
                tiebreakers: tiebreak_values.iter().map(|d| d.normalize().to_string()).collect(),
                played: entries.len(),
                counted_matches: counted.iter().map(|&i| entries[i].match_id.clone()).collect(),
                tied: false,
            },
            score,
            tiebreak_values,
        ));
    }

    let primary = |a: &(StandingRow, Decimal, Vec<Decimal>), b: &(StandingRow, Decimal, Vec<Decimal>)| {
        b.1.cmp(&a.1).then_with(|| {
            a.2.iter().zip(b.2.iter()).map(|(x, y)| y.cmp(x)).find(|o| *o != Ordering::Equal).unwrap_or(Ordering::Equal)
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
        rows[i].0.rank = if draw && equal_prev { rows[i - 1].0.rank } else { i + 1 };
        rows[i].0.tied = draw && (equal_prev || equal_next);
    }
    Ok(Standings { rows: rows.into_iter().map(|r| r.0).collect() })
}
```

Cập nhật `lib.rs`: `mod rank;` và `pub use rank::{MatchResult, StandingRow, Standings, rank};`.

- [ ] **Step 4: Chạy test**

Run: `cd crates/raso-rules && cargo test --lib`
Expected: PASS toàn bộ.

- [ ] **Step 5: Phá thử**

(a) Bỏ điều kiện `if entries.len() > drop.count` → `single_match_is_not_dropped` đỏ. (b) Bỏ `protected` (luôn `false`) → `drop_lowest_skips_red_card_match_and_averages` đỏ. Hoàn tác.

- [ ] **Step 6: Commit**

```bash
git add crates/raso-rules
git commit -m "feat(rules): rank() với keep_best, drop_lowest + not_if, tiebreaker"
```

---

### Task 6: Đề FGC 2026, trình chạy test của đề và CLI `raso`

**Files:**
- Create: `crates/raso-rules/src/pack.rs`
- Create: `crates/raso-rules/src/bin/raso.rs`
- Create: `crates/raso-rules/tests/packs.rs`
- Create: `pkgs/games/index.json`
- Create: `pkgs/games/fgc-2026/rules.yaml`
- Create: `pkgs/games/fgc-2026/tests/scoring/01-basic.yaml` … `08-observation-error.yaml`
- Modify: `crates/raso-rules/src/lib.rs`

**Interfaces:**
- Consumes: `load`, `score`, `rank`, `MapResolver`/`PackResolver`, `model::*`.
- Produces:
  - `pub struct DirResolver { pub games_root: PathBuf }` impl `PackResolver` (đọc `<games_root>/<id>/rules.yaml`)
  - `pub struct CaseOutcome { pub name: String, pub file: String, pub failures: Vec<String> }`
  - `pub struct PackReport { pub game: String, pub cases: Vec<CaseOutcome> }` với `fn ok(&self) -> bool`
  - `pub fn check_pack(dir: &Path) -> Result<PackReport, Vec<RuleError>>`
  - Định dạng file test (`tests/scoring/*.yaml`):

```yaml
name: "mô tả ca"
observation: { ... Observation ... }
expect:                       # hoặc expect_error: OBS_RANGE
  field: { coop: 10 }         # giá trị field (so sánh Decimal), tuỳ chọn
  sides:
    red:
      score: 155
      values: { climb_mult: 1.55, su_points: 155 }   # tuỳ chọn
      fouls_received: 0                              # tuỳ chọn
      teams: { VIE: 155, LAO: 0 }                    # tuỳ chọn
```

- [ ] **Step 1: Viết `pkgs/games/fgc-2026/rules.yaml`** — chép nguyên văn khối YAML ở spec mục 6.1 (từ `schema: raso.rules/v1` đến hết `open_questions`).

- [ ] **Step 2: Viết 8 file test** trong `pkgs/games/fgc-2026/tests/scoring/` (đã tính tay bằng `Decimal`, xem spec phụ lục A):

`01-basic.yaml`:

```yaml
name: "Cơ bản: hệ số leo từng liên minh"
observation:
  game: fgc-2026
  field: { ext_wildfire: 0 }
  sides:
    red:
      su_wildfire: 100
      robots:
        - { climb: ZONE_3, lifted_by_partner: false }
        - { climb: ZONE_2, lifted_by_partner: false }
        - { climb: CONTACT, lifted_by_partner: false }
    blue:
      su_wildfire: 80
      robots:
        - { climb: ZONE_1, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
expect:
  field: { knockdown: 0, coop: 0 }
  sides:
    red: { score: 155, values: { climb_mult: 1.55, su_points: 155 } }
    blue: { score: 88, values: { climb_mult: 1.10, su_points: 88 } }
```

`02-knockdown-coop4.yaml`:

```yaml
name: "Knockdown + Coopertition 4 robot Zone 3"
observation:
  game: fgc-2026
  field: { ext_wildfire: 150 }
  sides:
    red:
      su_wildfire: 120
      robots:
        - { climb: ZONE_3, lifted_by_partner: false }
        - { climb: ZONE_3, lifted_by_partner: false }
        - { climb: ZONE_3, lifted_by_partner: false }
    blue:
      su_wildfire: 100
      robots:
        - { climb: ZONE_3, lifted_by_partner: false }
        - { climb: ZONE_2, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
expect:
  field: { ext_points: 150, knockdown: 10, coop: 10 }
  sides:
    red: { score: 398, values: { su_points: 228 } }
    blue: { score: 320, values: { su_points: 150 } }
```

`03-knockdown-tie.yaml`:

```yaml
name: "Knockdown không tính khi Extinguisher bằng một Suppression Unit"
observation:
  game: fgc-2026
  field: { ext_wildfire: 120 }
  sides:
    red:
      su_wildfire: 120
      robots:
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
    blue:
      su_wildfire: 50
      robots:
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
expect:
  field: { knockdown: 0 }
  sides:
    red: { score: 240 }
    blue: { score: 170 }
```

`04-decimal-trap.yaml`:

```yaml
name: "Số thập phân chính xác: 50 × 1,10 = 55 (máy tính JS chính thức ra 56); 7 × 1,05 = 7,35 → 8"
observation:
  game: fgc-2026
  field: { ext_wildfire: 0 }
  sides:
    red:
      su_wildfire: 50
      robots:
        - { climb: ZONE_1, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
    blue:
      su_wildfire: 7
      robots:
        - { climb: CONTACT, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
expect:
  sides:
    red: { score: 55, values: { su_points: 55 } }
    blue: { score: 8, values: { su_points: 8 } }
```

`05-partner-coop6.yaml`:

```yaml
name: "Partner climb 25 điểm + Coopertition 6 robot = 40"
observation:
  game: fgc-2026
  field: { ext_wildfire: 0 }
  sides:
    red:
      su_wildfire: 10
      robots:
        - { climb: ZONE_3, lifted_by_partner: false }
        - { climb: ZONE_3, lifted_by_partner: true }
        - { climb: ZONE_3, lifted_by_partner: false }
    blue:
      su_wildfire: 10
      robots:
        - { climb: ZONE_3, lifted_by_partner: false }
        - { climb: ZONE_3, lifted_by_partner: false }
        - { climb: ZONE_3, lifted_by_partner: false }
expect:
  field: { coop: 40, knockdown: 0 }
  sides:
    red: { score: 84, values: { partner: 25, su_points: 19 } }
    blue: { score: 59 }
```

`06-fouls.yaml`:

```yaml
name: "Lỗi nhẹ 5% / nặng 10% điểm trước phạt, cộng cho đối thủ, làm tròn lên"
observation:
  game: fgc-2026
  field: { ext_wildfire: 0 }
  sides:
    red:
      su_wildfire: 100
      fouls: { minor: 1 }
      robots:
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
    blue:
      su_wildfire: 33
      fouls: { major: 1 }
      robots:
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
expect:
  sides:
    red: { score: 104, fouls_received: 3.3 }
    blue: { score: 38, fouls_received: 5 }
```

`07-cards.yaml`:

```yaml
name: "Thẻ đỏ/trắng: đội đó 0 điểm, điểm liên minh không đổi; thẻ vàng không trừ"
observation:
  game: fgc-2026
  field: { ext_wildfire: 0 }
  sides:
    red:
      su_wildfire: 10
      teams: [ { id: VIE, card: yellow }, { id: LAO, card: red }, { id: BRU } ]
      robots:
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
    blue:
      su_wildfire: 4
      teams: [ { id: SGP, card: white }, { id: THA }, { id: PHI } ]
      robots:
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
expect:
  sides:
    red: { score: 10, teams: { VIE: 10, LAO: 0, BRU: 10 } }
    blue: { score: 4, teams: { SGP: 0, THA: 4, PHI: 4 } }
```

`08-observation-error.yaml`:

```yaml
name: "Biên bản sai: số bóng vượt 500"
observation:
  game: fgc-2026
  field: { ext_wildfire: 0 }
  sides:
    red:
      su_wildfire: 501
      robots:
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
    blue:
      su_wildfire: 0
      robots:
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
        - { climb: NONE, lifted_by_partner: false }
expect_error: OBS_RANGE
```

`pkgs/games/index.json`:

```json
{
  "games": [
    { "id": "fgc-2026", "name": "Igniting Innovation", "year": 2026, "series": "FIRST Global Challenge", "simulated": true },
    { "id": "fgc-2025", "name": "Eco Equilibrium", "year": 2025, "series": "FIRST Global Challenge", "simulated": false }
  ]
}
```

(`fgc-2025` được tạo ở Task 7; test ở Step 3 chỉ duyệt thư mục có `rules.yaml` nên không lỗi khi chưa có.)

- [ ] **Step 3: Viết test tích hợp thất bại** `crates/raso-rules/tests/packs.rs`:

```rust
use raso_rules::pack::check_pack;
use std::path::PathBuf;

fn games_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../pkgs/games")
}

#[test]
fn every_pack_with_rules_passes_its_scoring_tests() {
    let mut checked = 0;
    for entry in std::fs::read_dir(games_root()).unwrap() {
        let dir = entry.unwrap().path();
        if !dir.join("rules.yaml").exists() {
            continue;
        }
        let report = check_pack(&dir).unwrap_or_else(|errors| {
            panic!("{}: {}", dir.display(), errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"))
        });
        for case in &report.cases {
            assert!(case.failures.is_empty(), "{} / {}: {}", report.game, case.file, case.failures.join("; "));
        }
        assert!(!report.cases.is_empty());
        checked += 1;
    }
    assert!(checked >= 1, "không tìm thấy đề nào có rules.yaml");
}

#[test]
fn fgc_2026_has_eight_cases() {
    let report = check_pack(&games_root().join("fgc-2026")).unwrap();
    assert_eq!(report.cases.len(), 8);
}
```

Run: `cd crates/raso-rules && cargo test --test packs`
Expected: lỗi biên dịch (thiếu `pack`).

- [ ] **Step 4: Viết `pack.rs`**

```rust
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
    let yaml = std::fs::read_to_string(&rules_path)
        .map_err(|e| vec![RuleError::new("RULE_YAML", rules_path.display().to_string(), e.to_string())])?;
    let resolver = DirResolver { games_root: dir.parent().unwrap_or(dir).to_path_buf() };
    let rules = load(&yaml, &resolver)?;
    let tests_dir = dir.join("tests/scoring");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&tests_dir)
        .map(|it| it.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "yaml")).collect())
        .unwrap_or_default();
    files.sort();
    if files.is_empty() {
        return Err(vec![RuleError::new("RULE_NO_TESTS", tests_dir.display().to_string(), "đề phải có ít nhất một file test trong tests/scoring")]);
    }
    let mut cases = Vec::new();
    for file in files {
        let shown = file.file_name().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let case: TestCase = match serde_yaml_ng::from_str(&text) {
            Ok(case) => case,
            Err(e) => {
                cases.push(CaseOutcome { name: shown.clone(), file: shown, failures: vec![format!("file test sai định dạng: {e}")] });
                continue;
            }
        };
        let mut failures = Vec::new();
        match (score(&rules, &case.observation), &case.expect_error, &case.expect) {
            (Err(errors), Some(code), _) => {
                if !errors.iter().any(|e| e.code == code) {
                    failures.push(format!("cần lỗi {code}, nhận {:?}", errors.iter().map(|e| e.code).collect::<Vec<_>>()));
                }
            }
            (Ok(_), Some(code), _) => failures.push(format!("cần lỗi {code} nhưng tính điểm thành công")),
            (Err(errors), None, _) => failures.extend(errors.iter().map(|e| e.to_string())),
            (Ok(sheet), None, None) => {
                let _ = sheet;
                failures.push("thiếu expect hoặc expect_error".into());
            }
            (Ok(sheet), None, Some(expect)) => {
                for (name, want) in &expect.field {
                    match sheet.field.iter().find(|v| &v.name == name) {
                        Some(got) if same(want, &got.value) => {}
                        Some(got) => failures.push(format!("field.{name}: cần {want:?}, nhận {}", got.value)),
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
                            failures.push(format!("{side}.score: cần {s}, nhận {} (thô {})", got.score, got.raw_score));
                        }
                    }
                    for (name, value) in &want.values {
                        match got.values.iter().find(|v| &v.name == name) {
                            Some(item) if same(value, &item.value) => {}
                            Some(item) => failures.push(format!("{side}.{name}: cần {value:?}, nhận {}", item.value)),
                            None => failures.push(format!("{side}.{name}: không có giá trị này")),
                        }
                    }
                    if let Some(fr) = &want.fouls_received {
                        if !same(fr, &got.fouls_received) {
                            failures.push(format!("{side}.fouls_received: cần {fr:?}, nhận {}", got.fouls_received));
                        }
                    }
                    for (team, s) in &want.teams {
                        match got.teams.iter().find(|t| &t.id == team) {
                            Some(t) if t.team_score == *s => {}
                            Some(t) => failures.push(format!("{side}.teams.{team}: cần {s}, nhận {}", t.team_score)),
                            None => failures.push(format!("{side}.teams.{team}: không có đội")),
                        }
                    }
                }
            }
        }
        cases.push(CaseOutcome { name: case.name, file: shown, failures });
    }
    Ok(PackReport { game: rules.file.game.clone(), cases })
}
```

Cập nhật `lib.rs`:

```rust
#[cfg(not(target_arch = "wasm32"))]
pub mod pack;
```

- [ ] **Step 5: Viết CLI `src/bin/raso.rs`**

```rust
//! raso check <thư-mục-đề>... | raso score <rules.yaml> <biên-bản.yaml> [--json] | raso rank <rules.yaml> <kết-quả.json>...
use raso_rules::pack::{DirResolver, check_pack};
use raso_rules::{MatchResult, Observation, load, rank, score};
use std::path::Path;
use std::process::ExitCode;

fn resolver_for(rules_path: &Path) -> DirResolver {
    let games_root = rules_path.parent().and_then(Path::parent).unwrap_or(Path::new(".")).to_path_buf();
    DirResolver { games_root }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") if args.len() > 1 => {
            let mut ok = true;
            for dir in &args[1..] {
                match check_pack(Path::new(dir)) {
                    Ok(report) => {
                        for case in &report.cases {
                            if case.failures.is_empty() {
                                println!("✓ {} / {}", report.game, case.file);
                            } else {
                                ok = false;
                                println!("✗ {} / {} — {}", report.game, case.file, case.name);
                                for f in &case.failures {
                                    println!("    {f}");
                                }
                            }
                        }
                    }
                    Err(errors) => {
                        ok = false;
                        for e in errors {
                            println!("✗ {dir}: {e}");
                        }
                    }
                }
            }
            if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
        }
        Some("score") if args.len() >= 3 => {
            let rules_path = Path::new(&args[1]);
            let yaml = std::fs::read_to_string(rules_path).expect("đọc rules.yaml");
            let rules = match load(&yaml, &resolver_for(rules_path)) {
                Ok(r) => r,
                Err(errors) => {
                    errors.iter().for_each(|e| eprintln!("{e}"));
                    return ExitCode::FAILURE;
                }
            };
            let obs: Observation = serde_yaml_ng::from_str(&std::fs::read_to_string(&args[2]).expect("đọc biên bản")).expect("biên bản sai định dạng");
            match score(&rules, &obs) {
                Ok(sheet) if args.iter().any(|a| a == "--json") => {
                    println!("{}", serde_json::to_string_pretty(&sheet).unwrap());
                    ExitCode::SUCCESS
                }
                Ok(sheet) => {
                    for (side, s) in &sheet.sides {
                        println!("{side}: {} (thô {})", s.score, s.raw_score);
                        for v in s.values.iter().filter(|v| v.label.is_some()) {
                            println!("  {}: {}", v.label.as_deref().unwrap_or(&v.name), v.value);
                        }
                    }
                    ExitCode::SUCCESS
                }
                Err(errors) => {
                    errors.iter().for_each(|e| eprintln!("{e}"));
                    ExitCode::FAILURE
                }
            }
        }
        Some("rank") if args.len() >= 3 => {
            let rules_path = Path::new(&args[1]);
            let yaml = std::fs::read_to_string(rules_path).expect("đọc rules.yaml");
            let rules = load(&yaml, &resolver_for(rules_path)).expect("luật hợp lệ");
            let results: Vec<MatchResult> = args[2..]
                .iter()
                .map(|p| serde_json::from_str(&std::fs::read_to_string(p).expect("đọc kết quả")).expect("kết quả sai định dạng"))
                .collect();
            match rank(&rules, &results) {
                Ok(standings) => {
                    println!("{}", serde_json::to_string_pretty(&standings).unwrap());
                    ExitCode::SUCCESS
                }
                Err(errors) => {
                    errors.iter().for_each(|e| eprintln!("{e}"));
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("Cách dùng:\n  raso check <thư-mục-đề>...\n  raso score <rules.yaml> <biên-bản.yaml> [--json]\n  raso rank <rules.yaml> <kết-quả.json>...");
            ExitCode::FAILURE
        }
    }
}
```

Ghi chú: `raso rank` đọc file JSON dạng `MatchResult` (`{ "match_id": "...", "sheet": {...} }`).

- [ ] **Step 6: Chạy test và CLI**

Run: `cd crates/raso-rules && cargo test && cargo run --features cli --bin raso -- check ../../pkgs/games/fgc-2026`
Expected: mọi test PASS; CLI in 8 dòng `✓ fgc-2026 / 0X-....yaml`, thoát mã 0.

Nếu một ca trong `tests/scoring` đỏ, **không sửa số mong đợi** trước khi tính tay lại theo spec phụ lục A; số trong kế hoạch đã được kiểm bằng `decimal.Decimal` của Python.

- [ ] **Step 7: Phá thử**

(a) Trong `rules.yaml`, đổi Knockdown thành `ext_wildfire >= red.su_wildfire && ...` → ca `03-knockdown-tie` đỏ. (b) Đổi `su_points` thành `su_wildfire * climb_mult` (bỏ `ceil`) → `04-decimal-trap` vẫn xanh nhờ `rounding: ceil` ở điểm cuối nhưng ca `01-basic` vẫn xanh — vì vậy thêm phá thử (c): đổi `rounding: ceil` thành `rounding: floor` → `04-decimal-trap` đỏ (7,35 → 7). (d) Xoá `08-observation-error.yaml` khỏi thư mục → `fgc_2026_has_eight_cases` đỏ. Hoàn tác tất cả.

- [ ] **Step 8: Commit**

```bash
git add crates/raso-rules pkgs/games/index.json pkgs/games/fgc-2026/rules.yaml pkgs/games/fgc-2026/tests/scoring
git commit -m "feat(rules): đề FGC 2026 + 8 ca test, trình chạy test đề và CLI raso"
```

---

### Task 7: Đề FGC 2025 (chỉ luật)

**Files:**
- Create: `pkgs/games/fgc-2025/manifest.json`
- Create: `pkgs/games/fgc-2025/rules.yaml`
- Create: `pkgs/games/fgc-2025/tests/scoring/01-even-level4.yaml` … `09-coop5.yaml`
- Modify: `crates/raso-rules/tests/packs.rs`

**Interfaces:**
- Consumes: `check_pack` (Task 6).
- Produces: đề `fgc-2025` được `index.json` (Task 6) tham chiếu.

- [ ] **Step 1: Viết test thất bại** — thêm vào `crates/raso-rules/tests/packs.rs`:

```rust
#[test]
fn fgc_2025_has_nine_cases() {
    let report = check_pack(&games_root().join("fgc-2025")).unwrap();
    assert_eq!(report.cases.len(), 9);
    assert!(report.ok());
}
```

Run: `cd crates/raso-rules && cargo test --test packs fgc_2025`
Expected: FAIL (không đọc được `fgc-2025/rules.yaml`).

- [ ] **Step 2: Viết `manifest.json` và `rules.yaml`**

`pkgs/games/fgc-2025/manifest.json`:

```json
{
  "id": "fgc-2025",
  "name": "Eco Equilibrium",
  "version": "1.0.0",
  "engineVersion": ">=0.1.0",
  "rules": { "scoring": "rules.yaml" }
}
```

`pkgs/games/fgc-2025/rules.yaml`: chép nguyên văn khối YAML ở spec mục 6.2.

- [ ] **Step 3: Viết 9 file test** trong `pkgs/games/fgc-2025/tests/scoring/`. Mỗi file dùng khung dưới đây, chỉ thay `name`, `field`, `robots` và `expect` theo bảng:

```yaml
name: "<tên>"
observation:
  game: fgc-2025
  field: { barriers_mitigated: <B>, barriers_in_ecosystems: <BIE>, bio_freshwater: <F>, bio_marine: <M>, bio_terrestrial: <T> }
  sides:
    red:
      robots: [ { rope: <R1> }, { rope: <R2> }, { rope: <R3> } ]
    blue:
      robots: [ { rope: <B1> }, { rope: <B2> }, { rope: <B3> } ]
expect:
  field: { distribution: <DF>, coop: <C> }
  sides:
    red: { score: <RS>, values: { protection: <RPM> } }
    blue: { score: <BS>, values: { protection: <BPM> } }
```

| File | Tên | B | BIE | F/M/T | Robot đỏ | Robot xanh | DF | C | RPM | RS | BPM | BS |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `01-even-level4.yaml` | Phân bố đều, đỏ cả 3 LEVEL_4 | 9 | 0 | 40/40/40 | L4,L4,L4 | N,N,N | 1.0 | 0 | 2.5 | 323 | 1 | 129 |
| `02-somewhat-even.yaml` | σ ≈ 8,165 → DF 0,6 | 5 | 0 | 30/40/50 | L1,L2,N | L3,L2,L2 | 0.6 | 0 | 1.375 | 106 | 1.875 | 145 |
| `03-uneven-coop6.yaml` | σ ≈ 28,3 → DF 0,5; 6 robot treo → 30 | 9 | 0 | 60/0/0 | L2,L2,L2 | L3,L3,L4 | 0.5 | 30 | 1.75 | 99 | 2.25 | 118 |
| `04-barrier-remains.yaml` | Còn BARRIER trong ECOSYSTEM → DF 0,5 dù σ = 0 | 8 | 1 | 40/40/40 | N,N,N | N,N,N | 0.5 | 0 | 1 | 68 | 1 | 68 |
| `05-sigma-below-1.yaml` | σ ≈ 0,943 → DF 1,0 | 0 | 0 | 0/0/2 | N,N,N | N,N,N | 1 | 0 | 1 | 2 | 1 | 2 |
| `06-sigma-above-1.yaml` | σ ≈ 1,247 → DF 0,6 | 0 | 0 | 0/1/3 | N,N,N | N,N,N | 0.6 | 0 | 1 | 3 | 1 | 3 |
| `07-sigma-below-10.yaml` | σ ≈ 9,978 → DF 0,6 | 0 | 0 | 0/8/24 | N,N,N | N,N,N | 0.6 | 0 | 1 | 20 | 1 | 20 |
| `08-sigma-above-10.yaml` | σ ≈ 10,033 → DF 0,5 | 0 | 0 | 0/4/23 | N,N,N | N,N,N | 0.5 | 0 | 1 | 14 | 1 | 14 |
| `09-coop5.yaml` | 5 robot treo → 15 | 0 | 0 | 0/0/0 | L2,L2,L2 | L2,L2,N | 1 | 15 | 1.75 | 15 | 1.5 | 15 |

Ký hiệu: `N` = `NONE`, `L1`…`L4` = `LEVEL_1`…`LEVEL_4`. Ví dụ đầy đủ `01-even-level4.yaml`:

```yaml
name: "Phân bố đều, đỏ cả 3 LEVEL_4"
observation:
  game: fgc-2025
  field: { barriers_mitigated: 9, barriers_in_ecosystems: 0, bio_freshwater: 40, bio_marine: 40, bio_terrestrial: 40 }
  sides:
    red:
      robots: [ { rope: LEVEL_4 }, { rope: LEVEL_4 }, { rope: LEVEL_4 } ]
    blue:
      robots: [ { rope: NONE }, { rope: NONE }, { rope: NONE } ]
expect:
  field: { distribution: 1.0, coop: 0 }
  sides:
    red: { score: 323, values: { protection: 2.5 } }
    blue: { score: 129, values: { protection: 1 } }
```

- [ ] **Step 4: Chạy test**

Run: `cd crates/raso-rules && cargo test --test packs && cargo run --features cli --bin raso -- check ../../pkgs/games/fgc-2025`
Expected: PASS; CLI in 9 dòng `✓`.

- [ ] **Step 5: Phá thử**

(a) Trong `rules.yaml` đổi `sigma <= 1` thành `sigma < 1` → không ca nào đỏ (vì không có σ = 1 nguyên) — ghi nhận, rồi đổi `sigma < 10` thành `sigma <= 9` → `07-sigma-below-10` đỏ. (b) Bỏ nhánh `if barriers_in_ecosystems > 0 { 0.5 } else` → `04-barrier-remains` đỏ. (c) Đổi `stddev` thành `avg` → nhiều ca đỏ. Hoàn tác.

- [ ] **Step 6: Commit**

```bash
git add pkgs/games/fgc-2025 crates/raso-rules/tests/packs.rs
git commit -m "feat(rules): đề FGC 2025 Eco Equilibrium + 9 ca test ranh giới σ"
```

---

### Task 8: Bản WASM và test tương đương trên Node

**Files:**
- Create: `crates/raso-rules/src/wasm.rs`
- Modify: `crates/raso-rules/src/lib.rs`
- Create: `scripts/build-rules-wasm.sh`
- Modify: `web/package.json` (devDependencies `vitest`, `yaml`; script `test:rules`)
- Create: `web/vitest.config.ts`
- Create: `web/src/lib/raso.parity.test.ts`

**Interfaces:**
- Consumes: `load`, `score`, `rank`, `MapResolver`, `Rules::blank_observation`.
- Produces (JS, từ `crates/raso-rules/pkg/raso_rules.js`):
  - `class RulesHandle { constructor(yaml: string, parentsJson: string); describe(): string; blankObservation(): string; score(observationJson: string): string; rank(resultsJson: string): string }`
  - `describe()` trả JSON của `RulesFile` đã kế thừa.
  - `score()` trả `{"ok":true,"sheet":{...}}` hoặc `{"ok":false,"errors":[...]}`; `rank()` tương tự với `standings`.
  - Constructor ném `Error` có `message` là JSON mảng `RuleError` khi luật lỗi.
  - Hàm khởi tạo mặc định `init()` và `initSync({ module })` do wasm-bindgen sinh.

- [ ] **Step 1: Cài công cụ**

Run: `rustup target add wasm32-unknown-unknown && cargo install wasm-pack --locked`
Expected: `wasm-pack --version` in ra phiên bản.

- [ ] **Step 2: Viết `wasm.rs`**

```rust
use crate::{MapResolver, MatchResult, Observation, Rules, load, rank, score};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct RulesHandle {
    rules: Rules,
}

#[wasm_bindgen]
impl RulesHandle {
    /// `parents_json`: `{"fgc-2026": "<yaml>"}` cho các đề cha (có thể `{}`).
    #[wasm_bindgen(constructor)]
    pub fn new(yaml: &str, parents_json: &str) -> Result<RulesHandle, JsError> {
        let parents: BTreeMap<String, String> = serde_json::from_str(parents_json).map_err(|e| JsError::new(&e.to_string()))?;
        load(yaml, &MapResolver(parents))
            .map(|rules| RulesHandle { rules })
            .map_err(|errors| JsError::new(&serde_json::to_string(&errors).unwrap_or_default()))
    }

    pub fn describe(&self) -> String {
        serde_json::to_string(&self.rules.file).unwrap_or_default()
    }

    #[wasm_bindgen(js_name = blankObservation)]
    pub fn blank_observation(&self) -> String {
        serde_json::to_string(&self.rules.blank_observation()).unwrap_or_default()
    }

    pub fn score(&self, observation_json: &str) -> String {
        let obs: Observation = match serde_json::from_str(observation_json) {
            Ok(o) => o,
            Err(e) => return serde_json::json!({ "ok": false, "errors": [{ "code": "OBS_JSON", "path": "", "message": e.to_string() }] }).to_string(),
        };
        match score(&self.rules, &obs) {
            Ok(sheet) => serde_json::json!({ "ok": true, "sheet": sheet }).to_string(),
            Err(errors) => serde_json::json!({ "ok": false, "errors": errors }).to_string(),
        }
    }

    pub fn rank(&self, results_json: &str) -> String {
        let results: Vec<MatchResult> = match serde_json::from_str(results_json) {
            Ok(r) => r,
            Err(e) => return serde_json::json!({ "ok": false, "errors": [{ "code": "OBS_JSON", "path": "", "message": e.to_string() }] }).to_string(),
        };
        match rank(&self.rules, &results) {
            Ok(standings) => serde_json::json!({ "ok": true, "standings": standings }).to_string(),
            Err(errors) => serde_json::json!({ "ok": false, "errors": errors }).to_string(),
        }
    }
}
```

`lib.rs`: thêm

```rust
#[cfg(feature = "wasm")]
pub mod wasm;
```

`scripts/build-rules-wasm.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../crates/raso-rules"
wasm-pack build --release --target web --out-dir pkg --out-name raso_rules -- --features wasm
ls -la pkg/raso_rules_bg.wasm
```

Run: `chmod +x scripts/build-rules-wasm.sh && ./scripts/build-rules-wasm.sh`
Expected: tạo `crates/raso-rules/pkg/raso_rules.js`, `raso_rules_bg.wasm` (khoảng 1–2 MB).

- [ ] **Step 3: Viết test tương đương thất bại** `web/src/lib/raso.parity.test.ts`:

```ts
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it, beforeAll } from 'vitest';
import { parse } from 'yaml';
import { initSync, RulesHandle } from '../../../crates/raso-rules/pkg/raso_rules.js';

const gamesRoot = join(__dirname, '../../../pkgs/games');

type Case = {
	name: string;
	observation: unknown;
	expect?: { sides: Record<string, { score?: number }> };
	expect_error?: string;
};

beforeAll(() => {
	initSync({ module: readFileSync(join(__dirname, '../../../crates/raso-rules/pkg/raso_rules_bg.wasm')) });
});

const packs = readdirSync(gamesRoot).filter((id) => existsSync(join(gamesRoot, id, 'rules.yaml')));

describe.each(packs)('WASM khớp bộ test của đề %s', (id) => {
	const dir = join(gamesRoot, id);
	const handle = () => new RulesHandle(readFileSync(join(dir, 'rules.yaml'), 'utf8'), '{}');
	const files = readdirSync(join(dir, 'tests/scoring')).filter((f) => f.endsWith('.yaml'));

	it.each(files)('%s', (file) => {
		const testCase = parse(readFileSync(join(dir, 'tests/scoring', file), 'utf8')) as Case;
		const result = JSON.parse(handle().score(JSON.stringify(testCase.observation)));
		if (testCase.expect_error) {
			expect(result.ok).toBe(false);
			expect(result.errors.map((e: { code: string }) => e.code)).toContain(testCase.expect_error);
			return;
		}
		expect(result.ok, JSON.stringify(result.errors)).toBe(true);
		for (const [side, want] of Object.entries(testCase.expect!.sides)) {
			if (want.score !== undefined) expect(result.sheet.sides[side].score).toBe(want.score);
		}
	});
});

it('luật lỗi ném Error có danh sách RuleError', () => {
	expect(() => new RulesHandle('schema: raso.rules/v1\ngame: x', '{}')).toThrow(/RULE_YAML/);
});

it('blankObservation tính được điểm 0', () => {
	const h = new RulesHandle(readFileSync(join(gamesRoot, 'fgc-2026/rules.yaml'), 'utf8'), '{}');
	const result = JSON.parse(h.score(h.blankObservation()));
	expect(result.ok).toBe(true);
	expect(result.sheet.sides.red.score).toBe(0);
});
```

`web/vitest.config.ts`:

```ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
	test: { include: ['src/**/*.test.ts'], environment: 'node' }
});
```

Thêm vào `web/package.json`: `"devDependencies"` có `"vitest": "^3.2.4"` và `"yaml": "^2.8.1"`; `"scripts"` có `"test:rules": "vitest run src/lib/raso.parity.test.ts"`.

Run: `cd web && pnpm install && pnpm test:rules`
Expected: FAIL nếu chưa build WASM; PASS sau Step 2. Chạy lại sau khi build để xác nhận PASS (17 ca + 2 test lẻ).

Ghi chú: phiên bản `vitest`/`yaml` ở trên là mốc tối thiểu; dùng bản ổn định hiện có khi `pnpm add -D vitest yaml`, miễn tương thích Vite 8 của dự án.

- [ ] **Step 4: Phá thử**

Trong `wasm.rs`, đổi `score(&self.rules, &obs)` sang một `Observation` rỗng (`self.rules.blank_observation()`) → mọi ca có điểm khác 0 phải đỏ. Build lại, chạy, hoàn tác, build lại.

- [ ] **Step 5: Commit**

```bash
git add crates/raso-rules/src scripts/build-rules-wasm.sh web/package.json web/pnpm-lock.yaml web/vitest.config.ts web/src/lib/raso.parity.test.ts
git commit -m "feat(rules): bản WASM RulesHandle + test tương đương trên Node"
```

---
### Task 9: Server — `observe.yaml` và dựng biên bản từ mô phỏng

**Files:**
- Modify: `server/Cargo.toml` (thêm `raso-rules = { path = "../crates/raso-rules" }`, `serde_yaml_ng = "0.10"`, đổi `rhai = "1.25.1"` thành `rhai = { version = "1.25.1", features = ["decimal", "sync"] }`)
- Create: `server/src/game/scoring_bridge.rs`
- Modify: `server/src/game/mod.rs` (thêm `pub mod scoring_bridge;`)
- Create: `pkgs/games/fgc-2026/observe.yaml`
- Modify: `server/src/game/sphere_runtime/scoring.rs` (thay toàn bộ nội dung)
- Modify: `server/src/game/sphere_runtime/tests.rs:368-420` (viết lại test chấm điểm thành test đếm)

**Interfaces:**
- Consumes: `raso_rules::{Rules, Observation, InputValue, SideObservation}`, `Rules::blank_observation`, `rules.file.inputs`.
- Produces:
  - `pub trait ObservationSource { fn object_kind(&self) -> Option<String>; fn count_in_target(&self, target: &str, released_by: Option<&str>) -> Option<i64>; fn robot_brace_labels(&self) -> Vec<(String, String)>; }` — `count_in_target` trả `None` khi target không tồn tại hoặc `enabled: false`; `robot_brace_labels` trả `(liên_minh_viết_thường, nhãn)` với nhãn ∈ `none | contact | zone1 | zone2 | zone3`.
  - `pub struct ObserveSpec { entries: Vec<(Level, String, Probe)> }`
  - `pub fn parse_observe(yaml: &str, rules: &Rules) -> Result<ObserveSpec, String>`
  - `pub fn build_observation(spec: &ObserveSpec, rules: &Rules, source: &dyn ObservationSource, locked_robots: Option<&[(String, String)]>) -> Observation` — đệm robot thiếu bằng `blank_observation`, cắt robot thừa, ghi `not_simulated`.
  - `impl SphereRuntime { pub fn count_in_target(&self, target_id: &str, released_by: Option<&str>) -> Option<i64>; pub fn robot_brace_labels(&self, triggers: &[FieldTrigger]) -> Vec<(String, String)> }`

- [ ] **Step 1: Viết `pkgs/games/fgc-2026/observe.yaml`**

```yaml
# Ánh xạ mỗi ô biên bản của rules.yaml sang một phép đo trong mô phỏng.
field.ext_wildfire:      { count_objects: wildfire, in_target: extinguisher }
side.su_wildfire:        { count_objects: wildfire, in_target: "{side}-suppression-unit", last_released_by: "{side}" }
robot.climb:             { brace_state: { none: NONE, contact: CONTACT, zone1: ZONE_1, zone2: ZONE_2, zone3: ZONE_3 } }
robot.lifted_by_partner: { not_simulated: false }
```

- [ ] **Step 2: Viết test thất bại** — tạo `server/src/game/scoring_bridge.rs` với khối test:

```rust
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
        let fake = Fake { red_su: 100, blue_su: 0, robots: vec![("red".into(), "zone2".into()), ("red".into(), "contact".into())] };
        let obs = build_observation(&spec, &rules, &fake, None);
        assert_eq!(obs.sides["red"].robots.len(), 3);
        assert_eq!(obs.sides["blue"].robots.len(), 3);
        assert_eq!(obs.source.as_deref(), Some("simulation"));
        assert!(obs.not_simulated.contains(&"field.ext_wildfire".to_string()));
        assert!(obs.not_simulated.contains(&"robot.lifted_by_partner".to_string()));
        let sheet = score(&rules, &obs).expect("biên bản mô phỏng phải hợp lệ");
        assert_eq!(sheet.sides["red"].score, 125); // ceil(100 * 1.25)
    }

    #[test]
    fn locked_robots_override_live_brace_state() {
        let rules = load(RULES, &NoParents).unwrap();
        let spec = parse_observe(OBSERVE, &rules).unwrap();
        let fake = Fake { red_su: 10, blue_su: 0, robots: vec![] };
        let locked = vec![("red".to_string(), "zone3".to_string())];
        let obs = build_observation(&spec, &rules, &fake, Some(&locked));
        assert_eq!(obs.sides["red"].robots[0]["climb"], raso_rules::InputValue::Text("ZONE_3".into()));
    }

    #[test]
    fn observe_must_cover_every_input_exactly_once() {
        let rules = load(RULES, &NoParents).unwrap();
        let missing = OBSERVE.lines().filter(|l| !l.starts_with("robot.lifted_by_partner")).collect::<Vec<_>>().join("\n");
        assert!(parse_observe(&missing, &rules).unwrap_err().contains("robot.lifted_by_partner"));
        let extra = format!("{OBSERVE}\nside.bogus: {{ not_simulated: 0 }}\n");
        assert!(parse_observe(&extra, &rules).unwrap_err().contains("side.bogus"));
    }

    #[test]
    fn unknown_brace_label_mapping_is_rejected() {
        let rules = load(RULES, &NoParents).unwrap();
        let bad = OBSERVE.replace("zone3: ZONE_3", "zone3: ZONE_9");
        assert!(parse_observe(&bad, &rules).unwrap_err().contains("ZONE_9"));
    }
}
```

Run: `cd server && cargo test scoring_bridge`
Expected: lỗi biên dịch.

- [ ] **Step 3: Viết code `scoring_bridge.rs`** (phía trên khối test)

```rust
//! Dựng `raso_rules::Observation` từ trạng thái mô phỏng theo `observe.yaml`.
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
    Count { object: String, target: String, released_by: Option<String> },
    Brace(BTreeMap<String, String>),
    NotSimulated(InputValue),
}

#[derive(Debug, Clone)]
pub struct ObserveSpec {
    entries: Vec<(Level, String, Probe)>,
}

fn level_inputs(rules: &Rules, level: Level) -> &indexmap::IndexMap<String, InputDef> {
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

pub fn parse_observe(yaml: &str, rules: &Rules) -> Result<ObserveSpec, String> {
    let raw: BTreeMap<String, ProbeDef> = serde_yaml_ng::from_str(yaml).map_err(|e| format!("observe.yaml không hợp lệ: {e}"))?;
    let mut entries = Vec::new();
    for (key, def) in raw {
        let (level, name) = match key.split_once('.') {
            Some(("field", n)) => (Level::Field, n),
            Some(("side", n)) => (Level::Side, n),
            Some(("robot", n)) => (Level::Robot, n),
            _ => return Err(format!("{key}: khoá phải có dạng field.<ô>, side.<ô> hoặc robot.<ô>")),
        };
        let input = level_inputs(rules, level).get(name).ok_or_else(|| format!("{key}: không có ô này trong rules.yaml"))?;
        let probe = match (&def.count_objects, &def.brace_state, &def.not_simulated) {
            (Some(object), None, None) => {
                let target = def.in_target.clone().ok_or_else(|| format!("{key}: count_objects cần in_target"))?;
                if input.kind != InputKind::Count {
                    return Err(format!("{key}: count_objects chỉ dùng cho ô count"));
                }
                Probe::Count { object: object.clone(), target, released_by: def.last_released_by.clone() }
            }
            (None, Some(map), None) => {
                if level != Level::Robot || input.kind != InputKind::Choice {
                    return Err(format!("{key}: brace_state chỉ dùng cho ô choice cấp robot"));
                }
                for value in map.values() {
                    if !input.values.contains(value) {
                        return Err(format!("{key}: '{value}' không có trong values của ô"));
                    }
                }
                Probe::Brace(map.clone())
            }
            (None, None, Some(v)) => Probe::NotSimulated(yaml_to_input(v).ok_or_else(|| format!("{key}: not_simulated cần bool/số nguyên/chuỗi"))?),
            _ => return Err(format!("{key}: cần đúng một trong count_objects / brace_state / not_simulated")),
        };
        entries.push((level, name.to_string(), probe));
    }
    for level in [Level::Field, Level::Side, Level::Robot] {
        let prefix = match level { Level::Field => "field", Level::Side => "side", Level::Robot => "robot" };
        for name in level_inputs(rules, level).keys() {
            if !entries.iter().any(|(l, n, _)| *l == level && n == name) {
                return Err(format!("{prefix}.{name}: ô của rules.yaml chưa có phép đo trong observe.yaml"));
            }
        }
    }
    Ok(ObserveSpec { entries })
}

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
        Some(r) => r,
        None => {
            live_robots = source.robot_brace_labels();
            &live_robots
        }
    };
    let kind = source.object_kind();
    for (level, name, probe) in &spec.entries {
        let prefix = match level { Level::Field => "field", Level::Side => "side", Level::Robot => "robot" };
        let path = format!("{prefix}.{name}");
        match (level, probe) {
            (_, Probe::NotSimulated(value)) => {
                obs.not_simulated.push(path.clone());
                match level {
                    Level::Field => {
                        obs.field.insert(name.clone(), value.clone());
                    }
                    Level::Side => obs.sides.values_mut().for_each(|s| {
                        s.inputs.insert(name.clone(), value.clone());
                    }),
                    Level::Robot => obs.sides.values_mut().flat_map(|s| s.robots.iter_mut()).for_each(|r| {
                        r.insert(name.clone(), value.clone());
                    }),
                }
            }
            (Level::Field, Probe::Count { object, target, released_by }) => {
                let n = if kind.as_deref() == Some(object.as_str()) { source.count_in_target(target, released_by.as_deref()) } else { None };
                if n.is_none() {
                    obs.not_simulated.push(path.clone());
                }
                obs.field.insert(name.clone(), InputValue::Int(n.unwrap_or(0)));
            }
            (Level::Side, Probe::Count { object, target, released_by }) => {
                for (id, side) in obs.sides.iter_mut() {
                    let target = target.replace("{side}", id);
                    let released = released_by.as_ref().map(|r| r.replace("{side}", id));
                    let n = if kind.as_deref() == Some(object.as_str()) { source.count_in_target(&target, released.as_deref()) } else { None };
                    if n.is_none() && !obs.not_simulated.contains(&path) {
                        obs.not_simulated.push(path.clone());
                    }
                    side.inputs.insert(name.clone(), InputValue::Int(n.unwrap_or(0)));
                }
            }
            (Level::Robot, Probe::Brace(map)) => {
                for (id, side) in obs.sides.iter_mut() {
                    let labels: Vec<&String> = robots.iter().filter(|(s, _)| s == id).map(|(_, l)| l).collect();
                    for (slot, robot) in side.robots.iter_mut().enumerate() {
                        let label = labels.get(slot).map(|l| l.as_str()).unwrap_or("none");
                        if let Some(value) = map.get(label) {
                            robot.insert(name.clone(), InputValue::Text(value.clone()));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    obs.not_simulated.sort();
    obs.not_simulated.dedup();
    obs
}
```

Ghi chú: biên bản luôn có đúng `sides.teams` robot vì xuất phát từ `blank_observation`; robot thừa trong `robot_brace_labels` bị bỏ qua, robot thiếu giữ giá trị trống. Nếu `indexmap` chưa có trong `server/Cargo.toml`, dùng `raso_rules::schema` re-export hoặc thêm `indexmap = "2"`.

- [ ] **Step 4: Thay `sphere_runtime/scoring.rs` bằng phép đo**

```rust
use super::*;

impl SphereRuntime {
    /// Số bóng đang hoạt động nằm trong vùng target. `released_by` lọc theo
    /// liên minh đã bắn bóng ra lần cuối. `None` nếu target không có/đang tắt.
    pub fn count_in_target(&self, target_id: &str, released_by: Option<&str>) -> Option<i64> {
        let target = self.scoring_targets.iter().find(|t| t.id == target_id && t.enabled)?;
        Some(
            self.balls
                .iter()
                .filter(|ball| {
                    ball.active
                        && point_inside_aabb(ball.position, target.min, target.max)
                        && released_by.is_none_or(|side| ball.last_outtake_alliance.as_deref() == Some(side))
                })
                .count() as i64,
        )
    }

    /// Nhãn trạng thái leo của từng robot, theo thứ tự id người chơi.
    pub fn robot_brace_labels(&self, triggers: &[FieldTrigger]) -> Vec<(String, String)> {
        let ground_offset_y = self.arena.as_ref().map(|arena| -arena.robot.height_m * 0.5).unwrap_or_default();
        let mut players: Vec<_> = self.players.iter().collect();
        players.sort_by(|a, b| a.0.cmp(b.0));
        players
            .into_iter()
            .filter_map(|(_, player)| {
                let team = player.team_name.to_ascii_lowercase();
                let side = if team.starts_with("red") { "red" } else if team.starts_with("blue") { "blue" } else { return None };
                let state = brace::brace_state_for_player(player, self.robot_definition.as_ref(), ground_offset_y, &self.field_colliders, triggers);
                let label = match (state.zone, state.multiplier > 1.0) {
                    (Some(zone), _) => format!("zone{zone}"),
                    (None, true) => "contact".to_string(),
                    (None, false) => "none".to_string(),
                };
                Some((side.to_string(), label))
            })
            .collect()
    }
}
```

Trong `sphere_runtime.rs`: xoá `score_state`, `scoring_enabled`, `scored_target_by_ball`, `semantic_events`-phần chấm điểm (`score_added`/`score_removed` chỉ sinh trong `reconcile_scoring` cũ), các lời gọi `self.reconcile_scoring()` (dòng ~968, ~1011), `set_scoring_enabled`, `brace_multipliers`, `apply_endgame_brace_multipliers`, `apply_su_multiplier`, và các dòng reset tương ứng (~726-727, ~751, ~804). Giữ `semantic_events` và `drain_semantic_events` nếu còn nơi khác sinh sự kiện; nếu không còn nơi nào ghi vào thì xoá luôn và cập nhật `match_registry.rs` ở Task 10.

Trong `pack_loader.rs` và `pack_loader/field.rs`: xoá trường `points`, `requires_robot_outtake` khỏi `ScoringTargetConfig` và `FieldScoringTarget`.

Trong `pkgs/games/fgc-2026/manifest.json`: xoá `"points"` và `"requiresRobotOuttake"` khỏi mọi target.

- [ ] **Step 5: Viết lại test cũ** — thay test `scores_only_matching_alliance_outtakes_and_reverses_on_exit` trong `sphere_runtime/tests.rs` bằng:

```rust
#[test]
fn counts_only_balls_released_by_the_matching_alliance_and_drops_on_exit() {
    let mut arena = arena();
    arena.object_count = 1;
    arena.gravity_scale = 0.0;
    arena.ramp.enabled = false;
    let field = FieldDefinition {
        colliders: Vec::new(),
        anchors: BTreeMap::new(),
        triggers: Vec::new(),
        scoring_targets: vec![FieldScoringTarget {
            id: "blue-suppression-unit".into(),
            kind: "suppression-unit".into(),
            alliance: Some("blue".into()),
            enabled: true,
            min: [-1.0, 0.0, -1.0],
            max: [1.0, 1.0, 1.0],
        }],
        floor_height_m: 0.0,
        boundary: FieldBoundary::default(),
    };
    let mut runtime = SphereRuntime::new("semantic".into(), "fgc-2026".into(), 0);
    runtime.create_field_arena(&arena, &field);
    runtime.balls[0].position = [0.0, 0.5, 0.0];
    runtime.balls[0].active = true;
    runtime.tick(1.0 / 60.0);
    assert_eq!(runtime.count_in_target("blue-suppression-unit", Some("blue")), Some(0), "bóng chưa bắn không tính");
    assert_eq!(runtime.count_in_target("blue-suppression-unit", None), Some(1), "đếm thô vẫn thấy bóng");

    runtime.balls[0].last_outtake_alliance = Some("red".into());
    assert_eq!(runtime.count_in_target("blue-suppression-unit", Some("blue")), Some(0), "liên minh đối thủ không tính");

    runtime.balls[0].last_outtake_alliance = Some("blue".into());
    assert_eq!(runtime.count_in_target("blue-suppression-unit", Some("blue")), Some(1));

    runtime.balls[0].position[2] = -1.5;
    assert_eq!(runtime.count_in_target("blue-suppression-unit", Some("blue")), Some(0), "ra khỏi vùng thì thôi tính");
    assert_eq!(runtime.count_in_target("nope", None), None);
}
```

Trong test reset ở `sphere_runtime/tests.rs:152-155`, xoá hai dòng `runtime.score_state.red_score = 11;` và `assert_eq!(runtime.score_state.red_score, 0);`.

- [ ] **Step 6: Chạy test server**

Run: `cd server && cargo test --release`
Expected: biên dịch được; `scoring_bridge` 4 test PASS; test đếm mới PASS. Các test còn tham chiếu `score_state`/`apply_score`/`on_trigger_enter` trong `match_registry` sẽ được sửa ở Task 10 — nếu chúng làm hỏng biên dịch ở bước này, gộp phần xoá tương ứng của Task 10 Step 3 vào đây.

- [ ] **Step 7: Phá thử**

(a) Trong `count_in_target`, bỏ điều kiện `released_by.is_none_or(...)` → test đếm đỏ ở dòng "liên minh đối thủ". (b) Trong `build_observation`, bỏ đoạn `obs.not_simulated.push(path.clone())` của nhánh `Field Count` → `builds_a_valid_observation…` đỏ. Hoàn tác.

- [ ] **Step 8: Commit**

```bash
git add server pkgs/games/fgc-2026/observe.yaml pkgs/games/fgc-2026/manifest.json
git commit -m "feat(server): dựng biên bản từ mô phỏng theo observe.yaml, bỏ cộng điểm trong vật lý"
```

---

### Task 10: Server — nối `raso-rules` vào vòng lặp trận, gỡ điểm viết cứng

**Files:**
- Modify: `server/src/game/pack_loader.rs` (manifest `rules`, snapshot `rules`/`observe`, `GamePackMetadata.rules`/`observe`)
- Modify: `server/src/game/pack_loader/tests.rs` (bỏ `scoring.rhai`, `penalties.rhai`; thêm test nạp luật)
- Modify: `server/src/game/match_runtime.rs:43-50` (`ScoreState`)
- Modify: `server/src/game/match_registry.rs` (bỏ `apply_score`, `brace_multipliers`, `apply_endgame_brace_multipliers`, `set_scoring_enabled`, nhánh `on_trigger_enter`, `brace_score_applied`; thêm tính điểm định kỳ, chốt robot, kết quả cuối)
- Modify: `server/src/game/rhai_engine.rs` (bỏ `add_score`, `on_trigger_enter`, `outcomes`, `scoring_ast`)
- Modify: `server/src/game/rhai_engine/tests.rs` (bỏ test `on_trigger_enter`; đổi test metadata dùng hàm khác `add_score`)
- Delete: `pkgs/games/fgc-2026/rules/scoring.rhai`, `pkgs/games/fgc-2026/rules/penalties.rhai`
- Modify: `pkgs/games/fgc-2026/manifest.json` (`scripts` bỏ `scoring`, `penalties`; thêm `"rules": { "scoring": "rules.yaml", "observe": "observe.yaml" }`)

**Interfaces:**
- Consumes: `scoring_bridge::{parse_observe, build_observation, ObservationSource, ObserveSpec}`, `raso_rules::{load, score, MapResolver, Rules, ScoreSheet}`.
- Produces:
  - `GamePackManifest.rules: Option<PackRulesManifest>` với `pub struct PackRulesManifest { pub scoring: String, #[serde(default)] pub observe: Option<String> }`
  - `GamePackRuntimeSnapshot.rules: BTreeMap<String, String>` (id đề → YAML, gồm đề chính và đề cha) và `observe: Option<String>` (cả hai `#[serde(default)]`)
  - `GamePackMetadata.rules: Option<Arc<raso_rules::Rules>>`, `GamePackMetadata.observe: Option<Arc<ObserveSpec>>` (`#[serde(skip)]`)
  - `ScoreState { pub blue_score: i32, pub red_score: i32, pub global_score: i32, pub breakdown: HashMap<String, i32>, pub sheet: Option<Arc<raso_rules::ScoreSheet>>, pub scoring_error: Option<String> }` (`Default`)
  - `MatchCompletionReport.result` có thêm `observation`, `scoreSheet`, `rulesVersion`.
  - `impl ObservationSource for SphereRuntime` (giữ `triggers` qua một struct bọc `SphereObservation<'a> { runtime: &'a SphereRuntime, triggers: &'a [FieldTrigger] }`).

- [ ] **Step 1: Viết test thất bại cho nạp luật** — thêm vào `server/src/game/pack_loader/tests.rs`:

```rust
#[test]
fn fgc_2026_pack_loads_rules_and_observe() {
    let pack = PackLoader::new("0.1.0").load_pack("../pkgs/games/fgc-2026/manifest.json").unwrap();
    let rules = pack.rules.as_ref().expect("gói fgc-2026 có rules.yaml");
    assert_eq!(rules.file.game, "fgc-2026");
    assert!(pack.observe.is_some());
    assert!(!pack.manifest.scripts.values().any(|p| p.ends_with("scoring.rhai") || p.ends_with("penalties.rhai")));
}

#[test]
fn invalid_rules_reject_the_pack() {
    let loader = PackLoader::new("0.1.0");
    let mut snapshot = fixture_snapshot(); // helper hiện có trong file test; nếu chưa có, dựng từ load_pack rồi sửa
    snapshot.rules.insert("fgc-2026".into(), "schema: raso.rules/v1\ngame: fgc-2026".into());
    let error = loader.load_runtime_snapshot(snapshot).unwrap_err();
    assert!(error.to_string().contains("RULE_YAML"), "{error}");
}
```

Nếu `pack_loader/tests.rs` chưa có `fixture_snapshot()`, thêm helper đọc các file của `pkgs/games/fgc-2026` giống `load_pack` nhưng trả `GamePackRuntimeSnapshot` (sao chép phần đọc file trong `load_pack`, gồm cả đọc `rules.yaml`/`observe.yaml` mới ở Step 2).

Sửa test hiện có ở `pack_loader/tests.rs:19-21` và `:134` để không còn `rules/penalties.rhai`, `rules/scoring.rhai` trong danh sách mong đợi.

Run: `cd server && cargo test pack_loader`
Expected: FAIL (không có trường `rules`).

- [ ] **Step 2: Nạp luật trong `pack_loader.rs`**

Thêm struct và trường:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackRulesManifest {
    pub scoring: String,
    #[serde(default)]
    pub observe: Option<String>,
}
```

- `GamePackManifest`: `#[serde(default)] pub rules: Option<PackRulesManifest>,`
- `GamePackRuntimeSnapshot`: `#[serde(default)] pub rules: BTreeMap<String, String>, #[serde(default)] pub observe: Option<String>,`
- `GamePackMetadata`: `#[serde(skip)] pub rules: Option<Arc<raso_rules::Rules>>, #[serde(skip)] pub observe: Option<Arc<crate::game::scoring_bridge::ObserveSpec>>,` (bỏ `Clone`/`Debug` derive nếu `Rules` không implement; khi đó implement `Debug` tay bỏ qua hai trường này).

Trong `load_runtime_snapshot`, trước `Ok(GamePackMetadata { ... })`:

```rust
let (rules, observe) = match &snapshot.manifest.rules {
    None => (None, None),
    Some(paths) => {
        let yaml = snapshot.rules.get(&snapshot.manifest.id).ok_or_else(|| {
            GameError::ManifestParseError(format!("Runtime snapshot thiếu {} của đề {}", paths.scoring, snapshot.manifest.id))
        })?;
        let rules = raso_rules::load(yaml, &raso_rules::MapResolver(snapshot.rules.clone().into_iter().collect()))
            .map_err(|errors| GameError::ScriptCompilationError(errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")))?;
        let observe = match &snapshot.observe {
            Some(text) => Some(Arc::new(
                crate::game::scoring_bridge::parse_observe(text, &rules).map_err(GameError::ScriptCompilationError)?,
            )),
            None => None,
        };
        info!(game = %rules.file.game, version = %rules.file.version, "Loaded scoring rules");
        (Some(Arc::new(rules)), observe)
    }
};
```

và thêm `rules, observe,` vào `GamePackMetadata { ... }`.

Trong `load_pack` (test adapter), đọc thêm:

```rust
let mut rules_sources = BTreeMap::new();
let mut observe = None;
if let Some(paths) = &manifest.rules {
    rules_sources.insert(
        manifest.id.clone(),
        std::fs::read_to_string(root.join(&paths.scoring)).map_err(|e| GameError::ManifestParseError(e.to_string()))?,
    );
    if let Some(path) = &paths.observe {
        observe = Some(std::fs::read_to_string(root.join(path)).map_err(|e| GameError::ManifestParseError(e.to_string()))?);
    }
}
```

và truyền `rules: rules_sources, observe` vào `GamePackRuntimeSnapshot`.

Sửa `manifest.json` của fgc-2026: `scripts` chỉ còn `"robot_rules"` và `"arena"`; thêm `"rules": { "scoring": "rules.yaml", "observe": "observe.yaml" }`. Xoá `rules/scoring.rhai`, `rules/penalties.rhai`.

Run: `cd server && cargo test pack_loader`
Expected: PASS.

- [ ] **Step 3: Gỡ chấm điểm cũ**

- `rhai_engine.rs`: xoá `RuleOutcome`, `outcomes`, `scoring_ast`, đăng ký `add_score`, hàm `on_trigger_enter`, nhánh `if path.ends_with("scoring.rhai")`. `RhaiEngine::new()` chỉ còn `Engine::new()`.
- `rhai_engine/tests.rs`: xoá test gọi `on_trigger_enter`; trong test metadata (dòng ~9-14) đổi `add_score` thành một hàm có thật trong engine hoặc kiểm `engine_calls` rỗng theo nguồn mới, ví dụ nguồn `"fn on_tick(state) { state.x = 1; }"` và `assert!(metadata.engine_calls.is_empty())`.
- `match_runtime.rs`: `ScoreState` thành

```rust
#[derive(Debug, Clone, Default)]
pub struct ScoreState {
    pub blue_score: i32,
    pub red_score: i32,
    pub global_score: i32,
    pub breakdown: HashMap<String, i32>,
    pub sheet: Option<std::sync::Arc<raso_rules::ScoreSheet>>,
    pub scoring_error: Option<String>,
}

impl ScoreState {
    pub fn from_sheet(sheet: std::sync::Arc<raso_rules::ScoreSheet>) -> Self {
        let side = |id: &str| sheet.sides.get(id).map(|s| s.score as i32).unwrap_or(0);
        let mut breakdown = HashMap::new();
        for (id, s) in &sheet.sides {
            for item in s.values.iter().filter(|v| v.label.is_some()) {
                if let Ok(d) = item.value.parse::<rust_decimal::Decimal>() {
                    breakdown.insert(format!("{id}.{}", item.name), d.ceil().try_into().unwrap_or(0));
                }
            }
        }
        Self { blue_score: side("blue"), red_score: side("red"), global_score: 0, breakdown, sheet: Some(sheet), scoring_error: None }
    }
}
```

(thêm `rust_decimal = "1.43"` vào `server/Cargo.toml` nếu cần `try_into`; hoặc dùng `raso_rules` re-export nếu có.)
- `match_registry.rs`: xoá các hàm `apply_score`, `set_scoring_enabled`, `brace_multipliers`, `apply_endgame_brace_multipliers` trong `RuntimeBackend`; xoá `brace_score_applied`, `endgame_brace_multipliers`, nhánh `rules.on_trigger_enter` (dòng ~1144-1158) và khối `apply_endgame_brace_multipliers` (dòng ~1256-1266). Biến `rules` (RhaiEngine) trong luồng trận vẫn dùng cho kịch bản khác thì giữ, không thì xoá.

- [ ] **Step 4: Thêm `ObservationSource` cho runtime** — trong `match_registry.rs`:

```rust
struct SphereObservation<'a> {
    runtime: &'a SphereRuntime,
    triggers: &'a [FieldTrigger],
    object_kind: Option<String>,
}

impl ObservationSource for SphereObservation<'_> {
    fn object_kind(&self) -> Option<String> {
        self.object_kind.clone()
    }
    fn count_in_target(&self, target: &str, released_by: Option<&str>) -> Option<i64> {
        self.runtime.count_in_target(target, released_by)
    }
    fn robot_brace_labels(&self) -> Vec<(String, String)> {
        self.runtime.robot_brace_labels(self.triggers)
    }
}

impl RuntimeBackend {
    fn observation_source<'a>(&'a self, pack: &'a GamePackMetadata) -> Option<SphereObservation<'a>> {
        match self {
            Self::Sphere(runtime) => Some(SphereObservation {
                runtime,
                triggers: &pack.field_definition.triggers,
                object_kind: pack.manifest.objects.first().and_then(|o| o.get("id")).and_then(|v| v.as_str()).map(str::to_string),
            }),
            Self::Rapier(_) => None,
        }
    }

    fn set_score(&mut self, score: ScoreState) {
        match self {
            Self::Rapier(runtime) => runtime.score_state = score,
            Self::Sphere(runtime) => runtime.score_state = score,
        }
    }
}

/// Tính điểm từ trạng thái hiện tại. Lỗi thì giữ bảng điểm cũ và gắn cờ.
fn rescore(runtime: &mut RuntimeBackend, pack: &GamePackMetadata, locked: Option<&[(String, String)]>) -> Option<(raso_rules::Observation, Arc<raso_rules::ScoreSheet>)> {
    let (rules, observe) = (pack.rules.as_ref()?, pack.observe.as_ref()?);
    let obs = {
        let source = runtime.observation_source(pack)?;
        build_observation(observe, rules, &source, locked)
    };
    match raso_rules::score(rules, &obs) {
        Ok(sheet) => {
            let sheet = Arc::new(sheet);
            runtime.set_score(ScoreState::from_sheet(sheet.clone()));
            Some((obs, sheet))
        }
        Err(errors) => {
            let message = errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; ");
            tracing::error!(%message, "Tính điểm thất bại; giữ bảng điểm gần nhất");
            let mut state = runtime.score_state();
            state.scoring_error = Some(message);
            runtime.set_score(state);
            None
        }
    }
}
```

`SphereRuntime.score_state` vẫn là trường `pub` (giữ ở `sphere_runtime.rs`, chỉ bỏ phần cộng điểm) để `score_state()` hiện có của `RuntimeBackend` hoạt động; nếu Step 3 đã xoá trường này khỏi `SphereRuntime` thì thêm lại `pub score_state: ScoreState` và reset nó trong `reset_match`.

- [ ] **Step 5: Gọi trong vòng lặp trận** — trong luồng `match-{match_id}`:

```rust
let mut locked_robots: Option<Vec<(String, String)>> = None;
let mut final_observation: Option<(raso_rules::Observation, Arc<raso_rules::ScoreSheet>)> = None;
```

(khai báo cạnh `tick`, thay cho `brace_score_applied`/`endgame_brace_multipliers`; reset cả hai về `None` ở chỗ trước đây reset `endgame_brace_multipliers = None`.)

Thay khối `if settling && endgame_brace_multipliers.is_none() { … }` bằng:

```rust
if settling && locked_robots.is_none() {
    // Chốt trạng thái leo tại tiếng còi (spec mục 7.2 bước 3).
    locked_robots = runtime.observation_source(&pack).map(|s| s.robot_brace_labels());
}
```

Sau `runtime.step(...)` và vòng `drain_semantic_events` (giữ phần ghi sự kiện, bỏ phần chấm điểm):

```rust
if (match_running || settling) && tick % 30 == 0 {
    rescore(&mut runtime, &pack, locked_robots.as_deref());
}
```

Thay khối hoàn tất (`if live_phase_entered && !practice_continue && clock_now >= match_finalizes { … }`) bằng:

```rust
if live_phase_entered && !practice_continue && clock_now >= match_finalizes {
    if final_observation.is_none() {
        final_observation = rescore(&mut runtime, &pack, locked_robots.as_deref());
    }
    let score = runtime.score_state();
    let (observation, sheet) = match &final_observation {
        Some((o, s)) => (serde_json::to_value(o).unwrap_or_default(), serde_json::to_value(s.as_ref()).unwrap_or_default()),
        None => (serde_json::Value::Null, serde_json::Value::Null),
    };
    if let Some(reports) = &report_tx { let _ = reports.send(MatchReport::Completion(MatchCompletionReport {
        match_id: match_id.clone(), completion_id: format!("{match_id}:complete:{tick}:finished"), tick,
        reason: "finished".to_string(), game_pack_version: pack.manifest.version.clone(),
        result: serde_json::json!({
            "redScore": score.red_score, "blueScore": score.blue_score, "globalScore": score.global_score,
            "breakdown": score.breakdown, "observation": observation, "scoreSheet": sheet,
            "rulesVersion": pack.rules.as_ref().map(|r| r.file.version.clone()),
        }),
    })); }
    simulation_shutdown.store(true, Ordering::Relaxed);
}
```

Kiểm tra `api/src/routes/game-servers.ts` (`/matches/:id/complete`) lưu `result` nguyên dạng JSON; nếu có schema zod chặn khoá lạ thì thêm `observation`, `scoreSheet`, `rulesVersion` là `z.unknown().optional()`.

- [ ] **Step 6: Viết test tích hợp** — thêm vào `server/src/game/match_registry/tests.rs`:

```rust
#[test]
fn rescore_turns_balls_in_target_into_score_sheet() {
    let pack = PackLoader::new("0.1.0").load_pack("../pkgs/games/fgc-2026/manifest.json").unwrap();
    let mut runtime = RuntimeBackend::new("t".into(), &pack, 1);
    let RuntimeBackend::Sphere(sphere) = &mut runtime else { panic!("cần sphere backend") };
    let target = pack.field_definition.scoring_targets.iter().find(|t| t.id == "red-suppression-unit").unwrap().clone();
    let centre = [(target.min[0] + target.max[0]) / 2.0, (target.min[1] + target.max[1]) / 2.0, (target.min[2] + target.max[2]) / 2.0];
    sphere.place_test_balls(&[centre, centre, centre], Some("red"));
    let (obs, sheet) = rescore(&mut runtime, &pack, None).expect("tính được");
    assert_eq!(obs.sides["red"].inputs["su_wildfire"], raso_rules::InputValue::Int(3));
    assert_eq!(sheet.sides["red"].score, 3);
    assert_eq!(runtime.score_state().red_score, 3);
}
```

Thêm helper test-only vào `sphere_runtime.rs`:

```rust
#[cfg(test)]
pub fn place_test_balls(&mut self, positions: &[Vec3], released_by: Option<&str>) {
    for (ball, position) in self.balls.iter_mut().zip(positions) {
        ball.position = *position;
        ball.active = true;
        ball.last_outtake_alliance = released_by.map(str::to_string);
    }
}
```

(Nếu `RuntimeBackend::new` cần `create_field_arena` trước khi có bóng, gọi theo đúng trình tự mà các test hiện có trong `match_registry/tests.rs` dùng.)

- [ ] **Step 7: Chạy toàn bộ test + benchmark**

Run: `cd server && cargo fmt --all && cargo test --release && cargo test --release benchmark_1000_ball_robot_interaction -- --ignored --nocapture`
Expected: mọi test PASS (56 cũ trừ các test bị xoá/viết lại + test mới); benchmark p99 ≤ 0,46 ms trên Mac (không tăng quá 5% so với 0,44 ms đo ngày 15/10/2026) vì `score` không chạy trong benchmark vật lý.

Thêm benchmark `score()` (test `#[ignore]` trong `scoring_bridge.rs`):

```rust
#[test]
#[ignore]
fn benchmark_score_fgc_2026() {
    let rules = raso_rules::load(RULES, &raso_rules::NoParents).unwrap();
    let obs = rules.blank_observation();
    let started = std::time::Instant::now();
    for _ in 0..1_000 {
        raso_rules::score(&rules, &obs).unwrap();
    }
    let per_call_ms = started.elapsed().as_secs_f64();
    println!("score() fgc-2026: {per_call_ms:.4} ms/lần");
    assert!(per_call_ms < 1.0, "score() phải < 1 ms");
}
```

Run: `cd server && cargo test --release benchmark_score_fgc_2026 -- --ignored --nocapture`
Expected: in thời gian mỗi lần < 1 ms.

- [ ] **Step 8: Chạy thử trận thật** (theo `docs/LOCAL_SETUP.md`)

Khởi động api, web, server; mở `http://localhost:5173/match/arena`; bắn vài bóng vào Suppression Unit của liên minh mình. Expected: điểm trên bảng tăng trong vòng 0,5 giây; log server có dòng `Loaded scoring rules game=fgc-2026`.

- [ ] **Step 9: Phá thử**

Trong vòng lặp, đổi `tick % 30 == 0` thành `false` → `rescore_turns…` vẫn xanh (gọi trực tiếp) nhưng bước 8 thấy điểm không tăng — ghi nhận đây là kiểm chứng thủ công. Đổi `locked_robots.as_deref()` thành `None` ở khối hoàn tất → không có test tự động bắt được; thêm test:

```rust
#[test]
fn locked_robot_labels_are_used_for_final_score() {
    let pack = PackLoader::new("0.1.0").load_pack("../pkgs/games/fgc-2026/manifest.json").unwrap();
    let mut runtime = RuntimeBackend::new("t".into(), &pack, 1);
    let locked = vec![("red".to_string(), "zone3".to_string())];
    let (obs, _) = rescore(&mut runtime, &pack, Some(&locked)).unwrap();
    assert_eq!(obs.sides["red"].robots[0]["climb"], raso_rules::InputValue::Text("ZONE_3".into()));
}
```

rồi phá `rescore` để bỏ qua `locked` → test này đỏ. Hoàn tác.

- [ ] **Step 10: Commit**

```bash
git add server pkgs/games/fgc-2026
git commit -m "feat(server): tính điểm trận bằng raso-rules, gỡ điểm viết cứng và scoring.rhai"
```

---

### Task 11: Giao thức section 14 và bảng "Chi tiết điểm" trên web

**Files:**
- Modify: `server/src/game/match_registry/protocol.rs` (hằng `SCORE_SHEET: u16 = 14`, minor 5 → 6, ghi section)
- Modify: `server/src/game/match_registry/tests.rs` hoặc test protocol hiện có
- Modify: `docs/match-state-protocol.md`
- Modify: `web/src/routes/(game)/match/test-match/match-protocol.ts` (đọc tag 14)
- Create: `web/src/routes/(game)/match/test-match/ScoreSheetPanel.svelte`
- Modify: `web/src/routes/(game)/match/test-match/+page.svelte` (hiển thị panel)

**Interfaces:**
- Consumes: `ScoreState.sheet`, `ScoreState.scoring_error`.
- Produces:
  - Section 14 payload: `put_string(json)` với `json = {"sheet": ScoreSheet | null, "scoringError": string | null}`.
  - TS: `snapshot.scoreSheet: { sheet: ScoreSheetJson | null; scoringError: string | null } | null`
  - `type ScoreSheetJson = { game: string; rules_version: string; field: LineItem[]; sides: Record<string, { values: LineItem[]; score: number; raw_score: string; fouls_received: string }>; not_simulated: string[] }`, `type LineItem = { name: string; label: string | null; label_en: string | null; value: string }`.

- [ ] **Step 1: Viết test thất bại (Rust)** — thêm vào nơi chứa test protocol hiện có (tìm bằng `grep -rn "FGS1" server/src/game/match_registry`):

```rust
#[test]
fn snapshot_carries_score_sheet_section_14() {
    let mut state = test_state(); // helper hiện có dùng cho test kích thước 1.000 bóng
    let rules = raso_rules::load(include_str!("../../../../pkgs/games/fgc-2026/rules.yaml"), &raso_rules::NoParents).unwrap();
    let sheet = std::sync::Arc::new(raso_rules::score(&rules, &rules.blank_observation()).unwrap());
    state.score = ScoreState::from_sheet(sheet);
    let bytes = encode_state(&state, ProcessMetrics::default(), false, false);
    assert_eq!(u16::from_le_bytes([bytes[6], bytes[7]]), 6, "minor version");
    let mut offset = 16;
    let mut found = None;
    while offset + 8 <= bytes.len() {
        let tag = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let len = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        if tag == 14 {
            found = Some(bytes[offset + 8..offset + 8 + len].to_vec());
        }
        offset += 8 + len;
    }
    let payload = found.expect("phải có section 14");
    let text_len = u32::from_le_bytes(payload[0..4].try_into().unwrap()) as usize; // nếu put_string dùng u16, đổi tương ứng
    let json: serde_json::Value = serde_json::from_slice(&payload[4..4 + text_len]).unwrap();
    assert_eq!(json["sheet"]["game"], "fgc-2026");
}
```

Kiểm tra `put_string` trong `protocol.rs` dùng tiền tố độ dài `u16` hay `u32` và sửa hai dòng giải mã trong test cho đúng. Nếu chưa có helper `test_state()`, dùng cách dựng `MatchStateSync` mà test kích thước 1.000 bóng đang dùng.

Run: `cd server && cargo test snapshot_carries_score_sheet`
Expected: FAIL (minor version 5, không có section 14).

- [ ] **Step 2: Ghi section 14**

Trong `protocol.rs`: thêm `const SCORE_SHEET: u16 = 14;`, đổi `put_u16(&mut output, 5);` (minor) thành `put_u16(&mut output, 6);`, và sau section `SCORE`:

```rust
if state.score.sheet.is_some() || state.score.scoring_error.is_some() {
    let json = serde_json::json!({
        "sheet": state.score.sheet.as_deref(),
        "scoringError": state.score.scoring_error,
    })
    .to_string();
    section(&mut output, SCORE_SHEET, |bytes| put_long_string(bytes, &json));
}
```

Nếu `put_string` dùng tiền tố `u16` (giới hạn 65.535 byte), thêm `put_long_string` với tiền tố `u32` (bảng điểm FGC 2026 khoảng 3–6 KB nhưng đề khác có thể lớn hơn):

```rust
fn put_long_string(bytes: &mut Vec<u8>, value: &str) {
    put_u32(bytes, value.len() as u32);
    bytes.extend_from_slice(value.as_bytes());
}
```

Để tránh gửi lại 5 KB mỗi 50 ms, chỉ ghi section 14 khi `include_physics` (baseline/khung đầy đủ) **hoặc** bảng điểm đổi: thêm `pub score_sheet_revision: u64` vào `MatchStateSync`, tăng mỗi lần `rescore` thành công, và publisher nhớ revision đã gửi theo từng socket (tìm chỗ `include_physics` được quyết định trong publisher và áp dụng cùng cách).

Cập nhật `docs/match-state-protocol.md`: minor version `6`; danh sách section ghi đủ 1–14 theo `protocol.rs` (7 `SEMANTIC_EVENTS`, 8 `DRIVE`, 9 `SCORE`, 10 `PLAYER_PHYSICS`, 11 `TRANSFER_DEBUG`, 12 `BALL_DEBUG`, 13 `INPUT_ACKS`, 14 `SCORE_SHEET` = `u32` độ dài + JSON UTF-8 `{sheet, scoringError}`).

Run: `cd server && cargo test --release`
Expected: PASS.

- [ ] **Step 3: Giải mã TypeScript** — trong `match-protocol.ts`:

Thêm kiểu:

```ts
export type LineItem = { name: string; label: string | null; label_en: string | null; value: string };
export type ScoreSheetJson = {
	game: string;
	rules_version: string;
	field: LineItem[];
	sides: Record<string, { values: LineItem[]; score: number; raw_score: string; fouls_received: string }>;
	not_simulated: string[];
};
export type ScoreSheetSection = { sheet: ScoreSheetJson | null; scoringError: string | null };
```

Thêm trường `scoreSheet: ScoreSheetSection | null` vào kiểu snapshot và giá trị khởi tạo `scoreSheet: null`. Trong `switch (tag)`:

```ts
case 14: {
	const length = section.u32();
	const text = new TextDecoder().decode(new Uint8Array(view.buffer, view.byteOffset + section.offset, length));
	snapshot.scoreSheet = JSON.parse(text) as ScoreSheetSection;
	break;
}
```

(Dùng đúng tên phương thức đọc `u32` và thuộc tính `offset` của lớp `Reader` hiện có; kiểm tra bằng cách xem `case 9`.)

- [ ] **Step 4: Panel hiển thị** — `ScoreSheetPanel.svelte`:

```svelte
<script lang="ts">
	import type { ScoreSheetSection } from './match-protocol';

	let { data }: { data: ScoreSheetSection | null } = $props();
	const sides = $derived(data?.sheet ? Object.entries(data.sheet.sides) : []);
	const fieldItems = $derived(data?.sheet?.field.filter((item) => item.label) ?? []);
</script>

{#if data}
	<section class="pointer-events-auto max-w-sm rounded-lg bg-black/70 p-3 text-xs text-white" aria-label="Chi tiết điểm">
		<h2 class="mb-2 text-sm font-semibold">Chi tiết điểm</h2>
		{#if data.scoringError}
			<p class="mb-2 rounded bg-red-700/80 p-2">Lỗi tính điểm: {data.scoringError}</p>
		{/if}
		{#each sides as [id, side] (id)}
			<div class="mb-2">
				<div class="flex justify-between font-semibold" class:text-red-300={id === 'red'} class:text-sky-300={id === 'blue'}>
					<span>{id.toUpperCase()}</span><span>{side.score}</span>
				</div>
				{#each side.values.filter((item) => item.label) as item (item.name)}
					<div class="flex justify-between gap-4 opacity-90"><span>{item.label}</span><span>{item.value}</span></div>
				{/each}
			</div>
		{/each}
		{#each fieldItems as item (item.name)}
			<div class="flex justify-between gap-4 opacity-90"><span>{item.label}</span><span>{item.value}</span></div>
		{/each}
		{#if data.sheet?.not_simulated.length}
			<p class="mt-2 opacity-70">Chưa mô phỏng: {data.sheet.not_simulated.join(', ')}</p>
		{/if}
	</section>
{/if}
```

Trong `+page.svelte`: thêm `let scoreSheet = $state<ScoreSheetSection | null>(null);`, gán `scoreSheet = message.scoreSheet ?? scoreSheet;` ở chỗ đang gán `blueScore = message.score.blue;` (dòng ~1497), và đặt `<ScoreSheetPanel data={scoreSheet} />` trong lớp phủ HUD (cạnh `ScoreboardGraphic`).

- [ ] **Step 5: Kiểm tra web**

Run: `cd web && pnpm check && pnpm build`
Expected: 0 lỗi svelte-check; build thành công.

Chạy cả hệ thống, vào `/match/arena`, xác nhận panel "Chi tiết điểm" hiện hệ số leo, Suppression Unit × hệ số, và dòng "Chưa mô phỏng: field.ext_wildfire, robot.lifted_by_partner".

- [ ] **Step 6: Phá thử**

Đổi `SCORE_SHEET` thành `15` → test Rust đỏ. Hoàn tác.

- [ ] **Step 7: Commit**

```bash
git add server docs/match-state-protocol.md web/src/routes/\(game\)/match/test-match
git commit -m "feat(protocol): section 14 SCORE_SHEET (minor 6) + bảng Chi tiết điểm"
```

---

### Task 12: API — luật công khai và snapshot cho server

**Files:**
- Create: `api/src/routes/rules.ts`
- Modify: `api/src/index.ts` (gắn `app.route('/api/rules', rulesRoutes)`)
- Modify: `api/src/routes/game-packs.ts` (`runtime` trả `rules`, `observe`; kiểu `Manifest` có `rules`)

**Interfaces:**
- Consumes: binding `PACK_ASSETS` (thư mục `pkgs/games`).
- Produces:
  - `GET /api/rules` → `{ success: true, data: { games: [{ id, name, year, series, simulated }] } }` (đọc `index.json`)
  - `GET /api/rules/:id` → `text/yaml; charset=utf-8` nội dung `<id>/rules.yaml`; 404 nếu `id` không có trong `index.json`.
  - `GET /api/game-packs/:id/runtime` thêm `rules: Record<string, string>` (đề chính + mọi đề cha theo `extends`) và `observe: string | null`.

- [ ] **Step 1: Viết `rules.ts`**

```ts
import { Hono } from 'hono';
import { jsonError, jsonSuccess } from '../responses';
import type { Bindings } from '../types';

type GameIndex = { games: { id: string; name: string; year: number; series: string; simulated: boolean }[] };

const app = new Hono<{ Bindings: Bindings }>();

async function readAsset(c: { env: Bindings; req: { url: string } }, path: string) {
	const url = new URL(c.req.url);
	url.pathname = `/${path}`;
	url.search = '';
	return c.env.PACK_ASSETS.fetch(new Request(url));
}

async function readIndex(c: { env: Bindings; req: { url: string } }): Promise<GameIndex> {
	const response = await readAsset(c, 'index.json');
	if (!response.ok) throw new Error('index.json is unavailable');
	return response.json<GameIndex>();
}

app.get('/', async (c) => {
	try {
		return jsonSuccess(c, await readIndex(c));
	} catch (error) {
		return jsonError(c, 503, 'INTERNAL_ERROR', error instanceof Error ? error.message : 'Rules index is unavailable.');
	}
});

app.get('/:id', async (c) => {
	const id = c.req.param('id');
	try {
		const index = await readIndex(c);
		if (!index.games.some((game) => game.id === id)) return c.text('Không có đề này.', 404);
		const response = await readAsset(c, `${id}/rules.yaml`);
		if (!response.ok) return c.text('Đề chưa có rules.yaml.', 404);
		return new Response(response.body, {
			headers: { 'content-type': 'text/yaml; charset=utf-8', 'cache-control': 'public, max-age=300' }
		});
	} catch {
		return c.text('Dịch vụ luật đang lỗi.', 503);
	}
});

export default app;
```

`index.ts`: `import rulesRoutes from './routes/rules'` và `app.route('/api/rules', rulesRoutes)` cạnh các route khác.

- [ ] **Step 2: Snapshot runtime** — trong `game-packs.ts` (`app.get('/:id/runtime'...)`), sau khi có `manifest`:

```ts
const rules: Record<string, string> = {};
let observe: string | null = null;
if (manifest.rules?.scoring) {
	let currentId = PACK_ID;
	let currentYaml = await readPackText(c, manifest.rules.scoring);
	rules[currentId] = currentYaml;
	for (let depth = 0; depth < 4; depth += 1) {
		const parent = /^extends:\s*([\w-]+)\s*$/m.exec(currentYaml)?.[1];
		if (!parent || rules[parent]) break;
		const response = await getPackAsset(c, `../${parent}/rules.yaml`);
		if (!response.ok) break;
		currentYaml = await response.text();
		rules[parent] = currentYaml;
		currentId = parent;
	}
	if (manifest.rules.observe) observe = await readPackText(c, manifest.rules.observe);
}
```

và thêm `rules, observe` vào object trả về. Thêm `rules?: { scoring: string; observe?: string }` vào kiểu `Manifest`. Lưu ý `packPath` ghép tiền tố `/${PACK_ID}/`; với đề cha, đường dẫn `../<parent>/rules.yaml` phải được chuẩn hoá thành `/<parent>/rules.yaml` — nếu `getPackAsset` không chấp nhận `..`, gọi `c.env.PACK_ASSETS.fetch` trực tiếp như `readAsset` ở Step 1.

- [ ] **Step 3: Kiểm tra**

Run: `cd api && pnpm check && pnpm dev` (nền), rồi:

```bash
curl -s localhost:8787/api/rules | head -c 300
curl -s localhost:8787/api/rules/fgc-2025 | head -5
curl -s -o /dev/null -w '%{http_code}\n' localhost:8787/api/rules/khong-co
```

Expected: JSON có 2 đề; YAML bắt đầu bằng `schema: raso.rules/v1`; `404`.

Khởi động lại server Rust với `API_URL` local → log có `Loaded scoring rules game=fgc-2026`.

- [ ] **Step 4: Phá thử**

Xoá `fgc-2025` khỏi `index.json` → `curl /api/rules/fgc-2025` phải trả 404. Hoàn tác.

- [ ] **Step 5: Commit**

```bash
git add api
git commit -m "feat(api): /api/rules công khai + runtime snapshot kèm rules/observe"
```

---

### Task 13: Trang trọng tài `/referee/[gameId]`

**Files:**
- Modify: `web/vite.config.ts` (alias `$raso`, `server.fs.allow`)
- Create: `web/src/lib/raso.ts`
- Create: `web/src/routes/(public)/referee/+page.svelte`
- Create: `web/src/routes/(public)/referee/[gameId]/+page.ts`
- Create: `web/src/routes/(public)/referee/[gameId]/+page.svelte`
- Create: `web/src/routes/(public)/referee/[gameId]/InputControl.svelte`
- Create: `web/src/lib/raso.test.ts`

**Interfaces:**
- Consumes: WASM `RulesHandle` (Task 8), `GET /api/rules`, `GET /api/rules/:id` (Task 12).
- Produces:
  - `export async function loadRules(gameId: string, fetchFn?: typeof fetch): Promise<{ handle: RulesHandle; rules: RulesDescription }>` — tải YAML đề và mọi đề cha (đọc dòng `extends:`), khởi tạo WASM một lần.
  - `export type RulesDescription` (JSON của `describe()`): `{ game, name, version, sides: { ids: string[]; teams: number }, inputs: { field, side, robot: Record<string, InputDef> }, fouls: Record<string, { label: string }>, cards: Record<string, string> }`, `InputDef = { type: 'count' | 'choice' | 'bool'; max?: number | null; values: string[]; label: string; label_en?: string | null; help?: string | null }`.
  - `export function draftKey(gameId: string): string` → `raso-referee-draft:<gameId>`.

- [ ] **Step 1: Alias WASM** — trong `web/vite.config.ts`, thêm vào options của `sveltekit({...})`:

```ts
alias: { $raso: '../crates/raso-rules/pkg' },
```

và vào `defineConfig`:

```ts
server: {
	fs: { allow: ['..'] },
	proxy: { '/api': { target: 'http://localhost:8787', changeOrigin: true } }
},
```

Run: `cd web && pnpm check`
Expected: không lỗi phân giải `$raso` (sau khi đã chạy `./scripts/build-rules-wasm.sh`). Nếu `alias` không được chấp nhận trong options plugin, đặt nó vào `svelte.config.js` mới (`export default { kit: { alias: { $raso: '../crates/raso-rules/pkg' } } }`) và chuyển các option kit hiện có sang đó.

- [ ] **Step 2: Viết test thất bại cho `raso.ts`** — `web/src/lib/raso.test.ts`:

```ts
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { draftKey, loadRules } from './raso';

const games = join(__dirname, '../../../pkgs/games');
const fakeFetch: typeof fetch = async (input) => {
	const url = String(input);
	const id = url.split('/').pop()!;
	try {
		return new Response(readFileSync(join(games, id, 'rules.yaml'), 'utf8'), { status: 200 });
	} catch {
		return new Response('not found', { status: 404 });
	}
};

describe('loadRules', () => {
	it('tải đề và mô tả ô biên bản', async () => {
		const { rules } = await loadRules('fgc-2026', fakeFetch, readFileSync(join(__dirname, '../../../crates/raso-rules/pkg/raso_rules_bg.wasm')));
		expect(rules.game).toBe('fgc-2026');
		expect(Object.keys(rules.inputs.robot)).toEqual(['climb', 'lifted_by_partner']);
		expect(rules.sides.teams).toBe(3);
	});

	it('báo lỗi rõ khi đề không tồn tại', async () => {
		await expect(loadRules('khong-co', fakeFetch, readFileSync(join(__dirname, '../../../crates/raso-rules/pkg/raso_rules_bg.wasm')))).rejects.toThrow(/khong-co/);
	});

	it('khoá bản nháp theo đề', () => {
		expect(draftKey('fgc-2026')).toBe('raso-referee-draft:fgc-2026');
	});
});
```

Run: `cd web && pnpm vitest run src/lib/raso.test.ts`
Expected: FAIL (chưa có `raso.ts`).

- [ ] **Step 3: Viết `web/src/lib/raso.ts`**

```ts
import init, { initSync, RulesHandle } from '$raso/raso_rules.js';

export type InputDef = {
	type: 'count' | 'choice' | 'bool';
	max?: number | null;
	values: string[];
	label: string;
	label_en?: string | null;
	help?: string | null;
};

export type RulesDescription = {
	game: string;
	name: string;
	version: string;
	source: string;
	sides: { ids: string[]; teams: number };
	inputs: { field: Record<string, InputDef>; side: Record<string, InputDef>; robot: Record<string, InputDef> };
	fouls: Record<string, { label: string; label_en?: string | null }>;
	cards: Record<string, string>;
};

let ready: Promise<void> | null = null;

function ensureWasm(wasmBytes?: BufferSource): Promise<void> {
	if (wasmBytes) {
		initSync({ module: wasmBytes });
		return Promise.resolve();
	}
	ready ??= init().then(() => undefined);
	return ready;
}

async function fetchYaml(gameId: string, fetchFn: typeof fetch): Promise<string> {
	const response = await fetchFn(`/api/rules/${encodeURIComponent(gameId)}`);
	if (!response.ok) throw new Error(`Không tải được luật của đề '${gameId}' (${response.status})`);
	return response.text();
}

/** Tải luật + các đề cha (tối đa 4 tầng) rồi khởi tạo `RulesHandle`. */
export async function loadRules(
	gameId: string,
	fetchFn: typeof fetch = fetch,
	wasmBytes?: BufferSource
): Promise<{ handle: RulesHandle; rules: RulesDescription }> {
	await ensureWasm(wasmBytes);
	const yaml = await fetchYaml(gameId, fetchFn);
	const parents: Record<string, string> = {};
	let current = yaml;
	for (let depth = 0; depth < 4; depth += 1) {
		const parent = /^extends:\s*([\w-]+)\s*$/m.exec(current)?.[1];
		if (!parent || parents[parent]) break;
		current = await fetchYaml(parent, fetchFn);
		parents[parent] = current;
	}
	let handle: RulesHandle;
	try {
		handle = new RulesHandle(yaml, JSON.stringify(parents));
	} catch (error) {
		throw new Error(`Luật của đề '${gameId}' không hợp lệ: ${error instanceof Error ? error.message : String(error)}`);
	}
	return { handle, rules: JSON.parse(handle.describe()) as RulesDescription };
}

export function draftKey(gameId: string): string {
	return `raso-referee-draft:${gameId}`;
}
```

Ghi chú: trong vitest, alias `$raso` cần khai báo thêm ở `web/vitest.config.ts`: `resolve: { alias: { $raso: new URL('../crates/raso-rules/pkg', import.meta.url).pathname } }`.

Run: `cd web && pnpm vitest run src/lib/raso.test.ts`
Expected: PASS 3 test.

- [ ] **Step 4: Trang danh sách đề** — `web/src/routes/(public)/referee/+page.svelte`:

```svelte
<script lang="ts">
	import { onMount } from 'svelte';

	type Game = { id: string; name: string; year: number; series: string };
	let games = $state<Game[]>([]);
	let error = $state<string | null>(null);

	onMount(async () => {
		try {
			const response = await fetch('/api/rules');
			const body = await response.json();
			games = body.data.games;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	});
</script>

<svelte:head><title>RASO — Chấm điểm trọng tài</title></svelte:head>

<main class="mx-auto max-w-2xl p-4">
	<h1 class="mb-4 text-2xl font-bold">Chấm điểm trận thật</h1>
	{#if error}<p class="text-red-600">{error}</p>{/if}
	<ul class="grid gap-2">
		{#each games as game (game.id)}
			<li>
				<a class="block rounded-lg border p-3 hover:bg-black/5" href={`/referee/${game.id}`}>
					<span class="font-semibold">{game.series} {game.year}</span> — {game.name}
				</a>
			</li>
		{/each}
	</ul>
</main>
```

`web/src/routes/(public)/referee/[gameId]/+page.ts`:

```ts
export const ssr = false;

export function load({ params }) {
	return { gameId: params.gameId };
}
```

- [ ] **Step 5: Ô nhập** — `InputControl.svelte`:

```svelte
<script lang="ts">
	import type { InputDef } from '$lib/raso';

	let { def, value = $bindable(), error }: { def: InputDef; value: number | string | boolean; error?: string } = $props();
	const step = (delta: number) => {
		const next = Number(value) + delta;
		if (next >= 0 && (def.max == null || next <= def.max)) value = next;
	};
</script>

<label class="flex flex-col gap-1">
	<span class="text-sm font-medium">{def.label}</span>
	{#if def.type === 'count'}
		<div class="flex items-center gap-2">
			<button type="button" class="h-12 w-12 rounded-lg border text-2xl" onclick={() => step(-1)} aria-label={`Giảm ${def.label}`}>−</button>
			<input class="h-12 w-20 rounded-lg border text-center text-xl" type="number" min="0" max={def.max ?? undefined} bind:value />
			<button type="button" class="h-12 w-12 rounded-lg border text-2xl" onclick={() => step(1)} aria-label={`Tăng ${def.label}`}>+</button>
		</div>
	{:else if def.type === 'choice'}
		<div class="flex flex-wrap gap-1" role="radiogroup" aria-label={def.label}>
			{#each def.values as option (option)}
				<button
					type="button"
					role="radio"
					aria-checked={value === option}
					class="min-h-11 rounded-lg border px-3"
					class:bg-amber-400={value === option}
					onclick={() => (value = option)}>{option}</button>
			{/each}
		</div>
	{:else}
		<input class="h-6 w-6" type="checkbox" checked={Boolean(value)} onchange={(e) => (value = e.currentTarget.checked)} />
	{/if}
	{#if error}<span class="text-sm text-red-600">{error}</span>{/if}
</label>
```

Ghi chú: ô `count` dùng `<input type="number">`; khi người dùng gõ `2.5`, giá trị gửi đi là `2.5` và WASM trả `OBS_RANGE` — đúng hành vi mong muốn (Review Focus 3), lỗi hiện ngay dưới ô.

- [ ] **Step 6: Trang chấm điểm** — `web/src/routes/(public)/referee/[gameId]/+page.svelte`:

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import InputControl from './InputControl.svelte';
	import { draftKey, loadRules, type RulesDescription } from '$lib/raso';
	import type { RulesHandle } from '$raso/raso_rules.js';

	let { data } = $props();
	type Obs = {
		game: string;
		field: Record<string, number | string | boolean>;
		sides: Record<string, Record<string, unknown> & {
			robots: Record<string, number | string | boolean>[];
			fouls: Record<string, number>;
			teams: { id: string; card?: string | null }[];
		}>;
	};

	let handle: RulesHandle | null = null;
	let rules = $state<RulesDescription | null>(null);
	let obs = $state<Obs | null>(null);
	let loadError = $state<string | null>(null);

	onMount(async () => {
		try {
			const loaded = await loadRules(data.gameId);
			handle = loaded.handle;
			rules = loaded.rules;
			let draft: Obs | null = null;
			try {
				const saved = localStorage.getItem(draftKey(data.gameId));
				draft = saved ? (JSON.parse(saved) as Obs) : null;
			} catch {
				draft = null;
			}
			const blank = JSON.parse(handle.blankObservation()) as Obs;
			for (const side of Object.values(blank.sides)) {
				side.teams = Array.from({ length: loaded.rules.sides.teams }, () => ({ id: '', card: null }));
			}
			obs = draft?.game === blank.game ? draft : blank;
		} catch (e) {
			loadError = e instanceof Error ? e.message : String(e);
		}
	});

	const forScoring = (o: Obs) => ({
		...o,
		sides: Object.fromEntries(
			Object.entries(o.sides).map(([id, s]) => [id, { ...s, teams: s.teams.filter((t) => t.id.trim() !== '') }])
		)
	});

	const result = $derived.by(() => {
		if (!handle || !obs) return null;
		return JSON.parse(handle.score(JSON.stringify(forScoring(obs))));
	});

	$effect(() => {
		if (!obs) return;
		try {
			localStorage.setItem(draftKey(data.gameId), JSON.stringify(obs));
		} catch {
			/* trình duyệt chặn lưu trữ: bỏ qua, trang vẫn chạy */
		}
	});

	const errorFor = (path: string) =>
		result && !result.ok ? result.errors.find((e: { path: string }) => e.path === path)?.message : undefined;

	function reset() {
		if (!handle || !rules) return;
		const blank = JSON.parse(handle.blankObservation()) as Obs;
		for (const side of Object.values(blank.sides)) {
			side.teams = Array.from({ length: rules.sides.teams }, () => ({ id: '', card: null }));
		}
		obs = blank;
	}

	function download() {
		if (!obs || !result?.ok) return;
		const blob = new Blob([JSON.stringify({ observation: forScoring(obs), sheet: result.sheet }, null, 2)], { type: 'application/json' });
		const a = document.createElement('a');
		a.href = URL.createObjectURL(blob);
		a.download = `bien-ban-${data.gameId}-${new Date().toISOString().replace(/[:.]/g, '-')}.json`;
		a.click();
		URL.revokeObjectURL(a.href);
	}
</script>

<svelte:head><title>RASO — Trọng tài {data.gameId}</title></svelte:head>

<main class="mx-auto max-w-6xl p-4 print:p-0">
	{#if loadError}
		<p class="text-red-600">{loadError}</p>
	{:else if !rules || !obs}
		<p>Đang tải luật…</p>
	{:else}
		<header class="mb-4 flex flex-wrap items-center justify-between gap-2">
			<h1 class="text-xl font-bold">{rules.name} <span class="text-sm font-normal opacity-70">v{rules.version}</span></h1>
			<div class="flex gap-2 print:hidden">
				<button class="rounded-lg border px-3 py-2" onclick={reset}>Xoá biên bản</button>
				<button class="rounded-lg border px-3 py-2" onclick={() => window.print()}>In</button>
				<button class="rounded-lg border px-3 py-2" disabled={!result?.ok} onclick={download}>Tải JSON</button>
			</div>
		</header>

		<div class="grid gap-4 lg:grid-cols-[2fr_1fr]">
			<div class="grid gap-4">
				{#if Object.keys(rules.inputs.field).length}
					<section class="grid gap-3 rounded-lg border p-3 sm:grid-cols-2">
						<h2 class="font-semibold sm:col-span-2">Chung toàn sân</h2>
						{#each Object.entries(rules.inputs.field) as [name, def] (name)}
							<InputControl {def} bind:value={obs.field[name]} error={errorFor(`field.${name}`)} />
						{/each}
					</section>
				{/if}
				<div class="grid gap-4 md:grid-cols-2">
					{#each rules.sides.ids as side (side)}
						<section class="grid gap-3 rounded-lg border p-3" class:border-red-500={side === 'red'} class:border-sky-500={side === 'blue'}>
							<h2 class="font-semibold uppercase">{side}</h2>
							{#each Object.entries(rules.inputs.side) as [name, def] (name)}
								<InputControl {def} bind:value={obs.sides[side][name] as number} error={errorFor(`sides.${side}.${name}`)} />
							{/each}
							{#each obs.sides[side].robots as robot, i (i)}
								<fieldset class="grid gap-2 rounded border p-2">
									<legend class="px-1 text-sm">Robot {i + 1}</legend>
									<input class="rounded border px-2 py-1" placeholder="Mã đội" bind:value={obs.sides[side].teams[i].id} />
									{#each Object.entries(rules.inputs.robot) as [name, def] (name)}
										<InputControl {def} bind:value={robot[name]} error={errorFor(`sides.${side}.robots.${i}.${name}`)} />
									{/each}
									{#if Object.keys(rules.cards).length}
										<select class="rounded border px-2 py-1" bind:value={obs.sides[side].teams[i].card}>
											<option value={null}>Không thẻ</option>
											{#each Object.keys(rules.cards) as card (card)}<option value={card}>Thẻ {card}</option>{/each}
										</select>
									{/if}
								</fieldset>
							{/each}
							{#each Object.entries(rules.fouls) as [id, foul] (id)}
								<InputControl
									def={{ type: 'count', values: [], label: foul.label, max: 99 }}
									bind:value={obs.sides[side].fouls[id]}
									error={errorFor(`sides.${side}.fouls.${id}`)} />
							{/each}
						</section>
					{/each}
				</div>
			</div>

			<aside class="h-fit rounded-lg border p-3 lg:sticky lg:top-4" aria-live="polite">
				<h2 class="mb-2 font-semibold">Bảng điểm</h2>
				{#if result?.ok}
					{#each Object.entries(result.sheet.sides) as [side, s] (side)}
						<div class="mb-3">
							<div class="flex justify-between text-lg font-bold"><span class="uppercase">{side}</span><span>{s.score}</span></div>
							{#each s.values.filter((v: { label: string | null }) => v.label) as v (v.name)}
								<div class="flex justify-between text-sm"><span>{v.label}</span><span>{v.value}</span></div>
							{/each}
							<div class="flex justify-between text-sm"><span>Phạt nhận từ đối thủ</span><span>{s.fouls_received}</span></div>
							<div class="flex justify-between text-sm opacity-70"><span>Điểm thô</span><span>{s.raw_score}</span></div>
							{#each s.teams as t (t.id)}
								<div class="flex justify-between text-sm"><span>Đội {t.id}{t.card ? ` (thẻ ${t.card})` : ''}</span><span>{t.team_score}</span></div>
							{/each}
						</div>
					{/each}
					{#each result.sheet.field.filter((v: { label: string | null }) => v.label) as v (v.name)}
						<div class="flex justify-between text-sm"><span>{v.label}</span><span>{v.value}</span></div>
					{/each}
				{:else if result}
					<ul class="text-sm text-red-600">
						{#each result.errors as e (e.path + e.code)}<li>{e.path}: {e.message}</li>{/each}
					</ul>
				{/if}
			</aside>
		</div>
	{/if}
</main>
```

Ghi chú: các đội có ô "Mã đội" trống bị lọc trước khi tính (`forScoring`), nên trọng tài không bắt buộc nhập mã đội. Ô lỗi của `fouls` được tạo bằng `def` tạm (`max: 99`).

- [ ] **Step 7: Kiểm tra**

Run: `cd web && pnpm check && pnpm build && pnpm vitest run`
Expected: 0 lỗi; build thành công; mọi test vitest PASS.

Chạy api + web, mở `http://localhost:5173/referee` → thấy 2 đề. Vào `fgc-2026`, nhập lại ca `02-knockdown-coop4` (Extinguisher 150, đỏ 120 + 3× ZONE_3, xanh 100 + ZONE_3/ZONE_2/NONE) → bảng điểm hiện đỏ 398, xanh 320. Nhập `2.5` vào ô số bóng → lỗi hiện dưới ô. Tải lại trang → biên bản vẫn còn. Thử ở bề rộng 375 px: không cuộn ngang.

- [ ] **Step 8: Phá thử**

Trong `raso.ts`, bỏ vòng tải đề cha → chưa có đề mẫu nào dùng `extends` nên test không đỏ; thêm test vào `raso.test.ts`:

```ts
it('tải đề cha theo extends', async () => {
	const child = 'schema: raso.rules/v1\nextends: fgc-2026\ngame: fgc-2026-mini\nname: Mini\nversion: 0.1.0\nsource: test\noverride:\n  constants.coop_bonus: { 6: 50 }\n';
	const fetchWithChild: typeof fetch = async (input) =>
		String(input).endsWith('fgc-2026-mini') ? new Response(child) : fakeFetch(input);
	const { rules } = await loadRules('fgc-2026-mini', fetchWithChild, readFileSync(join(__dirname, '../../../crates/raso-rules/pkg/raso_rules_bg.wasm')));
	expect(rules.game).toBe('fgc-2026-mini');
	expect(rules.sides.teams).toBe(3);
});
```

rồi bỏ vòng tải đề cha → test mới đỏ. Hoàn tác.

- [ ] **Step 9: Commit**

```bash
git add web
git commit -m "feat(web): trang trọng tài /referee chấm điểm trận thật bằng WASM"
```

---

### Task 14: CI, tài liệu và kiểm tra cuối

**Files:**
- Modify: `.github/workflows/ci.yml` (job `rules`; job `web` build WASM trước)
- Modify: `CONTRIBUTING.md` (mục viết `rules.yaml`, `observe.yaml`, test)
- Modify: `docs/ROADMAP.md` (đánh dấu giai đoạn 1)
- Modify: `docs/LOCAL_SETUP.md` (bước build WASM, trang `/referee`)

- [ ] **Step 1: CI**

Thêm job:

```yaml
  rules:
    name: Rules engine
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: crates/raso-rules
      - run: cargo fmt --all --check
        working-directory: crates/raso-rules
      - run: cargo test --all-targets
        working-directory: crates/raso-rules
        env:
          RUSTFLAGS: -D warnings
      - run: cargo run --features cli --bin raso -- check $(ls -d ../../pkgs/games/*/ | while read d; do [ -f "$d/rules.yaml" ] && echo "$d"; done)
        working-directory: crates/raso-rules
      - run: cargo install wasm-pack --locked
      - run: ./scripts/build-rules-wasm.sh
      - uses: actions/upload-artifact@v4
        with:
          name: raso-rules-wasm
          path: crates/raso-rules/pkg
```

Trong job `web`: thêm `needs: rules`, bước `actions/download-artifact@v4` (`name: raso-rules-wasm`, `path: crates/raso-rules/pkg`) trước `pnpm install`, và bước `pnpm vitest run` sau `pnpm run check`. Job `rust` (server) cần thấy `../crates/raso-rules` — đã có vì checkout toàn repo; thêm `crates/raso-rules` vào `workspaces` của `rust-cache` nếu muốn cache.

- [ ] **Step 2: Tài liệu**

`CONTRIBUTING.md`: thay ghi chú "Lưu ý (10/2026)" bằng mục mới:

```markdown
## Viết luật tính điểm (`rules.yaml`)

1. Tạo `pkgs/games/<id>/rules.yaml` theo `docs/superpowers/specs/2026-10-15-raso-rules-design.md` (mục 4). Mẫu: `pkgs/games/fgc-2026/rules.yaml`, `pkgs/games/fgc-2025/rules.yaml`.
2. Đề dựa trên đề khác: `extends: <id-đề-cha>` + khối `override:` chỉ ghi phần thay đổi.
3. Viết ít nhất 6 biên bản mẫu trong `tests/scoring/*.yaml`, phủ mọi dòng trong bảng điểm của game manual và các ranh giới.
4. Chạy `cd crates/raso-rules && cargo run --features cli --bin raso -- check ../../pkgs/games/<id>`.
5. Thêm đề vào `pkgs/games/index.json` để hiện trên trang `/referee`.
6. Muốn chơi mô phỏng: thêm `observe.yaml` ánh xạ từng ô biên bản sang phép đo (`count_objects`, `brace_state`, `not_simulated`).
```

`docs/LOCAL_SETUP.md`: thêm bước `./scripts/build-rules-wasm.sh` trước `cd web && pnpm dev`; thêm dòng "Trang chấm điểm trận thật: http://localhost:5173/referee".

`docs/ROADMAP.md`: đánh dấu Giai đoạn 1 "Xong (10/2026)" và liệt kê những gì còn ngoài phạm vi (Extinguisher/Partner climb chưa mô phỏng; SeaHeal chờ BTC).

- [ ] **Step 3: Kiểm tra cuối toàn bộ**

Run:

```bash
(cd crates/raso-rules && cargo fmt --all --check && cargo test --all-targets && cargo run --features cli --bin raso -- check ../../pkgs/games/fgc-2026 ../../pkgs/games/fgc-2025)
./scripts/build-rules-wasm.sh
(cd server && cargo fmt --all --check && cargo test --release)
(cd api && pnpm check && pnpm build)
(cd web && pnpm check && pnpm vitest run && pnpm build)
```

Expected: tất cả thành công. Ghi lại số test của từng phần vào mô tả PR.

Tuỳ chọn khi máy 201 đang bật: chép repo sang 201 và chạy `cargo test --release benchmark_score_fgc_2026 -- --ignored --nocapture` trong `server/` để có số đo x86 cho spec mục 11.

- [ ] **Step 4: Commit**

```bash
git add .github CONTRIBUTING.md docs
git commit -m "ci(rules): job raso-rules + WASM; docs viết luật và trang trọng tài"
```

---

## Tự rà soát

- **Độ phủ spec:** §3.1 API → Task 3–5; §4.1–4.7 → Task 1–5; §5 → Task 4; §6.1/6.2 → Task 6/7; §7.1–7.3 → Task 9–10; §7.4 → Task 11; §7.5 → Task 10 (manifest) + Task 12 (API); §8 → Task 13; §9 → Task 6; §10 → Task 2–4; §11 → các task + Task 14; §12 → bản đồ file.
- **Giới hạn đã biết:** `opponent`/`<side_id>` chỉ đọc ô biên bản (spec đã cập nhật); robot chốt tại còi, sân/side đo sau 3,5 giây (spec §7.2).
