use super::{GamePackRuntimeSnapshot, PackLoader};
use std::collections::BTreeMap;

/// Đọc gói fgc-2026 từ đĩa thành snapshot giống API trả về.
fn fixture_snapshot() -> GamePackRuntimeSnapshot {
    let root = std::path::Path::new("../pkgs/games/fgc-2026");
    let read = |path: &str| std::fs::read_to_string(root.join(path)).unwrap();
    let manifest: super::GamePackManifest = serde_json::from_str(&read("manifest.json")).unwrap();
    let field_physics = serde_json::from_str(&read("field.physics.json")).unwrap();
    let field_semantics = serde_json::from_str(&read("field.semantics.json")).unwrap();
    let scripts = manifest
        .scripts
        .values()
        .map(|path| (path.clone(), read(path)))
        .collect::<BTreeMap<_, _>>();
    let robots = ["starter-bot"]
        .into_iter()
        .map(|id| {
            let physics =
                serde_json::from_str(&read(&format!("robots/{id}/bot.physics.json"))).unwrap();
            let semantics =
                serde_json::from_str(&read(&format!("robots/{id}/bot.semantics.json"))).unwrap();
            (
                id.to_string(),
                super::RobotRuntimeAssets { physics, semantics },
            )
        })
        .collect();
    let paths = manifest.rules.clone().expect("manifest có mục rules");
    let rules = BTreeMap::from([(manifest.id.clone(), read(&paths.scoring))]);
    let observe = paths.observe.as_deref().map(read);
    GamePackRuntimeSnapshot {
        manifest,
        field_physics,
        field_semantics,
        scripts,
        robots,
        rules,
        observe,
    }
}

#[test]
fn loads_manifest_and_all_rhai_rules() {
    let loader = PackLoader::new("0.1.0");
    let metadata = loader.load_runtime_snapshot(fixture_snapshot()).unwrap();
    assert_eq!(metadata.manifest.id, "fgc-2026");
    assert_eq!(metadata.scripts.len(), 2);
    assert_eq!(metadata.arena.object_count, 500);
    assert_eq!(metadata.arena.ball.diameter_m, 0.100);
    assert_eq!(metadata.arena.ball.mass_kg, 0.062);
    assert_eq!(metadata.arena.ball.inertia_factor, 0.4);
    assert_eq!(metadata.arena.ball.drag_coefficient, 0.47);
    assert_eq!(metadata.arena.floor.material, "low-pile carpet");
    assert!(metadata.arena.floor.rolling_resistance_mps2 > 0.0);
    assert!(metadata.arena.robot.intake_enabled);
    assert_eq!(metadata.arena.robot.mass_kg, 18.0);
    assert_eq!(metadata.arena.robot.width_m, 0.50);
    assert_eq!(metadata.arena.robot.height_m, 0.50);
    assert_eq!(metadata.arena.robot.length_m, 0.50);
    let starter_bot = metadata
        .default_robot
        .as_ref()
        .expect("default robot must load");
    assert!(starter_bot.colliders.len() >= 30);
    assert_eq!(starter_bot.zones.len(), 4);
    assert_eq!(starter_bot.climb_colliders.len(), 2);
    let climber = starter_bot
        .climber
        .as_ref()
        .expect("starter bot climber must load");
    assert_eq!(climber.wheel_parts, ["ClimbWheel1", "ClimbWheel2"]);
    assert!(climber.stall_torque_nm > 0.0);
    assert!(metadata.arena.ramp.enabled);
    assert!(metadata.field_definition.colliders.len() >= 70);
    let front_wall = metadata
        .field_definition
        .colliders
        .iter()
        .find(|collider| collider.id == "blueSUfront")
        .expect("authored planar wall must be loaded");
    assert!(front_wall.max[2] - front_wall.min[2] >= 0.05 - f32::EPSILON);
    assert!(
        front_wall
            .half_extents
            .iter()
            .any(|extent| *extent >= 0.025)
    );
    assert!(metadata.field_definition.anchors.contains_key("redSpawn1"));
    assert!(metadata.field_definition.anchors.contains_key("blueSpawn3"));
    assert!(
        metadata
            .field_definition
            .anchors
            .contains_key("EXTballspawn")
    );
    assert!(
        metadata
            .field_definition
            .triggers
            .iter()
            .any(|trigger| trigger.id == "EXTscore")
    );
    let red_su = metadata
        .field_definition
        .scoring_targets
        .iter()
        .find(|target| target.id == "red-suppression-unit")
        .expect("red SU scoring area must be loaded");
    assert!(
        red_su.min[1] < 1.0,
        "hopper area extends below the sensor plane"
    );
    assert!(red_su.max[1] > 1.9);
    assert!((metadata.field_definition.boundary.min[0] + 3.5).abs() < 0.01);
    assert!((metadata.field_definition.boundary.max[0] - 3.5).abs() < 0.01);
    assert!(metadata.field_definition.floor_height_m > 0.65);
    assert!(
        !metadata
            .scripts
            .iter()
            .any(|script| script.path.ends_with("scoring.rhai")
                || script.path.ends_with("penalties.rhai"))
    );
    assert!(
        metadata
            .scripts
            .iter()
            .flat_map(|script| script.functions.iter())
            .any(|function| function.name == "validate_build")
    );
}

#[test]
fn fgc_2026_pack_loads_rules_and_observe() {
    let pack = PackLoader::new("0.1.0")
        .load_pack("../pkgs/games/fgc-2026/manifest.json")
        .unwrap();
    let rules = pack.rules.as_ref().expect("gói fgc-2026 có rules.yaml");
    assert_eq!(rules.file.game, "fgc-2026");
    assert!(pack.observe.is_some());
    assert!(
        !pack
            .manifest
            .scripts
            .values()
            .any(|p| p.ends_with("scoring.rhai") || p.ends_with("penalties.rhai"))
    );
}

#[test]
fn invalid_rules_reject_the_pack() {
    let loader = PackLoader::new("0.1.0");
    let mut snapshot = fixture_snapshot();
    snapshot.rules.insert(
        "fgc-2026".into(),
        "schema: riso.rules/v1\ngame: fgc-2026".into(),
    );
    let error = loader.load_runtime_snapshot(snapshot).unwrap_err();
    assert!(error.to_string().contains("RULE_YAML"), "{error}");
}

#[test]
fn invalid_observe_rejects_the_pack() {
    let loader = PackLoader::new("0.1.0");
    let mut snapshot = fixture_snapshot();
    snapshot.observe = Some("side.bogus: { not_simulated: 0 }".into());
    let error = loader.load_runtime_snapshot(snapshot).unwrap_err();
    assert!(error.to_string().contains("side.bogus"), "{error}");
}

#[test]
fn snapshot_without_rules_yaml_is_rejected() {
    let loader = PackLoader::new("0.1.0");
    let mut snapshot = fixture_snapshot();
    snapshot.rules.clear();
    let error = loader.load_runtime_snapshot(snapshot).unwrap_err();
    assert!(error.to_string().contains("rules.yaml"), "{error}");
}
