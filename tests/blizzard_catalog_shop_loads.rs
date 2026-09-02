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

fn assert_catalog_shop_loaded(env: &WowLuaEnv) {
    let (loaded, finished): (bool, bool) = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_CatalogShop')")
        .expect("Catalog Shop load state query should succeed");
    assert!(loaded, "Blizzard_CatalogShop should load during startup");
    assert!(finished, "Blizzard_CatalogShop startup load should finish");
}

prefork_full_ui_case! {
fn blizzard_catalog_shop_loads_without_errors(env: &WowLuaEnv) {
    assert_catalog_shop_loaded(&env);

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_CatalogShop emitted Lua errors during startup:\n  {}",
        load_errors.join("\n  ")
    );

    let frame_present: bool = env
        .eval(
            "local frame = _G.CatalogShopFrame \
             or (__secureenv and rawget(__secureenv, 'CatalogShopFrame')); \
             return frame ~= nil",
        )
        .expect("frame query should succeed");
    assert!(
        frame_present,
        "CatalogShopFrame should be defined after load"
    );

    let mixin_present: bool = env
        .eval(
            "local mixin = _G.CatalogShopMixin \
             or (__secureenv and rawget(__secureenv, 'CatalogShopMixin')); \
             return type(mixin) == 'table'",
        )
        .expect("mixin query should succeed");
    assert!(
        mixin_present,
        "CatalogShopMixin should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn catalog_shop_show_and_hide_run_without_errors(env: &WowLuaEnv) {
    assert_catalog_shop_loaded(&env);

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    env.exec(
        "local frame = _G.CatalogShopFrame \
         or (__secureenv and rawget(__secureenv, 'CatalogShopFrame')); \
         frame:Show()",
    )
    .expect("CatalogShopFrame Show should run");

    let (view_present, provider_type): (bool, String) = env
        .eval(
            "local frame = _G.CatalogShopFrame \
             or (__secureenv and rawget(__secureenv, 'CatalogShopFrame')); \
             local scrollBox = frame.ProductContainerFrame.ProductsScrollBoxContainer.ScrollBox; \
             return scrollBox:GetView() ~= nil, type(scrollBox:GetDataProvider())",
        )
        .expect("Catalog Shop product ScrollBox should be queryable");
    assert!(view_present, "product ScrollBox should have a view");
    assert_eq!(
        provider_type, "table",
        "product ScrollBox should have a provider"
    );

    env.exec(
        "local frame = _G.CatalogShopFrame \
         or (__secureenv and rawget(__secureenv, 'CatalogShopFrame')); \
         frame:Hide()",
    )
    .expect("CatalogShopFrame Hide should run");

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
        "CatalogShopFrame Show/Hide emitted unexpected Lua errors:\n  {}",
        unexpected_errors.join("\n  ")
    );
}
}
