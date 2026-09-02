#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn settings_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_HousingHouseSettings")
}

fn settings_toc() -> PathBuf {
    settings_dir().join("Blizzard_HousingHouseSettings.toc")
}

fn load_housing_house_settings(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &settings_toc())
        .expect("Blizzard_HousingHouseSettings should load via explicit Rust loader call");
}

fn assert_mixin_methods(env: &WowLuaEnv, mixin: &str, methods: &[&str], rationale: &str) {
    for method in methods {
        let exists: bool = env
            .eval(&format!("return type({mixin}['{method}']) == 'function'"))
            .unwrap_or_else(|err| panic!("{mixin}.{method} existence query failed: {err}"));
        assert!(exists, "{mixin} must expose `:{method}()` — {rationale}");
    }
}

#[test]
fn blizzard_housing_house_settings_find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&settings_dir()).expect("Blizzard_HousingHouseSettings TOC should resolve");
    assert_eq!(
        resolved,
        settings_toc(),
        "Blizzard_HousingHouseSettings ships exactly one bare TOC — retail-only addon resolves \
         via `find_toc_file` fallthrough"
    );
}

#[test]
fn blizzard_housing_house_settings_toc_declares_lod_with_one_dependency() {
    let toc = TocFile::from_file(&settings_toc()).expect("HousingHouseSettings TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_HousingHouseSettings declares `## LoadOnDemand: 1` — pulled via explicit \
         LoadAddOn from Blizzard_HousingControls/Blizzard_HousingControlButton.lua:176 (the \
         in-house controls panel's House Settings button)"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_HousingTemplates".to_string()],
        "Single `## Dependencies:` entry: Blizzard_HousingTemplates (provides the housing \
         atlas/sound surface plus C_Housing/C_HousingNeighborhood query and mutator surface used \
         to read CURRENT_HOUSE_INFO_RECIEVED data and to push owner / access-flag / abandon-house \
         changes back to the server)"
    );
}

#[test]
fn blizzard_housing_house_settings_toc_is_retail_only_and_omits_allow_load() {
    let toc = TocFile::from_file(&settings_toc()).expect("HousingHouseSettings TOC should parse");
    let toc_text = std::fs::read_to_string(settings_toc()).expect("TOC should read");
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Declares `## AllowLoadGameType: standard` — retail-only Midnight feature"
    );
    assert!(!toc.is_game_type_restricted());
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Omits `## AllowLoad:` — LoadOnDemand precludes auto-discovery gating"
    );
    assert!(!toc_text.contains("## DefaultState:"));
    assert!(
        toc.saved_variables().is_empty(),
        "No `## SavedVariables*` — settings panel is server-driven via \
         PLAYER_CHARACTER_LIST_UPDATED + CURRENT_HOUSE_INFO_RECIEVED events plus the C_Housing \
         API surface; no client-side persistence"
    );
}

#[test]
fn blizzard_housing_house_settings_toc_lists_three_files_in_order() {
    let toc = TocFile::from_file(&settings_toc()).expect("HousingHouseSettings TOC should parse");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        files,
        vec![
            "Blizzard_HousingHouseSettings.lua".to_string(),
            "Blizzard_HousingHouseSettings.xml".to_string(),
            "Blizzard_HousingHouseSettingsRegistration.lua".to_string(),
        ],
        "TOC body lists exactly 3 source files in this exact order — the .lua defines the 3 \
         mixins consumed by the .xml's mixin= attributes, then the Registration tail loads LAST \
         so the HousingHouseSettingsFrame `_G` reference is published when RegisterUIPanel runs"
    );
}

#[test]
fn blizzard_housing_house_settings_directory_holds_four_entries() {
    let dir = settings_dir();
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_HousingHouseSettings directory should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        4,
        "Directory ships exactly 4 entries (3 source + 1 TOC, no flavor subdirectory). Got: \
         {entries:?}"
    );
    assert!(entries.contains(&"Blizzard_HousingHouseSettings.toc".to_string()));
    assert!(entries.contains(&"Blizzard_HousingHouseSettings.lua".to_string()));
    assert!(entries.contains(&"Blizzard_HousingHouseSettings.xml".to_string()));
    assert!(entries.contains(&"Blizzard_HousingHouseSettingsRegistration.lua".to_string()));
}

