use super::*;

fn bootstrap_with_driver(participant: BootstrapParticipant) -> MatchBootstrap {
    MatchBootstrap {
        match_id: "match".into(),
        host_id: "host".into(),
        assigned_server_id: "server".into(),
        game_pack_id: "fgc-2026".into(),
        game_pack_version: "1.0.0".into(),
        match_seed: 1,
        starts_at: 1,
        duration_seconds: 150,
        max_players: 8,
        scenario_id: "default".into(),
        visibility: "private".into(),
        open_arena: false,
        participants: vec![participant],
    }
}

fn driver(
    robot_id: Option<&str>,
    robot_revision: Option<u64>,
    robot_data: Option<&str>,
) -> BootstrapParticipant {
    BootstrapParticipant {
        user_id: "driver".into(),
        role: "driver".into(),
        slot_id: "red-driver-1".into(),
        alliance: "red".into(),
        robot_id: robot_id.map(str::to_owned),
        robot_revision,
        robot_data: robot_data.map(str::to_owned),
    }
}

#[test]
fn bootstrap_requires_a_frozen_driver_robot_revision() {
    let missing_revision = bootstrap_with_driver(driver(
        Some("saved-robot"),
        None,
        Some(r#"{"driveType":"tank"}"#),
    ));
    assert!(validate_bootstrap_robot_revisions(&missing_revision).is_err());

    let mismatched_pack = bootstrap_with_driver(driver(
        Some("pack:starter-bot"),
        None,
        Some(r#"{"kind":"pack-robot","robotId":"other"}"#),
    ));
    assert!(validate_bootstrap_robot_revisions(&mismatched_pack).is_err());

    let frozen_custom = bootstrap_with_driver(driver(
        Some("saved-robot"),
        Some(42),
        Some(r#"{"driveType":"tank"}"#),
    ));
    assert!(validate_bootstrap_robot_revisions(&frozen_custom).is_ok());
}

#[test]
fn binary_state_has_versioned_header_and_compact_positions() {
    let state = MatchStateSync {
        tick: 7,
        game_pack_id: "pack".into(),
        game_pack_version: "1".into(),
        players: Vec::new(),
        object_id: "ball".into(),
        object_radius: 0.05,
        object_color: "test".to_string(),
        object_positions: ObjectPositionsSync {
            count: 0,
            active_mask: vec![],
            moving_mask: vec![],
            quantized_positions: vec![],
        },
        baseline_object_positions: ObjectPositionsSync {
            count: 0,
            active_mask: vec![],
            moving_mask: vec![],
            quantized_positions: vec![],
        },
        input_acknowledgements: Vec::new(),
        contacts: 0,
        match_clock: 1.0,
        match_duration_seconds: 150.0,
        pre_match_remaining_seconds: 0.0,
        match_running: true,
        simulation_clock: 1.0,
        physics_tick_ms: 2.0,
        physics_load_percent: 12.0,
        ticks_per_second: 60.0,
        target_ticks_per_second: 60.0,
        clock_drift_ms: 0.0,
        step_metrics: StepMetrics::default(),
        physics: PhysicsSync::from(
            &crate::game::pack_loader::PackLoader::new("0.1.0")
                .load_pack("../pkgs/games/fgc-2026/manifest.json")
                .unwrap()
                .arena,
        ),
        drive: DriveSync::default(),
        semantic_events: Vec::new(),
        score: ScoreState::default(),
        practice_running: false,
        transfer_debug: Vec::new(),
        ball_debug: Vec::new(),
        ball_contact_colliders: Vec::new(),
    };
    let encoded = encode_state(&state, ProcessMetrics::default(), false, false);
    assert_eq!(&encoded[..4], b"FGS1");
    assert_eq!(u16::from_le_bytes(encoded[4..6].try_into().unwrap()), 1);
    assert_eq!(u16::from_le_bytes(encoded[6..8].try_into().unwrap()), 5);
    assert_eq!(u16::from_le_bytes(encoded[8..10].try_into().unwrap()), 1);
    assert_eq!(
        u32::from_le_bytes(encoded[12..16].try_into().unwrap()) as usize,
        encoded.len() - 16
    );
    assert!(
        encoded.len() < 14_000,
        "snapshot was {} bytes",
        encoded.len()
    );
}

fn fgc_2026_pack() -> GamePackMetadata {
    crate::game::pack_loader::PackLoader::new("0.1.0")
        .load_pack("../pkgs/games/fgc-2026/manifest.json")
        .unwrap()
}

#[test]
fn rescore_turns_balls_in_target_into_score_sheet() {
    let pack = fgc_2026_pack();
    let mut runtime = RuntimeBackend::new("t".into(), &pack, 1);
    let RuntimeBackend::Sphere(sphere) = &mut runtime else {
        panic!("cần sphere backend")
    };
    let target = pack
        .field_definition
        .scoring_targets
        .iter()
        .find(|t| t.id == "red-suppression-unit")
        .unwrap()
        .clone();
    let centre = [
        (target.min[0] + target.max[0]) / 2.0,
        (target.min[1] + target.max[1]) / 2.0,
        (target.min[2] + target.max[2]) / 2.0,
    ];
    sphere.place_test_balls(&[centre, centre, centre], Some("red"));
    let (obs, sheet) = rescore(&mut runtime, &pack, None).expect("tính được");
    assert_eq!(
        obs.sides["red"].inputs["su_wildfire"],
        riso_rules::InputValue::Int(3)
    );
    assert_eq!(sheet.sides["red"].score, 3);
    let score = runtime.score_state();
    assert_eq!(score.red_score, 3);
    assert_eq!(score.blue_score, 0);
    assert_eq!(score.breakdown.get("red.su_points"), Some(&3));
    assert!(score.sheet.is_some());
    assert!(score.scoring_error.is_none());
}

#[test]
fn locked_robot_labels_are_used_for_final_score() {
    let pack = fgc_2026_pack();
    let mut runtime = RuntimeBackend::new("t".into(), &pack, 1);
    let locked = vec![("red".to_string(), "zone3".to_string())];
    let (obs, _) = rescore(&mut runtime, &pack, Some(&locked)).unwrap();
    assert_eq!(
        obs.sides["red"].robots[0]["climb"],
        riso_rules::InputValue::Text("ZONE_3".into())
    );
}

#[test]
fn completion_result_carries_observation_and_score_sheet() {
    let pack = fgc_2026_pack();
    let mut runtime = RuntimeBackend::new("t".into(), &pack, 1);
    let scored = rescore(&mut runtime, &pack, None);
    let result = completion_result(&runtime.score_state(), scored.as_ref(), &pack);
    assert_eq!(result["rulesVersion"], "1.0.0");
    assert_eq!(result["observation"]["game"], "fgc-2026");
    assert_eq!(result["scoreSheet"]["sides"]["red"]["score"], 0);
    assert_eq!(result["redScore"], 0);
    // API chặn payload hoàn tất > 16 KB (api/src/lib/validation.ts durablePayload).
    let size = serde_json::to_string(&result).unwrap().len();
    assert!(size <= 16_000, "result {size} bytes");
}

#[test]
fn score_state_from_sheet_reads_side_scores_and_labelled_values() {
    let rules = riso_rules::load(
        include_str!("../../../../pkgs/games/fgc-2026/rules.yaml"),
        &riso_rules::NoParents,
    )
    .unwrap();
    let mut obs = rules.blank_observation();
    obs.sides
        .get_mut("blue")
        .unwrap()
        .inputs
        .insert("su_wildfire".into(), riso_rules::InputValue::Int(7));
    let sheet = Arc::new(riso_rules::score(&rules, &obs).unwrap());
    let state = ScoreState::from_sheet(sheet);
    assert_eq!(state.blue_score, 7);
    assert_eq!(state.red_score, 0);
    assert_eq!(state.breakdown.get("blue.su_points"), Some(&7));
    // Hệ số leo 1 có nhãn → ceil(1) = 1.
    assert_eq!(state.breakdown.get("blue.climb_mult"), Some(&1));
}

fn runtime_with_three_red_balls(pack: &GamePackMetadata) -> RuntimeBackend {
    let mut runtime = RuntimeBackend::new("t".into(), pack, 1);
    let RuntimeBackend::Sphere(sphere) = &mut runtime else {
        panic!("cần sphere backend")
    };
    let target = pack
        .field_definition
        .scoring_targets
        .iter()
        .find(|t| t.id == "red-suppression-unit")
        .unwrap();
    let centre = [
        (target.min[0] + target.max[0]) / 2.0,
        (target.min[1] + target.max[1]) / 2.0,
        (target.min[2] + target.max[2]) / 2.0,
    ];
    sphere.place_test_balls(&[centre, centre, centre], Some("red"));
    runtime
}

#[test]
fn match_scoring_rescores_every_thirty_ticks() {
    let pack = fgc_2026_pack();
    let mut runtime = runtime_with_three_red_balls(&pack);
    let mut scoring = MatchScoring::default();
    scoring.periodic(&mut runtime, &pack, 29);
    assert_eq!(runtime.score_state().red_score, 0);
    scoring.periodic(&mut runtime, &pack, 30);
    assert_eq!(runtime.score_state().red_score, 3);
}

#[test]
fn final_score_uses_robot_states_locked_at_buzzer() {
    let pack = fgc_2026_pack();
    let mut runtime = RuntimeBackend::new("t".into(), &pack, 1);
    let mut scoring = MatchScoring::default();
    scoring.lock_at_buzzer(&runtime, &pack);
    assert_eq!(scoring.locked_robots, Some(Vec::new()));
    // Đã chốt thì lần gọi sau không ghi đè.
    scoring.locked_robots = Some(vec![("red".to_string(), "zone3".to_string())]);
    scoring.lock_at_buzzer(&runtime, &pack);
    let (obs, _) = scoring.finalize(&mut runtime, &pack).expect("tính được");
    assert_eq!(
        obs.sides["red"].robots[0]["climb"],
        riso_rules::InputValue::Text("ZONE_3".into())
    );
}
