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
fn blizzard_colors_appears_in_both_screens_discovery() {
    let ui = blizzard_ui_dir();

    let game_addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_Colors");
    assert!(
        in_game,
        "Blizzard_Colors (`## AllowLoad: Both`) should appear in Game-screen discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_Colors");
    assert!(
        in_login,
        "Blizzard_Colors (`## AllowLoad: Both`) should also appear in Login-screen discovery"
    );
}

prefork_full_ui_case! {
fn blizzard_colors_loads_without_errors(env: &WowLuaEnv) {

    let colors_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("Blizzard_Colors"))
        .cloned()
        .collect();
    assert!(
        colors_errors.is_empty(),
        "Blizzard_Colors emitted Lua errors during load:\n  {}",
        colors_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_colors_material_color_tables_are_populated(env: &WowLuaEnv) {

    let materials_present: bool = env
        .eval(
            "return type(MATERIAL_TEXT_COLOR_TABLE) == 'table' \
                and type(MATERIAL_TITLETEXT_COLOR_TABLE) == 'table' \
                and MATERIAL_TEXT_COLOR_TABLE.Default ~= nil \
                and MATERIAL_TEXT_COLOR_TABLE.Stone ~= nil \
                and MATERIAL_TEXT_COLOR_TABLE.Parchment ~= nil \
                and MATERIAL_TEXT_COLOR_TABLE.Marble ~= nil \
                and MATERIAL_TEXT_COLOR_TABLE.Silver ~= nil \
                and MATERIAL_TEXT_COLOR_TABLE.Bronze ~= nil \
                and MATERIAL_TEXT_COLOR_TABLE.ParchmentLarge ~= nil \
                and MATERIAL_TEXT_COLOR_TABLE.Progenitor ~= nil",
        )
        .expect("material table query should succeed");
    assert!(
        materials_present,
        "MATERIAL_TEXT_COLOR_TABLE should contain all 7 Shared keys plus the Mainline-added \
         Progenitor key (and MATERIAL_TITLETEXT_COLOR_TABLE should also be populated)"
    );

    let helper_works: bool = env
        .eval(
            "local text, title = GetMaterialTextColors('Stone'); \
             return type(text) == 'table' and type(title) == 'table' \
                and #text == 3 and #title == 3 \
                and type(text[1]) == 'number'",
        )
        .expect("GetMaterialTextColors query should succeed");
    assert!(
        helper_works,
        "GetMaterialTextColors('Stone') should return two RGB triplets"
    );
}
}

prefork_full_ui_case! {
fn blizzard_colors_quality_atlas_tables_are_populated(env: &WowLuaEnv) {

    let atlases_present: bool = env
        .eval(
            "return type(BAG_ITEM_QUALITY_COLORS) == 'table' \
                and type(AUCTION_HOUSE_ITEM_QUALITY_ICON_BORDER_ATLASES) == 'table' \
                and type(NEW_ITEM_ATLAS_BY_QUALITY) == 'table' \
                and type(LOOT_BORDER_BY_QUALITY) == 'table' \
                and type(DRESS_UP_FRAME_QUALITY_COLORS) == 'table' \
                and AUCTION_HOUSE_ITEM_QUALITY_ICON_BORDER_ATLASES[Enum.ItemQuality.Rare] \
                    == 'auctionhouse-itemicon-border-blue' \
                and DRESS_UP_FRAME_QUALITY_COLORS[Enum.ItemQuality.Epic] == 'purple'",
        )
        .expect("atlas table query should succeed");
    assert!(
        atlases_present,
        "Item-quality atlas/color lookup tables should be populated with the expected entries"
    );
}
}

prefork_full_ui_case! {
fn blizzard_colors_color_manager_helpers_are_defined(env: &WowLuaEnv) {

    let manager_present: bool = env
        .eval(
            "return type(ColorManager) == 'table' \
                and type(ColorManager.UpdateColorData) == 'function' \
                and type(ColorManager.GetColorDataForItemQuality) == 'function' \
                and type(ColorManager.GetDefaultColorDataForItemQuality) == 'function' \
                and type(ColorManager.GetColorDataForBagItemQuality) == 'function' \
                and type(ColorManager.GetAtlasDataForAuctionHouseItemQuality) == 'function' \
                and type(ColorManager.GetAtlasDataForProfessionsItemQuality) == 'function' \
                and type(ColorManager.GetAtlasDataForWardrobeSetItemQuality) == 'function' \
                and type(ColorManager.GetFormattedStringForItemQuality) == 'function'",
        )
        .expect("ColorManager query should succeed");
    assert!(
        manager_present,
        "ColorManager and its main accessor functions should be defined after load"
    );

    let formatted: String = env
        .eval(
            "return ColorManager.GetFormattedStringForItemQuality('Sword', Enum.ItemQuality.Epic)",
        )
        .expect("GetFormattedStringForItemQuality query should succeed");
    assert_eq!(
        formatted,
        format!("|cnIQ{}:Sword|r", 4),
        "GetFormattedStringForItemQuality should emit the |cnIQ<quality>:text|r token"
    );
}
}
