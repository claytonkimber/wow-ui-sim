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
fn blizzard_code_of_conduct_appears_in_game_discovery() {
    let ui = blizzard_ui_dir();
    let addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);

    let discovered = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CodeOfConduct");
    assert!(
        discovered,
        "Blizzard_CodeOfConduct should appear in Game-screen discovery (`## AllowLoad: game`, non-LOD)"
    );
}

prefork_full_ui_case! {
fn blizzard_code_of_conduct_registers_player_entering_world_callback(env: &WowLuaEnv) {

    let registered: bool = env
        .eval(
            "local funcs = EventRegistry.callbackTables \
                and EventRegistry.callbackTables[2] \
                and EventRegistry.callbackTables[2].PLAYER_ENTERING_WORLD; \
             if not funcs then return false end; \
             for _, fn in pairs(funcs) do \
                 local info = debug.getinfo(fn, 'S'); \
                 if info and info.short_src \
                    and info.short_src:find('Blizzard_CodeOfConduct') then \
                     return true \
                 end \
             end; \
             return false",
        )
        .expect("EventRegistry callback enumeration should succeed");
    assert!(
        registered,
        "Blizzard_CodeOfConduct should register a PLAYER_ENTERING_WORLD function callback \
         on EventRegistry (Function-table, callbackTables[2])"
    );
}
}

prefork_full_ui_case! {
fn blizzard_code_of_conduct_loads_without_errors(env: &WowLuaEnv) {

    let cof_addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("Blizzard_CodeOfConduct"))
        .cloned()
        .collect();
    assert!(
        cof_addon_errors.is_empty(),
        "Blizzard_CodeOfConduct emitted Lua errors during load:\n  {}",
        cof_addon_errors.join("\n  ")
    );
}
}