#[test]
fn blizzard_housing_house_settings_excluded_from_all_screen_auto_discovery() {
    let ui = blizzard_ui_dir();
    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
        ScreenKind::Game,
    ] {
        let addons = discover_blizzard_addons_for_screen(&ui, screen);
        let discovered = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_HousingHouseSettings");
        assert!(
            !discovered,
            "Must NOT appear in {screen:?} auto-discovery — `## LoadOnDemand: 1` excludes the \
             addon from every ScreenKind pass; it is only ever pulled via explicit LoadAddOn \
             from the in-house Controls panel's House Settings button"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_housing_house_settings(env);

    let lua_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let related: Vec<&String> = lua_errors
        .iter()
        .filter(|e| {
            e.contains("Blizzard_HousingHouseSettings/")
                || e.contains("Blizzard_HousingHouseSettings\\")
                || e.contains("HousingHouseSettingsFrameMixin")
                || e.contains("HouseSettingsAccessOptionsMixin")
                || e.contains("AbandonHouseConfirmationDialogMixin")
        })
        .collect();
    assert!(
        related.is_empty(),
        "Loading Blizzard_HousingHouseSettings must not emit any addon-specific Lua errors. Got \
         {} errors: {:?}",
        related.len(),
        related
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_is_addon_loaded_via_explicit_lod_call(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingHouseSettings')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_HousingHouseSettings') must return true after the \
         explicit LoD load — proves the loader registered the addon name"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_template_dependency_loads_via_game_screen_pass(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    let templates_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingTemplates')")
        .expect("HousingTemplates IsAddOnLoaded query should succeed");
    assert!(
        templates_loaded,
        "Blizzard_HousingTemplates (the only declared dep) must auto-load via the Game-screen \
         discovery pass before the explicit House Settings LoD load runs — it is NOT \
         LoadOnDemand so the auto-discovery sweep includes it"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_publishes_main_frame_global(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    let frame_type: String = env
        .eval("return type(HousingHouseSettingsFrame)")
        .expect("HousingHouseSettingsFrame type query should succeed");
    assert_eq!(
        frame_type, "table",
        "HousingHouseSettingsFrame must publish as a `_G` frame — the XML declares the named \
         non-virtual frame at line 133 (mixin HousingHouseSettingsFrameMixin, toplevel, \
         parent=UIParent, movable, enableMouse, hidden)"
    );
    let frame_name: String = env
        .eval("return HousingHouseSettingsFrame:GetName()")
        .expect("HousingHouseSettingsFrame:GetName query should succeed");
    assert_eq!(frame_name, "HousingHouseSettingsFrame");
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_publishes_abandon_dialog_global(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    let dialog_type: String = env
        .eval("return type(AbandonHouseConfirmationDialog)")
        .expect("AbandonHouseConfirmationDialog type query should succeed");
    assert_eq!(
        dialog_type, "table",
        "AbandonHouseConfirmationDialog must publish as a `_G` frame — the XML declares the \
         named non-virtual TOPLEVEL DIALOG-strata companion frame at line 61 inheriting \
         TranslucentFrameTemplate, hidden by default, with mixin AbandonHouseConfirmationDialogMixin"
    );
    let dialog_name: String = env
        .eval("return AbandonHouseConfirmationDialog:GetName()")
        .expect("AbandonHouseConfirmationDialog:GetName query should succeed");
    assert_eq!(dialog_name, "AbandonHouseConfirmationDialog");
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_virtual_templates_stay_nil_in_globals(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    for template in [
        "HouseSettingsAccessOptionsTemplate",
        "HouseSettingsAccessButtonTemplate",
    ] {
        let value_type: String = env
            .eval(&format!("return type(_G['{template}'])"))
            .expect("template _G lookup should succeed");
        assert_eq!(
            value_type, "nil",
            "{template} is `virtual=\"true\"` so it must NOT publish to `_G` — proves the loader \
             honors the virtual flag"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_frame_mixin_publishes_thirteen_methods(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    assert_mixin_methods(
        &env,
        "HousingHouseSettingsFrameMixin",
        &[
            "OnLoad",
            "SetHouseInfo",
            "OnEvent",
            "OnOwnerSelected",
            "SetupOwnerDropdown",
            "GetSelectedOwnerText",
            "OnShow",
            "OnHide",
            "OnAccessChanged",
            "OnIgnoreListClicked",
            "OnAbandonHouseClicked",
            "OnSaveClicked",
        ],
        "HousingHouseSettingsFrameMixin owns 12 methods on the umbrella settings panel: OnLoad / \
         OnEvent / OnShow / OnHide lifecycle, SetHouseInfo (populate from \
         CURRENT_HOUSE_INFO_RECIEVED payload), OnOwnerSelected / SetupOwnerDropdown / \
         GetSelectedOwnerText (owner-character dropdown driven by PLAYER_CHARACTER_LIST_UPDATED), \
         OnAccessChanged (refresh save-button enabled state), OnIgnoreListClicked / \
         OnAbandonHouseClicked / OnSaveClicked (the 3 footer buttons routing to the ignore-list \
         panel, the abandon-house confirmation dialog, and the C_Housing.SaveHouseSettings RPC)",
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_access_options_mixin_publishes_seven_methods(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    assert_mixin_methods(
        &env,
        "HouseSettingsAccessOptionsMixin",
        &[
            "SetSelectedSettings",
            "DisableSettings",
            "SetupAccessTypeDropdown",
            "OnAccessTypeSelected",
            "SetupOptions",
            "SetOptionSelectedCallback",
            "OptionSelected",
        ],
        "HouseSettingsAccessOptionsMixin owns 7 methods on the reusable access-options template \
         instantiated twice in the main frame (parentKey=PlotAccess + parentKey=HouseAccess): \
         SetSelectedSettings (mark currently-checked flags), DisableSettings, \
         SetupAccessTypeDropdown / OnAccessTypeSelected (Public/Friends-only/Private dropdown), \
         SetupOptions (called per group with HouseAccessFlags or PlotAccessFlags), \
         SetOptionSelectedCallback / OptionSelected (notify the parent frame so it can re-enable \
         the Save button)",
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_abandon_dialog_mixin_publishes_three_methods(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    assert_mixin_methods(
        &env,
        "AbandonHouseConfirmationDialogMixin",
        &["OnLoad", "SetHouseInfo", "OnConfirmClicked"],
        "AbandonHouseConfirmationDialogMixin owns 3 methods on the standalone DIALOG-strata \
         confirmation popup: OnLoad (wire the ConfirmButton/CancelButton OnClicks), SetHouseInfo \
         (populate the refund MoneyFrame from C_Housing refund estimate before showing), \
         OnConfirmClicked (call C_Housing.AbandonHouse RPC then HideUIPanel)",
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_main_frame_publishes_six_top_level_parent_keys(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    for key in [
        "CloseButton",
        "PlotAccess",
        "HouseAccess",
        "IgnoreListButton",
        "SaveButton",
        "AbandonHouseButton",
    ] {
        let key_type: String = env
            .eval(&format!("return type(HousingHouseSettingsFrame.{key})"))
            .expect("HousingHouseSettingsFrame parentKey lookup should succeed");
        assert_eq!(
            key_type, "table",
            "HousingHouseSettingsFrame.{key} must publish as a child frame — the XML declares it \
             as a top-level parentKey under HousingHouseSettingsFrame's <Frames>"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_access_options_inherit_template_methods(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    for slot in ["PlotAccess", "HouseAccess"] {
        let inherits: bool = env
            .eval(&format!(
                "return type(HousingHouseSettingsFrame.{slot}.SetupOptions) == 'function'"
            ))
            .expect("access slot SetupOptions lookup should succeed");
        assert!(
            inherits,
            "HousingHouseSettingsFrame.{slot} must inherit HouseSettingsAccessOptionsMixin \
             methods — the XML declares the slot as `inherits=\"HouseSettingsAccessOptionsTemplate\"` \
             so the mixin is mixed into the instance, exposing SetupOptions"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_abandon_dialog_publishes_refund_money_frame_companion(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    let global_type: String = env
        .eval("return type(AbandonHouseRefundMoneyFrame)")
        .expect("AbandonHouseRefundMoneyFrame type query should succeed");
    assert_eq!(
        global_type, "table",
        "AbandonHouseRefundMoneyFrame must publish as a `_G` frame — the XML declares the named \
         SmallMoneyFrameTemplate child at line 111 with BOTH parentKey=\"RefundMoneyFrame\" AND \
         name=\"AbandonHouseRefundMoneyFrame\" (parented to AbandonHouseConfirmationDialog's \
         RefundContainer); the named form is what existing C_Housing money-frame update calls \
         can find via `_G`"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_house_settings_registers_ui_panel_with_center_area_and_pushable_zero(env: &WowLuaEnv) {
    load_housing_house_settings(env);
    let area: String = env
        .eval("return UIPanelWindows['HousingHouseSettingsFrame'].area")
        .expect("UIPanelWindows area lookup should succeed");
    let pushable: i64 = env
        .eval("return UIPanelWindows['HousingHouseSettingsFrame'].pushable")
        .expect("UIPanelWindows pushable lookup should succeed");
    assert_eq!(
        area, "center",
        "Blizzard_HousingHouseSettingsRegistration.lua must register HousingHouseSettingsFrame \
         with area=\"center\" — the panel floats centered (movable=true in the XML lets the \
         player drag it), distinct from the left-area HouseFinder / Dashboard panels"
    );
    assert_eq!(
        pushable, 0,
        "Blizzard_HousingHouseSettingsRegistration.lua must register HousingHouseSettingsFrame \
         with pushable=0 — lowest displacement priority gets pushed by every higher-rank panel"
    );
}
}
