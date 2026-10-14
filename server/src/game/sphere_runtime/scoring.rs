use super::*;

impl SphereRuntime {
    /// Số bóng đang hoạt động nằm trong vùng target. `released_by` lọc theo
    /// liên minh đã bắn bóng ra lần cuối. `None` nếu target không có/đang tắt.
    pub fn count_in_target(&self, target_id: &str, released_by: Option<&str>) -> Option<i64> {
        let target = self
            .scoring_targets
            .iter()
            .find(|target| target.id == target_id && target.enabled)?;
        Some(
            self.balls
                .iter()
                .filter(|ball| {
                    ball.active
                        && point_inside_aabb(ball.position, target.min, target.max)
                        && released_by
                            .is_none_or(|side| ball.last_outtake_alliance.as_deref() == Some(side))
                })
                .count() as i64,
        )
    }

    /// Nhãn trạng thái leo của từng robot, theo thứ tự id người chơi.
    pub fn robot_brace_labels(&self, triggers: &[FieldTrigger]) -> Vec<(String, String)> {
        let ground_offset_y = self
            .arena
            .as_ref()
            .map(|arena| -arena.robot.height_m * 0.5)
            .unwrap_or_default();
        self.players
            .values()
            .filter_map(|player| {
                let team = player.team_name.to_ascii_lowercase();
                let side = if team.starts_with("red") {
                    "red"
                } else if team.starts_with("blue") {
                    "blue"
                } else {
                    return None;
                };
                let state = brace::brace_state_for_player(
                    player,
                    self.robot_definition.as_ref(),
                    ground_offset_y,
                    &self.field_colliders,
                    triggers,
                );
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
