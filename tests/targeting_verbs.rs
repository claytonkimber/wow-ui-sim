//! Integration tests for the 6 targeting globals migrated off GLOBAL_NIL_STUBS.
//!
//! Covers: TargetUnit, FocusUnit, ClearTarget, TargetLastTarget,
//! TargetNearestFriend, TargetNearestEnemy.

use wow_ui_sim::lua_api::WowLuaEnv;

fn env() -> WowLuaEnv {
    WowLuaEnv::new().expect("Failed to create Lua environment")
}

// ── TargetUnit ────────────────────────────────────────────────────────────────

#[test]
fn target_unit_target_token_round_trips_via_unit_exists() {
    let env = env();
    let exists: bool = env
        .eval(
            r#"
            A_Admin.SetTarget("Boss", 63, 1, true)
            TargetUnit("target")
            return UnitExists("target")
            "#,
        )
        .unwrap();
    assert!(
        exists,
        "UnitExists('target') should be true after TargetUnit"
    );
}

#[test]
fn target_unit_player_token_sets_player_as_target() {
    let env = env();
    let is_player: bool = env
        .eval(
            r#"
            TargetUnit("player")
            return UnitExists("target")
            "#,
        )
        .unwrap();
    assert!(is_player);
}

#[test]
fn target_unit_unknown_token_is_silent_noop() {
    let env = env();
    let exists: bool = env
        .eval(
            r#"
            TargetUnit("nonexistent_unit_xyz")
            return UnitExists("target")
            "#,
        )
        .unwrap();
    assert!(!exists);
}

#[test]
fn target_unit_enemy1_falls_back_to_default_enemy() {
    let env = env();
    let name: String = env
        .eval(
            r#"
            TargetUnit("enemy1")
            return UnitName("target")
            "#,
        )
        .unwrap();
    assert_eq!(name, "Hogger");
}

#[test]
fn target_unit_does_not_crash_and_target_exists() {
    // Smoke test: TargetUnit against an existing target token doesn't crash
    // and the target is still set afterwards.
    let env = env();
    let exists: bool = env
        .eval(
            r#"
            A_Admin.SetTarget("Boss", 63, 1, true)
            TargetUnit("target")
            return UnitExists("target")
            "#,
        )
        .unwrap();
    assert!(exists);
}

// ── FocusUnit ─────────────────────────────────────────────────────────────────

#[test]
fn focus_unit_focus_token_round_trips_via_unit_exists() {
    let env = env();
    let exists: bool = env
        .eval(
            r#"
            A_Admin.SetFocus("Tank", 80, 2, false)
            FocusUnit("focus")
            return UnitExists("focus")
            "#,
        )
        .unwrap();
    assert!(exists, "UnitExists('focus') should be true after FocusUnit");
}

#[test]
fn focus_unit_player_token_sets_player_as_focus() {
    let env = env();
    let exists: bool = env
        .eval(
            r#"
            FocusUnit("player")
            return UnitExists("focus")
            "#,
        )
        .unwrap();
    assert!(exists);
}

#[test]
fn focus_unit_unknown_token_leaves_focus_empty() {
    let env = env();
    let exists: bool = env
        .eval(
            r#"
            FocusUnit("nonexistent_unit_xyz")
            return UnitExists("focus")
            "#,
        )
        .unwrap();
    assert!(!exists);
}

// ── ClearTarget ───────────────────────────────────────────────────────────────

#[test]
fn clear_target_removes_current_target() {
    let env = env();
    let exists: bool = env
        .eval(
            r#"
            A_Admin.SetTarget("Boss", 63, 1, true)
            ClearTarget()
            return UnitExists("target")
            "#,
        )
        .unwrap();
    assert!(!exists, "ClearTarget should remove target");
}

#[test]
fn clear_target_returns_true_when_existing_target_is_cleared() {
    let env = env();
    let cleared: bool = env
        .eval(
            r#"
            A_Admin.SetTarget("Boss", 63, 1, true)
            return ClearTarget()
            "#,
        )
        .unwrap();
    assert!(cleared);
}

