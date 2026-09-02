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
fn blizzard_combat_log_is_load_on_demand_startup_publisher() {
    let ui = blizzard_ui_dir();
    let addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    let combat_log_position = addons
        .iter()
        .position(|(name, _)| name == "Blizzard_CombatLog");
    let game_position = addons.iter().position(|(name, _)| name == "Blizzard_Game");

    assert!(
        combat_log_position.is_some(),
        "Blizzard_CombatLog remains `## LoadOnDemand: 1` but must be discovered to publish \
         CombatLog_LoadUI before Blizzard_Game startup events"
    );
    assert!(
        combat_log_position < game_position,
        "Blizzard_CombatLog must load before Blizzard_Game"
    );
}

prefork_full_ui_case! {
fn blizzard_combat_log_is_loaded_via_runtime_loadaddon(env: &WowLuaEnv) {

    let already_loaded: bool = env
        .eval(
            "return type(C_AddOns) == 'table' \
                and type(C_AddOns.IsAddOnLoaded) == 'function' \
                and (C_AddOns.IsAddOnLoaded('Blizzard_CombatLog') and true or false)",
        )
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        already_loaded,
        "Blizzard_CombatLog should be loaded by the time the Game-screen UI is up — \
         Blizzard_ChatFrameBase issues a runtime `LoadAddOn(\"Blizzard_CombatLog\")` \
         while wiring up COMBATLOG=ChatFrame2"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_loads_without_errors(env: &WowLuaEnv) {

    let combat_log_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_CombatLog")
                && !message.contains("Blizzard_CombatLogBase")
                && !message.contains("Blizzard_CombatLogProcessor")
        })
        .cloned()
        .collect();
    assert!(
        combat_log_errors.is_empty(),
        "Blizzard_CombatLog emitted Lua errors during load:\n  {}",
        combat_log_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_quick_button_frame_is_defined(env: &WowLuaEnv) {

    let frame_present: bool = env
        .eval(
            "return CombatLogQuickButtonFrame_Custom ~= nil \
                and CombatLogQuickButtonFrame_CustomProgressBar ~= nil \
                and CombatLogQuickButtonFrame_CustomAdditionalFilterButton ~= nil",
        )
        .expect("CombatLogQuickButtonFrame_Custom query should succeed");
    assert!(
        frame_present,
        "CombatLogQuickButtonFrame_Custom (parented to ChatFrame2) and its $parentProgressBar \
         + $parentAdditionalFilterButton children should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_globals_and_settings_are_defined(env: &WowLuaEnv) {

    let globals_present: bool = env
        .eval(
            "return type(COMBATLOG_FILTER_VERSION) == 'number' \
                and COMBATLOG == ChatFrame2 \
                and type(COMBATLOG_DEFAULT_SETTINGS) == 'table' \
                and type(COMBATLOG_EVENT_LIST) == 'table' \
                and type(Blizzard_CombatLog_Filters) == 'table' \
                and type(Blizzard_CombatLog_Filter_Defaults) == 'table' \
                and type(Blizzard_CombatLog_CurrentSettings) == 'table'",
        )
        .expect("COMBATLOG globals query should succeed");
    assert!(
        globals_present,
        "COMBATLOG_FILTER_VERSION/COMBATLOG/COMBATLOG_DEFAULT_SETTINGS/COMBATLOG_EVENT_LIST/\
         Blizzard_CombatLog_Filters/Filter_Defaults/CurrentSettings should all be populated"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_driver_mixin_is_defined(env: &WowLuaEnv) {

    let mixin_present: bool = env
        .eval(
            "return type(CombatLogDriverMixin) == 'table' \
                and type(CombatLogDriverMixin.OnLoad) == 'function' \
                and type(CombatLogDriverMixin.OnEvent) == 'function' \
                and type(CombatLogDriverMixin.OnCombatLogMessage) == 'function' \
                and type(CombatLogDriverMixin.OnCombatLogRefilterStarted) == 'function' \
                and type(CombatLogDriverMixin.OnCombatLogRefilterUpdate) == 'function' \
                and type(CombatLogDriverMixin.OnCombatLogRefilterFinished) == 'function' \
                and type(CombatLogDriverMixin.OnCombatLogMessageLimitChanged) == 'function' \
                and type(CombatLogDriverMixin.OnCombatLogEntriesCleared) == 'function'",
        )
        .expect("CombatLogDriverMixin query should succeed");
    assert!(
        mixin_present,
        "CombatLogDriverMixin and its OnLoad/OnEvent/OnCombatLog* methods should be defined \
         after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_helper_functions_are_defined(env: &WowLuaEnv) {

    let helpers_present: bool = env
        .eval(
            "return type(Blizzard_CombatLog_InitializeFilters) == 'function' \
                and type(Blizzard_CombatLog_GenerateFullEventList) == 'function' \
                and type(Blizzard_CombatLog_HasEvent) == 'function' \
                and type(Blizzard_CombatLog_EnableEvent) == 'function' \
                and type(Blizzard_CombatLog_DisableEvent) == 'function' \
                and type(Blizzard_CombatLog_QuickButton_OnClick) == 'function' \
                and type(Blizzard_CombatLog_Update_QuickButtons) == 'function' \
                and type(Blizzard_CombatLog_RefreshGlobalLinks) == 'function' \
                and type(Blizzard_CombatLog_CreateUnitMenu) == 'function' \
                and type(Blizzard_CombatLog_CreateFilterMenu) == 'function' \
                and type(Blizzard_CombatLog_CreateSpellMenu) == 'function' \
                and type(Blizzard_CombatLog_CreateActionMenu) == 'function' \
                and type(CreateCombatLogContextMenu) == 'function'",
        )
        .expect("Blizzard_CombatLog helper-function query should succeed");
    assert!(
        helpers_present,
        "Top-level Blizzard_CombatLog_* helpers (InitializeFilters/GenerateFullEventList/\
         HasEvent/EnableEvent/DisableEvent/QuickButton_OnClick/Update_QuickButtons/\
         RefreshGlobalLinks/CreateUnitMenu/CreateFilterMenu/CreateSpellMenu/CreateActionMenu) \
         and CreateCombatLogContextMenu should be defined after load"
    );
}
}
