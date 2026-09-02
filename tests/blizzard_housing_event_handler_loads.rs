#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn handler_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_HousingEventHandler")
}

fn handler_toc() -> PathBuf {
    handler_dir().join("Blizzard_HousingEventHandler.toc")
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

fn assert_module_methods(env: &WowLuaEnv, module: &str, methods: &[&str], rationale: &str) {
    for method in methods {
        let exists: bool = env
            .eval(&format!("return type({module}['{method}']) == 'function'"))
            .unwrap_or_else(|err| panic!("{module}.{method} existence query failed: {err}"));
        assert!(exists, "{module} must expose `.{method}()` — {rationale}");
    }
}

#[test]
fn blizzard_housing_event_handler_find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&handler_dir()).expect("Blizzard_HousingEventHandler TOC should resolve");
    assert_eq!(
        resolved,
        handler_toc(),
        "Blizzard_HousingEventHandler ships exactly one bare TOC — retail-only addon resolves \
         via `find_toc_file` fallthrough"
    );
}

#[test]
fn blizzard_housing_event_handler_toc_is_not_load_on_demand_with_zero_dependencies() {
    let toc = TocFile::from_file(&handler_toc()).expect("HousingEventHandler TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_HousingEventHandler omits `## LoadOnDemand` — auto-loads via Game-screen \
         discovery so its EventRegistry callbacks are wired up before any housing-related \
         server event fires; this addon is the dispatcher that LoadAddOn-pulls every other LoD \
         housing addon (Dashboard / Cornerstone / Charter / CreateNeighborhood / Controls / \
         HouseEditor / ModelPreview) on demand"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_HousingEventHandler omits `## Dependencies` — it references many globals \
         (HousingResultToErrorText, HousingTutorialUtil, HousingItemEarnedAlertFrameSystem, \
         EventRegistry, StaticPopup_Show / StaticPopupDialogs) but does NOT declare deps in \
         the TOC, relying instead on Blizzard's load-order convention that core addons \
         (Blizzard_FrameXML / Blizzard_SharedXML) provide these primitives before \
         Blizzard_HousingEventHandler runs"
    );
}

#[test]
fn blizzard_housing_event_handler_toc_is_retail_only_and_omits_allow_load() {
    let toc = TocFile::from_file(&handler_toc()).expect("HousingEventHandler TOC should parse");
    let toc_text = std::fs::read_to_string(handler_toc()).expect("TOC should read");
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Declares `## AllowLoadGameType: standard` — retail-only Midnight feature"
    );
    assert!(!toc.is_game_type_restricted());
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Omits `## AllowLoad:` — Game-screen-only auto-load semantics inferred from absence of \
         AllowLoad gating"
    );
    assert!(!toc_text.contains("## DefaultState:"));
    assert!(
        toc.saved_variables().is_empty(),
        "No `## SavedVariables*` — the EventHandler is a stateless dispatcher; all state lives \
         in the LoD addons it pulls"
    );
}

#[test]
fn blizzard_housing_event_handler_toc_lists_single_lua_file() {
    let toc = TocFile::from_file(&handler_toc()).expect("HousingEventHandler TOC should parse");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        files,
        vec!["Blizzard_HousingEventHandler.lua".to_string()],
        "TOC body lists exactly 1 source file — the handler is a single 445-line .lua module \
         with no XML companion (no frames, no templates, no panels — just a global \
         HousingFramesUtil utility namespace + HousingEventHandlerMixin singleton + 5 \
         StaticPopupDialogs entries)"
    );
}

#[test]
fn blizzard_housing_event_handler_directory_holds_two_entries() {
    let dir = handler_dir();
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_HousingEventHandler directory should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        2,
        "Directory ships exactly 2 entries (1 source file + 1 TOC). Got: {entries:?}"
    );
    assert!(entries.contains(&"Blizzard_HousingEventHandler.toc".to_string()));
    assert!(entries.contains(&"Blizzard_HousingEventHandler.lua".to_string()));
}

