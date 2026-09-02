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
    env.state().borrow_mut().addon_base_paths = vec![blizzard_ui_dir()];

    wow_ui_sim::xml::register_intrinsic_templates();

    for (name, toc_path) in
        discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game)
    {
        load_addon(&env.loader_env(), &toc_path)
            .unwrap_or_else(|error| panic!("[load {name}] FAILED: {error}"));
    }

    env.apply_post_load_workarounds();
    fire_startup_events_for_screen(&env, ScreenKind::Game);
    env
}

prefork_full_ui_case! {
fn collections_journal_opened_to_heirlooms_closes_on_escape(env: &WowLuaEnv) {

    let (loaded, reason): (bool, Option<String>) = env
        .eval(
            r#"
            local loaded, reason = C_AddOns.LoadAddOn("Blizzard_Collections")
            return loaded, reason
            "#,
        )
        .expect("Blizzard_Collections load result should be readable");
    assert!(
        loaded,
        "Blizzard_Collections should load explicitly, reason: {}",
        reason.unwrap_or_else(|| "<nil>".to_string())
    );

    let missing_surface: String = env
        .eval(
            r#"
            local missing = {}
            local function requireSurface(available, name)
                if not available then
                    table.insert(missing, name)
                end
            end

            requireSurface(CollectionsJournal ~= nil, "CollectionsJournal")
            requireSurface(HeirloomsJournal ~= nil, "HeirloomsJournal")
            requireSurface(GameMenuFrame ~= nil, "GameMenuFrame")
            requireSurface(type(Menu) == "table", "Menu")
            requireSurface(type(Menu.GetManager) == "function", "Menu.GetManager")
            requireSurface(Menu.GetManager() ~= nil, "Menu manager")
            requireSurface(type(ToggleCollectionsJournal) == "function", "ToggleCollectionsJournal")
            requireSurface(COLLECTIONS_JOURNAL_TAB_INDEX_HEIRLOOMS == 4, "Heirlooms tab index")

            return table.concat(missing, ", ")
            "#,
        )
        .expect("real Collections and game-menu surface should be readable");
    assert!(
        missing_surface.is_empty(),
        "full game UI is missing required Blizzard surface: {missing_surface}"
    );

    env.exec("ToggleCollectionsJournal(COLLECTIONS_JOURNAL_TAB_INDEX_HEIRLOOMS)")
        .expect("real ToggleCollectionsJournal should open Heirlooms");

    let opened_to_heirlooms: bool = env
        .eval(
            r#"
            return CollectionsJournal:IsShown()
                and HeirloomsJournal:IsShown()
                and PanelTemplates_GetSelectedTab(CollectionsJournal) == COLLECTIONS_JOURNAL_TAB_INDEX_HEIRLOOMS
                and not GameMenuFrame:IsShown()
            "#,
        )
        .expect("opened Collections state should be readable");
    assert!(
        opened_to_heirlooms,
        "real CollectionsJournal should open to the visible Heirlooms panel while GameMenu stays hidden"
    );

    env.send_key_press("ESCAPE", None)
        .expect("ESCAPE dispatch should succeed");

    let closed_without_menu: (bool, bool) = env
        .eval(
            r#"
            return not CollectionsJournal:IsShown(),
                not GameMenuFrame:IsShown()
            "#,
        )
        .expect("closed Collections state should be readable");
    assert_eq!(
        closed_without_menu,
        (true, true),
        "ESCAPE should close the real CollectionsJournal without opening GameMenuFrame"
    );
}
}
