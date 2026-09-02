#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn commentator_toc() -> PathBuf {
    find_toc_file(&blizzard_ui_dir().join("Blizzard_Commentator"))
        .expect("Blizzard_Commentator TOC should resolve")
}

fn load_full_game_ui() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);
    env.set_screen_mode(ScreenKind::Game);

    {
        let mut state = env.state().borrow_mut();
        state.addon_base_paths = vec![blizzard_ui_dir()];
    }

    wow_ui_sim::xml::register_intrinsic_templates();

    let ui = blizzard_ui_dir();
    let addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("[load {name}] FAILED: {err}"));
    }

    env.apply_post_load_workarounds();
    fire_startup_events_for_screen(&env, ScreenKind::Game);
    env
}

#[test]
fn blizzard_commentator_toc_declares_load_on_demand_and_saved_variables() {
    let toc = TocFile::from_file(&commentator_toc()).expect("Commentator TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_Commentator declares `## LoadOnDemand: 1` so it should NOT auto-load on Game \
         screen — it is loaded by the spectator client when entering tournament arenas"
    );
    let saved_vars = toc.saved_variables();
    assert!(
        saved_vars.contains(&"CommentatorSave".to_string()),
        "Blizzard_Commentator declares `## SavedVariables: CommentatorSave` (for series scores \
         + commentator history persistence), got {saved_vars:?}"
    );
}

#[test]
fn blizzard_commentator_does_not_auto_discover_on_game_screen() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_Commentator");
    assert!(
        !in_game,
        "Blizzard_Commentator (LOD, no other addon depends on it) should not appear in \
         Game-screen auto-discovery"
    );
}