#[test]
fn blizzard_housing_event_handler_auto_discovered_on_game_screen_only() {
    let ui = blizzard_ui_dir();
    let game = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    let game_includes_handler = game
        .iter()
        .any(|(name, _)| name == "Blizzard_HousingEventHandler");
    assert!(
        game_includes_handler,
        "Must appear in Game-screen auto-discovery — without `## LoadOnDemand` and without \
         `## AllowLoad`, the addon loads on the Game screen so housing server events are \
         routed before any other housing UI activity"
    );

    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&ui, screen);
        let discovered = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_HousingEventHandler");
        assert!(
            !discovered,
            "Must NOT appear in {screen:?} auto-discovery — housing UI is Game-screen only; the \
             EventHandler stays out of the login / character-select / character-create passes \
             since those screens never raise housing server events"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_housing_event_handler_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let lua_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let related: Vec<&String> = lua_errors
        .iter()
        .filter(|e| {
            e.contains("Blizzard_HousingEventHandler/")
                || e.contains("Blizzard_HousingEventHandler\\")
                || e.contains("HousingFramesUtil")
                || e.contains("HousingEventHandlerMixin")
        })
        .collect();
    assert!(
        related.is_empty(),
        "Blizzard_HousingEventHandler emitted addon-specific Lua errors during Game-screen \
         auto-load:\n  {}",
        related
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_event_handler_is_addon_loaded_via_game_screen_pass(env: &WowLuaEnv) {
    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingEventHandler')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        loaded,
        "Game-screen auto-discovery must load Blizzard_HousingEventHandler — the addon's TOC \
         omits LoadOnDemand and AllowLoad, so the loader pulls it during the Game-screen \
         discovery pass alongside every other non-LoD Blizzard_* addon"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_event_handler_publishes_housing_frames_util_namespace(env: &WowLuaEnv) {
    let exists: bool = env
        .eval("return type(HousingFramesUtil) == 'table'")
        .expect("HousingFramesUtil global lookup should succeed");
    assert!(
        exists,
        "HousingFramesUtil must publish as a `_G` table — file scope assigns \
         `HousingFramesUtil = {{}}` at line 2 then attaches utility functions; this is the \
         primary public surface used by Blizzard_HousingControls / Blizzard_HouseEditor / \
         keybind shortcuts so they can dispatch housing API calls without taking a hard \
         dependency on any LoD housing addon"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_event_handler_frames_util_publishes_twenty_one_methods(env: &WowLuaEnv) {
    assert_module_methods(
        &env,
        "HousingFramesUtil",
        &[
            "IsHousingMarketShopAvailable",
            "LeaveHouseEditor",
            "ToggleHouseEditor",
            "IsHouseEditorModeAvailable",
            "ActivateHouseEditorMode",
            "SetExpertDecorSubmode",
            "ToggleHousingDashboard",
            "SetGridVisible",
            "RemoveSelectedDecor",
            "SetGridSnapEnabled",
            "SetFreePlaceEnabled",
            "ZoomLayoutCamera",
            "RotateBasicDecorSelection",
            "PreviewHousingCatalogEntryInfo",
            "PreviewHousingCatalogEntryVariantID",
            "PreviewHousingDecorID",
            "PreviewHousingItem",
            "HandleRotateBasicDecorSelectionLeftKeybind",
            "HandleRotateBasicDecorSelectionRightKeybind",
            "OpenFrameToTaskID",
            "ShowFixtureDecorActionConfirmation",
        ],
        "HousingFramesUtil aggregates 21 keybind / dashboard-toggle / preview / editor-mode / \
         decor-rotation / fixture-confirmation helpers used by the Controls strip + the \
         keybind shortcut layer; ToggleHousingDashboard / PreviewHousingCatalogEntryInfo / \
         OpenFrameToTaskID are the LoadAddOn-pulling entry points",
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_event_handler_publishes_event_handler_mixin(env: &WowLuaEnv) {
    let exists: bool = env
        .eval("return type(HousingEventHandlerMixin) == 'table'")
        .expect("HousingEventHandlerMixin global lookup should succeed");
    assert!(
        exists,
        "HousingEventHandlerMixin must publish as a `_G` table — file scope assigns \
         `HousingEventHandlerMixin = {{}}` at line 277 then attaches handler methods; the \
         singleton instance built via CreateAndInitFromMixin at line 431 is registered to 13 \
         EventRegistry frame events"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_event_handler_mixin_publishes_fourteen_methods(env: &WowLuaEnv) {
    assert_module_methods(
        &env,
        "HousingEventHandlerMixin",
        &[
            "Init",
            "OnPlotEntered",
            "OnEditorModeChanged",
            "OnEditorModeChangeFailed",
            "OpenCornerstone",
            "OpenCharter",
            "OpenCharterSignatureRequest",
            "OpenCreateGuildNeighborhoodUI",
            "OpenCreateCharterNeighborhoodUI",
            "OpenCreateCharterNeighborhoodConfirmation",
            "ShowPlayerEvictedConfirmation",
            "ShowOwnershipTransferRequestConfirmation",
            "ShowStairDirectionConfirmation",
            "ShowHousingItemAcquiredAlert",
        ],
        "HousingEventHandlerMixin owns 14 methods: Init (no-op), OnPlotEntered (loads \
         Controls), OnEditorModeChanged (loads HouseEditor + replays the event), \
         OnEditorModeChangeFailed (UIErrorsFrame), OpenCornerstone (loads Cornerstone + \
         branches Purchase/Visitor), OpenCharter / OpenCharterSignatureRequest (load Charter \
         + populate the dialog), OpenCreateGuildNeighborhoodUI / \
         OpenCreateCharterNeighborhoodUI / OpenCreateCharterNeighborhoodConfirmation (load \
         CreateNeighborhood), and 4 dialog-show wrappers \
         (ShowPlayerEvictedConfirmation / ShowOwnershipTransferRequestConfirmation / \
         ShowStairDirectionConfirmation / ShowHousingItemAcquiredAlert)",
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_event_handler_registers_thirteen_event_registry_callbacks(env: &WowLuaEnv) {
    for event in [
        "HOUSE_PLOT_ENTERED",
        "HOUSE_EDITOR_MODE_CHANGED",
        "HOUSE_EDITOR_MODE_CHANGE_FAILURE",
        "OPEN_PLOT_CORNERSTONE",
        "OPEN_NEIGHBORHOOD_CHARTER",
        "OPEN_NEIGHBORHOOD_CHARTER_SIGNATURE_REQUEST",
        "OPEN_CREATE_GUILD_NEIGHBORHOOD_UI",
        "OPEN_CREATE_CHARTER_NEIGHBORHOOD_UI",
        "OPEN_CHARTER_CONFIRMATION_UI",
        "SHOW_PLAYER_EVICTED_DIALOG",
        "SHOW_NEIGHBORHOOD_OWNERSHIP_TRANSFER_DIALOG",
        "SHOW_STAIR_DIRECTION_CONFIRMATION",
        "NEW_HOUSING_ITEM_ACQUIRED",
    ] {
        let has_registrants: bool = env
            .eval(&format!(
                "return EventRegistry:HasRegistrantsForEvent('{event}')"
            ))
            .expect("EventRegistry:HasRegistrantsForEvent query should succeed");
        assert!(
            has_registrants,
            "EventRegistry:HasRegistrantsForEvent('{event}') must return true after the addon's \
             bottom-of-file `EventRegistry:RegisterFrameEventAndCallback` block runs (lines \
             432-444 register 13 frame-event → HousingEventHandler-method bindings)."
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_event_handler_registers_five_static_popup_dialogs(env: &WowLuaEnv) {
    for dialog_key in [
        "CONFIRM_DESTROY_PREVIEW_DECOR",
        "HOUSING_FIXTURE_DECOR_ACTION_CONFIRM",
        "HOUSING_BULLETIN_EVICTED_CONFIRMATION",
        "HOUSING_TRANSFER_OWNER_REQUEST_CONFIRMATION",
        "HOUSING_LAYOUT_STAIRS_DIRECTION_CONFIRM",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(StaticPopupDialogs['{dialog_key}']) == 'table'"
            ))
            .expect("StaticPopupDialogs lookup should succeed");
        assert!(
            exists,
            "StaticPopupDialogs['{dialog_key}'] must register as a table after the addon \
             loads — the file declares 5 popup descriptors at file scope wrapping \
             housing-flow confirmations: preview-decor destruction, fixture-attached decor \
             store/detach, bulletin-board eviction, ownership-transfer accept/decline, and \
             stair direction up/down"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_event_handler_fixture_dialog_uses_select_callback_by_index(env: &WowLuaEnv) {
    let select_by_index: bool = env
        .eval(
            "return StaticPopupDialogs['HOUSING_FIXTURE_DECOR_ACTION_CONFIRM'] and \
                StaticPopupDialogs['HOUSING_FIXTURE_DECOR_ACTION_CONFIRM'].selectCallbackByIndex \
                == true",
        )
        .expect("selectCallbackByIndex query should succeed");
    assert!(
        select_by_index,
        "HOUSING_FIXTURE_DECOR_ACTION_CONFIRM must declare `selectCallbackByIndex = true` \
         because OnButton1 / OnButton2 / OnCloseClicked each invoke `data.callback(action)` \
         with distinct Enum.HousingFixtureDecorAction.{{Store, Detach}} arguments rather than \
         dispatching by named handler"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_event_handler_singleton_handler_is_local_not_global(env: &WowLuaEnv) {
    let exists_global: bool = env
        .eval("return _G['HousingEventHandler'] ~= nil")
        .expect("HousingEventHandler global lookup should succeed");
    assert!(
        !exists_global,
        "HousingEventHandler is declared `local HousingEventHandler = \
         CreateAndInitFromMixin(HousingEventHandlerMixin)` at line 431 — it must NOT publish \
         to `_G`; only its 13 EventRegistry frame-event registrations expose its behaviour to \
         the rest of the UI"
    );
}
}