#[test]
fn clear_target_returns_false_when_no_target_exists() {
    let env = env();
    let cleared: bool = env.eval("return ClearTarget()").unwrap();
    assert!(!cleared);
}

// ── TargetLastTarget ──────────────────────────────────────────────────────────

#[test]
fn target_last_target_swaps_after_clear_target() {
    let env = env();
    let (before, after): (bool, bool) = env
        .eval(
            r#"
            A_Admin.SetTarget("Boss", 63, 1, true)
            -- Current: Boss. No previous yet.
            ClearTarget()
            -- Current: none. Previous: Boss.
            local before = UnitExists("target")  -- false
            TargetLastTarget()
            -- Current: Boss again. Previous: none.
            local after = UnitExists("target")   -- true
            return before, after
            "#,
        )
        .unwrap();
    assert!(!before, "target should be gone after ClearTarget");
    assert!(after, "TargetLastTarget should restore previous target");
}

#[test]
fn target_last_target_no_previous_is_noop() {
    let env = env();
    let exists: bool = env
        .eval(
            r#"
            TargetLastTarget()
            return UnitExists("target")
            "#,
        )
        .unwrap();
    assert!(!exists);
}

#[test]
fn target_unit_snapshots_previous_target() {
    let env = env();
    let name_after_swap: String = env
        .eval(
            r#"
            A_Admin.SetTarget("Alpha", 63, 1, true)
            A_Admin.SetTarget("Beta", 60, 1, true)
            -- Now target both sequentially via TargetUnit
            TargetUnit("target")   -- current=Beta, previous=Beta (no-op replay)
            ClearTarget()          -- current=nil, previous=Beta
            TargetLastTarget()     -- current=Beta
            return UnitName("target")
            "#,
        )
        .unwrap();
    assert_eq!(name_after_swap, "Beta");
}

// ── TargetNearestFriend ───────────────────────────────────────────────────────

#[test]
fn target_nearest_friend_picks_first_party_member() {
    let env = env();
    let name: String = env
        .eval(
            r#"
            A_Admin.SetPartySize(2)
            A_Admin.SetPartyMember(1, "Healer", 5, 80)
            A_Admin.SetPartyMember(2, "Tank", 2, 80)
            TargetNearestFriend()
            return UnitName("target")
            "#,
        )
        .unwrap();
    assert_eq!(name, "Healer", "should pick first party member");
}

#[test]
fn target_nearest_friend_noop_when_no_party() {
    let env = env();
    let exists: bool = env
        .eval(
            r#"
            TargetNearestFriend()
            return UnitExists("target")
            "#,
        )
        .unwrap();
    assert!(!exists);
}

// ── TargetNearestEnemy ────────────────────────────────────────────────────────

#[test]
fn target_nearest_enemy_noop_on_empty_pool() {
    let env = env();
    let exists: bool = env
        .eval(
            r#"
            TargetNearestEnemy()
            return UnitExists("target")
            "#,
        )
        .unwrap();
    assert!(!exists, "empty enemy pool should be a no-op");
}

#[test]
fn target_nearest_enemy_picks_first_after_admin_seeding() {
    let env = env();
    let name: String = env
        .eval(
            r#"
            A_Admin.SetEnemyPool(
                {name="Kyveza", level=63, class_index=1},
                {name="Minion",  level=60, class_index=1}
            )
            TargetNearestEnemy()
            return UnitName("target")
            "#,
        )
        .unwrap();
    assert_eq!(name, "Kyveza", "should pick first enemy pool entry");
}

#[test]
fn target_nearest_enemy_enemy_pool_is_replaceable() {
    let env = env();
    let (first, second): (String, String) = env
        .eval(
            r#"
            A_Admin.SetEnemyPool({name="Alpha", level=63, class_index=1})
            TargetNearestEnemy()
            local first = UnitName("target")

            ClearTarget()
            A_Admin.SetEnemyPool({name="Beta", level=60, class_index=1})
            TargetNearestEnemy()
            local second = UnitName("target")
            return first, second
            "#,
        )
        .unwrap();
    assert_eq!(first, "Alpha");
    assert_eq!(second, "Beta");
}
