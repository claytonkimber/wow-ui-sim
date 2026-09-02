#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::discover_blizzard_addons_for_screen;
use wow_ui_sim::loader::load_addon;
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::paths::default_blizzard_ui_addons_path;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;

fn blizzard_ui_dir() -> PathBuf {
    default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
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

prefork_full_ui_case! {
fn blizzard_class_trial_secure_frame_and_mixin_are_defined(env: &WowLuaEnv) {

    let frame_present: bool = env
        .eval(
            "local frame = _G.ClassTrialSecureFrame \
             or (__secureenv and rawget(__secureenv, 'ClassTrialSecureFrame')); \
             return frame ~= nil",
        )
        .expect("ClassTrialSecureFrame query should succeed");
    assert!(
        frame_present,
        "ClassTrialSecureFrame should be defined after load (looked up via _G with __secureenv fallback)"
    );

    let mixin_present: bool = env
        .eval(
            "local function lookup(name) \
               return _G[name] or (__secureenv and rawget(__secureenv, name)) \
             end; \
             return type(lookup('ClassTrialSecureFrameMixin')) == 'table'",
        )
        .expect("mixin query should succeed");
    assert!(
        mixin_present,
        "ClassTrialSecureFrameMixin should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn class_trial_outbound_helpers_are_defined(env: &WowLuaEnv) {

    let helpers_present: bool = env
        .eval(
            "local function lookup(name) \
               return _G[name] or (__secureenv and rawget(__secureenv, name)) \
             end; \
             local outbound = lookup('ClassTrialOutbound'); \
             return type(outbound) == 'table' \
                and type(outbound.ShowUpgradeConfirmation) == 'function' \
                and type(outbound.ShowUpgradeLogoutConfirmation) == 'function' \
                and type(outbound.ShowStoreServices) == 'function' \
                and type(outbound.SetClassTrialHasAvailableBoost) == 'function'",
        )
        .expect("ClassTrialOutbound query should succeed");
    assert!(
        helpers_present,
        "ClassTrialOutbound table and its four functions should be defined after load"
    );
}
}
