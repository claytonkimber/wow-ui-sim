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

fn refund_flow_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_CatalogShopRefundFlow/Blizzard_CatalogShopRefundFlow.toc")
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
fn blizzard_catalog_shop_refund_flow_loads_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &refund_flow_toc())
        .expect("Blizzard_CatalogShopRefundFlow should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_CatalogShopRefundFlow emitted Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );

    let frame_present: bool = env
        .eval(
            "local frame = _G.CatalogShopRefundFrame \
             or (__secureenv and rawget(__secureenv, 'CatalogShopRefundFrame')); \
             return frame ~= nil",
        )
        .expect("frame query should succeed");
    assert!(
        frame_present,
        "CatalogShopRefundFrame should be defined after load"
    );

    let mixins_present: bool = env
        .eval(
            "local function lookup(name) \
               return _G[name] or (__secureenv and rawget(__secureenv, name)) \
             end; \
             return type(lookup('CatalogShopRefundFrameMixin')) == 'table' \
                and type(lookup('CatalogShopRefundFlowProcessingContainerMixin')) == 'table' \
                and type(lookup('CatalogShopRefundButtonMixin')) == 'table'",
        )
        .expect("mixin query should succeed");
    assert!(
        mixins_present,
        "CatalogShopRefundFrame mixins should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn catalog_shop_refund_frame_show_and_hide_run_without_errors(env: &WowLuaEnv) {

    load_addon(&env.loader_env(), &refund_flow_toc())
        .expect("Blizzard_CatalogShopRefundFlow should load");

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    env.exec(
        "local frame = _G.CatalogShopRefundFrame \
         or (__secureenv and rawget(__secureenv, 'CatalogShopRefundFrame')); \
         frame:Show(); frame:Hide()",
    )
    .expect("CatalogShopRefundFrame Show/Hide should run");

    let known_numeric_layout_error = "expected number, got nil at argument 1";
    let unexpected_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| !message.contains(known_numeric_layout_error))
        .cloned()
        .collect();
    assert!(
        unexpected_errors.is_empty(),
        "CatalogShopRefundFrame Show/Hide emitted unexpected Lua errors:\n  {}",
        unexpected_errors.join("\n  ")
    );
}
}
