use super::RhaiEngine;

#[test]
fn inspects_functions_and_engine_calls() {
    let engine = RhaiEngine::new();
    let metadata = engine
        .inspect_source(
            "rules/example.rhai",
            "fn on_tick(state) { log_event(\"blue\", 1); }",
        )
        .unwrap();
    assert_eq!(metadata.functions[0].name, "on_tick");
    assert_eq!(metadata.functions[0].parameters, vec!["state"]);
    assert_eq!(metadata.engine_calls, vec!["log_event"]);
}

#[test]
fn rejects_invalid_rhai() {
    let engine = RhaiEngine::new();
    assert!(
        engine
            .inspect_source("rules/broken.rhai", "fn broken( {")
            .is_err()
    );
}
