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
            panic!(
                "{}: {}",
                dir.display(),
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        });
        for case in &report.cases {
            assert!(
                case.failures.is_empty(),
                "{} / {}: {}",
                report.game,
                case.file,
                case.failures.join("; ")
            );
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

#[test]
fn fgc_2025_has_nine_cases() {
    let report = check_pack(&games_root().join("fgc-2025")).unwrap();
    assert_eq!(report.cases.len(), 9);
    assert!(report.ok());
}
