//! Giao diện WebAssembly (wasm-bindgen) cho trình duyệt và Node.
use crate::{MapResolver, MatchResult, Observation, Rules, load, rank, score};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct RulesHandle {
    rules: Rules,
}

fn json_error(message: String) -> String {
    serde_json::json!({
        "ok": false,
        "errors": [{ "code": "OBS_JSON", "path": "", "message": message }]
    })
    .to_string()
}

#[wasm_bindgen]
impl RulesHandle {
    /// `parents_json`: `{"fgc-2026": "<yaml>"}` cho các đề cha (có thể `{}`).
    #[wasm_bindgen(constructor)]
    pub fn new(yaml: &str, parents_json: &str) -> Result<RulesHandle, JsError> {
        let parents: BTreeMap<String, String> =
            serde_json::from_str(parents_json).map_err(|e| JsError::new(&e.to_string()))?;
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
            Err(e) => return json_error(e.to_string()),
        };
        match score(&self.rules, &obs) {
            Ok(sheet) => serde_json::json!({ "ok": true, "sheet": sheet }).to_string(),
            Err(errors) => serde_json::json!({ "ok": false, "errors": errors }).to_string(),
        }
    }

    pub fn rank(&self, results_json: &str) -> String {
        let results: Vec<MatchResult> = match serde_json::from_str(results_json) {
            Ok(r) => r,
            Err(e) => return json_error(e.to_string()),
        };
        match rank(&self.rules, &results) {
            Ok(standings) => serde_json::json!({ "ok": true, "standings": standings }).to_string(),
            Err(errors) => serde_json::json!({ "ok": false, "errors": errors }).to_string(),
        }
    }
}
