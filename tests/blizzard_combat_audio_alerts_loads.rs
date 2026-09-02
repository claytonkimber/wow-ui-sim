#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::discover_blizzard_addons_for_screen;
use wow_ui_sim::loader::load_addon;
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
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
fn blizzard_combat_audio_alerts_appears_in_game_discovery() {
    let ui = blizzard_ui_dir();
    let addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);

    let discovered = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CombatAudioAlerts");
    assert!(
        discovered,
        "Blizzard_CombatAudioAlerts should appear in Game-screen discovery \
         (`## AllowLoad: game`, non-LOD, `## DefaultState: enabled`)"
    );
}

prefork_full_ui_case! {
fn blizzard_combat_audio_alerts_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_CombatAudioAlerts")
                || message.contains("Blizzard_CombatAudioAlertManager")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_CombatAudioAlerts emitted Lua errors during load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_audio_alerts_manager_frame_and_mixin_are_defined(env: &WowLuaEnv) {

    let frame_present: bool = env
        .eval("return CombatAudioAlertManager ~= nil")
        .expect("CombatAudioAlertManager frame query should succeed");
    assert!(
        frame_present,
        "CombatAudioAlertManager (toplevel frame, mixin=CombatAudioAlertManagerMixin) \
         should be defined after load"
    );

    let mixin_present: bool = env
        .eval(
            "return type(CombatAudioAlertManagerMixin) == 'table' \
                and type(CombatAudioAlertManagerMixin.OnLoad) == 'function' \
                and type(CombatAudioAlertManagerMixin.OnEvent) == 'function' \
                and type(CombatAudioAlertManagerMixin.Init) == 'function' \
                and type(CombatAudioAlertManagerMixin.RefreshEvents) == 'function' \
                and type(CombatAudioAlertManagerMixin.RefreshThrottles) == 'function'",
        )
        .expect("mixin query should succeed");
    assert!(
        mixin_present,
        "CombatAudioAlertManagerMixin and its OnLoad/OnEvent/Init/Refresh* methods \
         should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_audio_alerts_setting_query_helpers_are_defined(env: &WowLuaEnv) {

    let helpers_present: bool = env
        .eval(
            "return type(CombatAudioAlertManagerMixin.IsSayCombatStartEnabled) == 'function' \
                and type(CombatAudioAlertManagerMixin.IsSayCombatEndEnabled) == 'function' \
                and type(CombatAudioAlertManagerMixin.IsSayPlayerHealthEnabled) == 'function' \
                and type(CombatAudioAlertManagerMixin.IsSayTargetNameEnabled) == 'function' \
                and type(CombatAudioAlertManagerMixin.IsSayTargetHealthEnabled) == 'function' \
                and type(CombatAudioAlertManagerMixin.IsSayPartyHealthEnabled) == 'function' \
                and type(CombatAudioAlertManagerMixin.IsInterruptCastEnabled) == 'function' \
                and type(CombatAudioAlertManagerMixin.IsSayYourDebuffsEnabled) == 'function'",
        )
        .expect("setting-query helper presence query should succeed");
    assert!(
        helpers_present,
        "CombatAudioAlertManagerMixin should expose the IsSay*/IsInterrupt* setting-query \
         helpers after load"
    );
}
}
