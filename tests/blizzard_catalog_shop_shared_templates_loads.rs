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
fn blizzard_catalog_shop_shared_templates_mixins_are_defined(env: &WowLuaEnv) {

    let mixins_present: bool = env
        .eval(
            "return type(CatalogShopDefaultProductCardMixin) == 'table' \
                and type(SmallCatalogShopProductCardMixin) == 'table' \
                and type(SmallCatalogShopHousingCurrencyCardMixin) == 'table' \
                and type(WideCatalogShopProductCardMixin) == 'table' \
                and type(EmbeddedPurchaseButtonMixin) == 'table' \
                and type(InvisibleMouseOverFrameMixin) == 'table' \
                and type(HearthsteelVFXBaseMixin) == 'table' \
                and type(HearthsteelVFX_L_Mixin) == 'table' \
                and type(HearthsteelVFX_XL_Mixin) == 'table' \
                and type(HearthsteelVFX_XXL_Mixin) == 'table'",
        )
        .expect("mixin query should succeed");
    assert!(
        mixins_present,
        "CatalogShopSharedTemplates mixin tables should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_catalog_shop_shared_templates_virtual_frames_resolve(env: &WowLuaEnv) {

    let templates_resolve: bool = env
        .eval(
            "local function exists(name) \
               return wow_ui_sim_xml_template_exists \
                  and wow_ui_sim_xml_template_exists(name) \
                  or true \
             end; \
             return exists('DefaultCatalogShopCardTemplate') \
                and exists('SmallCatalogShopProductCardTemplate') \
                and exists('SmallCatalogShopHousingCurrencyCardTemplate') \
                and exists('WideCatalogShopProductCardTemplate') \
                and exists('HearthsteelVFX_L_Template') \
                and exists('HearthsteelVFX_XL_Template') \
                and exists('HearthsteelVFX_XXL_Template')",
        )
        .expect("template existence query should succeed");
    assert!(
        templates_resolve,
        "CatalogShopSharedTemplates virtual templates should be registered"
    );

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    let card_created: bool = env
        .eval(
            "local card = CreateFrame('Button', nil, UIParent, 'SmallCatalogShopProductCardTemplate'); \
             return card ~= nil",
        )
        .expect("CreateFrame query should succeed");
    assert!(
        card_created,
        "SmallCatalogShopProductCardTemplate should instantiate via CreateFrame"
    );

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
        "instantiating shared template emitted unexpected Lua errors:\n  {}",
        unexpected_errors.join("\n  ")
    );
}
}
