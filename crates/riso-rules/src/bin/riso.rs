//! riso check <thư-mục-đề>... | riso score <rules.yaml> <biên-bản.yaml> [--json] | riso rank <rules.yaml> <kết-quả.json>...
use riso_rules::pack::{DirResolver, check_pack};
use riso_rules::{MatchResult, Observation, load, rank, score};
use std::path::Path;
use std::process::ExitCode;

fn resolver_for(rules_path: &Path) -> DirResolver {
    let games_root = rules_path
        .parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new("."))
        .to_path_buf();
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
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
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
            let obs: Observation =
                serde_yaml_ng::from_str(&std::fs::read_to_string(&args[2]).expect("đọc biên bản"))
                    .expect("biên bản sai định dạng");
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
                .map(|p| {
                    serde_json::from_str(&std::fs::read_to_string(p).expect("đọc kết quả"))
                        .expect("kết quả sai định dạng")
                })
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
            eprintln!(
                "Cách dùng:\n  riso check <thư-mục-đề>...\n  riso score <rules.yaml> <biên-bản.yaml> [--json]\n  riso rank <rules.yaml> <kết-quả.json>..."
            );
            ExitCode::FAILURE
        }
    }
}
