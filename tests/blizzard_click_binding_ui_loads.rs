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

fn click_binding_ui_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_ClickBindingUI/Blizzard_ClickBindingUI.toc")
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

const KNOWN_GET_MODIFIED_CLICK_GAP: &str =
    "attempt to call global 'GetModifiedClick' (a nil value)";
const KNOWN_CLICK_BINDING_PROFILE_GAP: &str =
    "bad argument #1 to 'ipairs' (table expected, got nil)";

fn unexpected_errors(env: &WowLuaEnv) -> Vec<String> {
    env.state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            !message.contains(KNOWN_GET_MODIFIED_CLICK_GAP)
                && !message.contains(KNOWN_CLICK_BINDING_PROFILE_GAP)
        })
        .cloned()
        .collect()
}

prefork_full_ui_case! {
fn blizzard_click_binding_ui_loads_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &click_binding_ui_toc())
        .expect("Blizzard_ClickBindingUI should load via Rust loader");

    let load_errors = unexpected_errors(&env);
    assert!(
        load_errors.is_empty(),
        "Blizzard_ClickBindingUI emitted unexpected Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );

    let frame_present: bool = env
        .eval("return ClickBindingFrame ~= nil")
        .expect("ClickBindingFrame query should succeed");
    assert!(
        frame_present,
        "ClickBindingFrame should be defined after load"
    );

    let mixins_present: bool = env
        .eval(
            "return type(ClickBindingFrameMixin) == 'table' \
                and type(ClickBindingLineMixin) == 'table' \
                and type(ClickBindingHeaderMixin) == 'table' \
                and type(ClickBindingFramePortraitMixin) == 'table' \
                and type(ClickBindingTutorialMixin) == 'table' \
                and type(ClickableBindingsEnableMouseoverCastCheckboxMixin) == 'table'",
        )
        .expect("mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_ClickBindingUI mixin tables should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn click_binding_frame_show_and_hide_run_without_errors(env: &WowLuaEnv) {

    load_addon(&env.loader_env(), &click_binding_ui_toc())
        .expect("Blizzard_ClickBindingUI should load");

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    env.exec("ClickBindingFrame:Show(); ClickBindingFrame:Hide()")
        .expect("ClickBindingFrame Show/Hide should run");

    let leftover = unexpected_errors(&env);
    assert!(
        leftover.is_empty(),
        "ClickBindingFrame Show/Hide emitted unexpected Lua errors:\n  {}",
        leftover.join("\n  ")
    );
}
}