prefork_full_ui_case! {
fn blizzard_commentator_loads_explicitly_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &commentator_toc())
        .expect("Blizzard_Commentator should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_Commentator emitted Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_commentator_constants_and_globals_are_populated(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &commentator_toc()).expect("Blizzard_Commentator should load");

    let constants_present: bool = env
        .eval(
            "return COMMENTATOR_MAX_OFFENSIVE_SPELLS == 5 \
                and COMMENTATOR_MAX_DEFENSIVE_SPELLS == 5 \
                and COMMENTATOR_MAX_DEBUFF_SPELLS == 2 \
                and COMMENTATOR_SCORE_LIMIT == 3 \
                and math.abs(COMMENTATOR_INVERSE_SCALE - (768/1080)) < 1e-9 \
                and TOURNAMENT_OBSERVE_PLAYER_PRIMARY == 1 \
                and TOURNAMENT_OBSERVE_PLAYER_SECONDARY == 2 \
                and TOURNAMENT_OBSERVE_PLAYER_PRIMARY_SNAP == 3 \
                and type(SetFollowCameraTransitionPreset) == 'function' \
                and type(CycleFollowCameraTransitionPreset) == 'function' \
                and type(CommentatorSave) == 'table'",
        )
        .expect("constants query should succeed");
    assert!(
        constants_present,
        "Blizzard_Commentator's constants \
         (COMMENTATOR_MAX_OFFENSIVE_SPELLS=5, _DEFENSIVE=5, _DEBUFF=2, _SCORE_LIMIT=3, \
         INVERSE_SCALE=768/1080, TOURNAMENT_OBSERVE_PLAYER_PRIMARY=1/_SECONDARY=2/_PRIMARY_SNAP=3) \
         and globals (SetFollowCameraTransitionPreset, CycleFollowCameraTransitionPreset, \
         CommentatorSave) should all be defined after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_commentator_mixins_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &commentator_toc()).expect("Blizzard_Commentator should load");

    let mixins_present: bool = env
        .eval(
            "return type(CommentatorMixin) == 'table' \
                and type(FunctionThrottleMixin) == 'table' \
                and type(CommentatorUtil) == 'table' \
                and type(CommentatorCooldownMixin) == 'table' \
                and type(CommentatorModelSceneMixin) == 'table' \
                and type(CommentatorNamePlateMixin) == 'table' \
                and type(CommentatorScoreboardMixin) == 'table' \
                and type(CommentatorSpellBaseMixin) == 'table' \
                and type(CommentatorSpellMixin) == 'table' \
                and type(CommentatorDebuffMixin) == 'table' \
                and type(CommentatorSpellCache) == 'table' \
                and type(CommentatorSpellTrayMixin) == 'table' \
                and type(CommentatorUnitFrameMixin) == 'table' \
                and type(CommentatorVictoryFanfareFrameMixin) == 'table' \
                and type(CooldownCircleTrackerMixin) == 'table' \
                and type(FadeToBlackMixin) == 'table'",
        )
        .expect("mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_Commentator should define all 16 mixins/namespaces \
         (CommentatorMixin, FunctionThrottleMixin, CommentatorUtil, CommentatorCooldownMixin, \
         CommentatorModelSceneMixin, CommentatorNamePlateMixin, CommentatorScoreboardMixin, \
         CommentatorSpellBaseMixin, CommentatorSpellMixin, CommentatorDebuffMixin, \
         CommentatorSpellCache, CommentatorSpellTrayMixin, CommentatorUnitFrameMixin, \
         CommentatorVictoryFanfareFrameMixin, CooldownCircleTrackerMixin, FadeToBlackMixin)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_commentator_mixin_methods_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &commentator_toc()).expect("Blizzard_Commentator should load");

    let methods_present: bool = env
        .eval(
            "return type(CommentatorMixin.OnLoad) == 'function' \
                and type(CommentatorMixin.OnEvent) == 'function' \
                and type(CommentatorMixin.OnUpdate) == 'function' \
                and type(CommentatorMixin.ResetInternal) == 'function' \
                and type(CommentatorMixin.SwapTeams) == 'function' \
                and type(CommentatorMixin.ObservePlayer) == 'function' \
                and type(CommentatorMixin.SetDefaultSettings) == 'function' \
                and type(CommentatorMixin.SetDefaultBindings) == 'function' \
                and type(CommentatorMixin.SetDefaultCVars) == 'function' \
                and type(CommentatorMixin.Start) == 'function' \
                and type(CommentatorMixin.Shutdown) == 'function' \
                and type(CommentatorMixin.Reset) == 'function' \
                and type(CommentatorMixin.OnCommentatorEnterWorld) == 'function' \
                and type(CommentatorMixin.InitUnitFrames) == 'function' \
                and type(CommentatorMixin.ClearUnitFrames) == 'function' \
                and type(CommentatorMixin.ReinitializeUnitFrames) == 'function' \
                and type(CommentatorMixin.CheckObserverState) == 'function' \
                and type(CommentatorMixin.SetObserverState) == 'function' \
                and type(CommentatorMixin.OnObserverStateChanged) == 'function' \
                and type(CommentatorMixin.GetNameplateTemplate) == 'function' \
                and type(CommentatorMixin.JoinInstance) == 'function' \
                and type(CommentatorMixin.StopObserving) == 'function'",
        )
        .expect("CommentatorMixin method query should succeed");
    assert!(
        methods_present,
        "CommentatorMixin should expose its 22 documented methods after load \
         (OnLoad/OnEvent/OnUpdate/ResetInternal/SwapTeams/ObservePlayer/Set{{Default,Bindings,CVars}}/\
         Start/Shutdown/Reset/OnCommentatorEnterWorld/InitUnitFrames/ClearUnitFrames/\
         ReinitializeUnitFrames/CheckObserverState/SetObserverState/OnObserverStateChanged/\
         GetNameplateTemplate/JoinInstance/StopObserving)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_commentator_util_get_opposite_team_index_returns_other_side(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &commentator_toc()).expect("Blizzard_Commentator should load");

    let opposite_returns_expected_pair: bool = env
        .eval(
            "return CommentatorUtil.GetOppositeTeamIndex(1) == 2 \
                and CommentatorUtil.GetOppositeTeamIndex(2) == 1",
        )
        .expect("CommentatorUtil.GetOppositeTeamIndex query should succeed");
    assert!(
        opposite_returns_expected_pair,
        "CommentatorUtil.GetOppositeTeamIndex should map 1↔2 via `teamIndex == 1 and 2 or 1` \
         (the only two valid team indices in spectator mode)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_commentator_function_throttle_mixin_fires_after_threshold_elapses(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &commentator_toc()).expect("Blizzard_Commentator should load");

    let fires_when_threshold_reached: bool = env
        .eval(
            "local throttle = CreateFromMixins(FunctionThrottleMixin); \
             local fired = 0; \
             throttle:Init(0.5, function() fired = fired + 1; end); \
             throttle:Update(0.2); \
             local before_threshold = fired; \
             throttle:Update(0.4); \
             local after_threshold = fired; \
             return before_threshold == 0 and after_threshold == 1",
        )
        .expect("FunctionThrottleMixin query should succeed");
    assert!(
        fires_when_threshold_reached,
        "FunctionThrottleMixin should NOT fire while accumulated dt < threshold, and should fire \
         exactly once when dt accumulates past the threshold (0.2 + 0.4 = 0.6 > 0.5)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_commentator_toplevel_frames_exist_with_expected_parents(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &commentator_toc()).expect("Blizzard_Commentator should load");

    let frames_wired: bool = env
        .eval(
            "return type(_G.Commentator) == 'table' \
                and Commentator:GetParent() == UIParent \
                and type(_G.CommentatorFadeToBlackFrame) == 'table' \
                and CommentatorFadeToBlackFrame:GetParent() == WorldFrame \
                and type(_G.CommentatorVictoryFanfareFrame) == 'table' \
                and CommentatorVictoryFanfareFrame:GetParent() == UIParent",
        )
        .expect("frame parent query should succeed");
    assert!(
        frames_wired,
        "Blizzard_Commentator should define three toplevel frames after load: `Commentator` \
         (parent UIParent, mixin=CommentatorMixin, frameStrata=LOW), \
         `CommentatorFadeToBlackFrame` (parent WorldFrame, frameStrata=TOOLTIP, used for \
         pre-match fade in/out), and `CommentatorVictoryFanfareFrame` (parent UIParent, hidden, \
         used for end-of-match team announcement)"
    );
}
}
