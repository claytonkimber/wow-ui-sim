#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn buff_frame_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_BuffFrame/Blizzard_BuffFrame.toc")
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
fn blizzard_buff_frame_loads_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &buff_frame_toc())
        .expect("Blizzard_BuffFrame should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_BuffFrame emitted Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );

    let frames_present: bool = env
        .eval(
            "return BuffFrame ~= nil \
             and DebuffFrame ~= nil \
             and DeadlyDebuffFrame ~= nil \
             and type(BuffFrameMixin) == 'table' \
             and type(DebuffFrameMixin) == 'table' \
             and type(BaseAuraFrameMixin) == 'table' \
             and type(AuraFrameMixin) == 'table' \
             and type(AuraContainerMixin) == 'table' \
             and type(DeadlyDebuffFrameMixin) == 'table'",
        )
        .expect("frame/mixin query should succeed");
    assert!(
        frames_present,
        "BuffFrame, DebuffFrame, DeadlyDebuffFrame and aura mixins should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn buff_frame_update_runs_without_errors(env: &WowLuaEnv) {

    load_addon(&env.loader_env(), &buff_frame_toc()).expect("Blizzard_BuffFrame should load");

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    env.exec("BuffFrame:Update(); DebuffFrame:Update()")
        .expect("BuffFrame/DebuffFrame Update should run");

    let errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        errors.is_empty(),
        "BuffFrame/DebuffFrame Update emitted Lua errors:\n  {}",
        errors.join("\n  ")
    );
}
}
