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

prefork_full_ui_case! {
fn blizzard_class_menu_constants_and_helper_are_defined(env: &WowLuaEnv) {

    let constants_present: bool = env
        .eval(
            "return UNSPECIFIED_CLASS_FILTER == 0 \
                and UNSPECIFIED_SPEC_FILTER == 0",
        )
        .expect("constants query should succeed");
    assert!(
        constants_present,
        "UNSPECIFIED_CLASS_FILTER and UNSPECIFIED_SPEC_FILTER should both equal 0 after load"
    );

    let helper_present: bool = env
        .eval(
            "return type(ClassMenu) == 'table' \
                and type(ClassMenu.InitClassSpecDropdown) == 'function'",
        )
        .expect("ClassMenu query should succeed");
    assert!(
        helper_present,
        "ClassMenu.InitClassSpecDropdown should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn class_menu_init_class_spec_dropdown_runs_against_intrinsic_dropdown(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    env.exec(
        "local dropdown = CreateFrame('DropdownButton', nil, UIParent, 'WowStyle1FilterDropdownTemplate'); \
         local classFilter, specFilter = UNSPECIFIED_CLASS_FILTER, UNSPECIFIED_SPEC_FILTER; \
         ClassMenu.InitClassSpecDropdown(dropdown, \
             function() return classFilter end, \
             function() return specFilter end, \
             function(c, s) classFilter, specFilter = c, s end, \
             false, false); \
         _G.__test_class_menu_dropdown = dropdown",
    )
    .expect("ClassMenu.InitClassSpecDropdown should run on a fresh dropdown");

    let dropdown_attached: bool = env
        .eval("return _G.__test_class_menu_dropdown ~= nil")
        .expect("dropdown handle query should succeed");
    assert!(
        dropdown_attached,
        "InitClassSpecDropdown should leave the dropdown reachable after setup"
    );

    let unexpected_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        unexpected_errors.is_empty(),
        "ClassMenu.InitClassSpecDropdown emitted Lua errors:\n  {}",
        unexpected_errors.join("\n  ")
    );
}
}
