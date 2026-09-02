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

fn housing_controls_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_HousingControls")
}

fn housing_controls_toc() -> PathBuf {
    housing_controls_dir().join("Blizzard_HousingControls.toc")
}

fn load_housing_controls(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &housing_controls_toc())
        .expect("Blizzard_HousingControls should load via explicit Rust loader call");
}

#[test]
fn blizzard_housing_controls_find_toc_resolves_bare_variant() {
    let resolved = find_toc_file(&housing_controls_dir())
        .expect("Blizzard_HousingControls TOC should resolve");
    assert_eq!(
        resolved,
        housing_controls_toc(),
        "Blizzard_HousingControls ships exactly one bare TOC \
         (`Blizzard_HousingControls.toc`) — no flavor variants. The decor controls top-strip \
         only ships on retail (`## AllowLoadGameType: standard`) and uses the bare TOC suffix \
         that `find_toc_file` (src/loader/mod.rs:65) falls through to"
    );
}

#[test]
fn blizzard_housing_controls_toc_declares_lod_with_single_dependency() {
    let toc =
        TocFile::from_file(&housing_controls_toc()).expect("HousingControls TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_HousingControls declares `## LoadOnDemand: 1` — the decor-controls top-strip \
         only loads when the player enters a housing plot or house. Three independent triggers \
         pull it: `Blizzard_UIParent/Mainline/UIParent.lua:1622` calls \
         `C_AddOns.LoadAddOn(\"Blizzard_HousingControls\")` from a global event guarded by \
         `C_Housing.IsInsideHouseOrPlot()`; \
         `Blizzard_HousingEventHandler/Blizzard_HousingEventHandler.lua:285` calls it from \
         `HousingEventHandlerMixin:OnPlotEntered` after a HousingControlsFrame nil-check; \
         `Blizzard_HouseEditor/Blizzard_HouseEditor.toc:4` declares it as a regular \
         `## Dependencies` entry, so any explicit LoD load of HouseEditor pulls HousingControls \
         transitively"
    );
    assert!(
        !toc.is_load_first(),
        "Blizzard_HousingControls does not declare `## LoadFirst: 1` — LoadOnDemand precludes \
         any load-order priority"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_HousingControls does not declare `## UseSecureEnvironment` — runs in the \
         standard Lua environment"
    );
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_HousingTemplates".to_string()],
        "Blizzard_HousingControls declares exactly one `## Dependencies:` entry — \
         Blizzard_HousingTemplates provides the `controls-frame` + `controls-frame-guest` + \
         `decor-controls-decoratemode-{{inactive,active,pressed}}` + \
         `decor-controls-settings-{{default,active,pressed}}` + \
         `decor-controls-exit-{{default,active,pressed}}` + \
         `decor-controls-houseinfo-{{default,active,pressed}}` + \
         `decor-controls-inspect-{{default,active,pressed}}` + `keybind-bg` + `keybind-bg_active` \
         atlas references plus the BaseHousingActionButtonTemplate / \
         BaseHousingModeButtonTemplate base button templates plus the \
         BaseHousingActionButtonMixin / BaseHousingModeButtonMixin base mixins, the \
         HousingFramesUtil module, and the HOUSING_CONTROL_PANEL_TITLE / WHITE_FONT_COLOR / \
         DARKGRAY_COLOR color references"
    );
}

#[test]
fn blizzard_housing_controls_toc_is_retail_only_and_omits_allow_load() {
    let toc =
        TocFile::from_file(&housing_controls_toc()).expect("HousingControls TOC should parse");
    let toc_text = std::fs::read_to_string(housing_controls_toc()).expect("TOC should read");
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Blizzard_HousingControls declares `## AllowLoadGameType: standard` — the decor \
         controls top-strip is a Midnight expansion feature that only ships on retail. \
         `is_game_type_restricted()` (src/toc.rs:294) treats `standard` and `mainline` as the \
         unrestricted retail flavor"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_HousingControls must NOT be game-type restricted — `## AllowLoadGameType: \
         standard` matches the retail flavor that the simulator runs as"
    );
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_HousingControls omits `## AllowLoad:` — LoadOnDemand precludes auto-discovery \
         gating, so the AllowLoad value would be inert"
    );
    assert!(
        !toc_text.contains("## DefaultState:"),
        "Blizzard_HousingControls omits `## DefaultState:` — relies on the loader's \
         implicit-enabled default for Blizzard prefix LoD addons"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_HousingControls declares NO `## SavedVariables*` — control-strip visibility \
         is server-driven via HOUSE_PLOT_ENTERED / HOUSE_PLOT_EXITED / \
         HOUSE_EDITOR_AVAILABILITY_CHANGED / CURRENT_HOUSE_INFO_RECIEVED / \
         HOUSE_EDITOR_MODE_CHANGED / UPDATE_BINDINGS / HOUSE_INFO_UPDATED events plus \
         C_Housing.IsInsideHouseOrPlot() polling, so no per-installation persistence is needed"
    );
}

#[test]
fn blizzard_housing_controls_toc_lists_bootstrap_and_four_runtime_files() {
    let toc =
        TocFile::from_file(&housing_controls_toc()).expect("HousingControls TOC should parse");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        files,
        vec![
            "Blizzard_HousingControls_Bootstrap.lua".to_string(),
            "Blizzard_HousingControlButton.lua".to_string(),
            "Blizzard_HousingControlButton.xml".to_string(),
            "Blizzard_HousingControls.lua".to_string(),
            "Blizzard_HousingControls.xml".to_string(),
        ],
        "Blizzard_HousingControls TOC body lists exactly 5 source files in this exact order. \
         The first file is Blizzard_HousingControls_Bootstrap.lua, annotated inline with \
         `[Bootstrap]`; it publishes HousingControls_LoadUI(), which delegates to \
         LoadAddOnWithErrorHandling(AddonName). The annotation does not reorder this TOC. \
         The remaining files define the control-button mixins and templates, then the \
         HousingControls frame and XML."
    );
}

#[test]
fn blizzard_housing_controls_directory_holds_six_entries() {
    let dir = housing_controls_dir();
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_HousingControls directory should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        6,
        "Blizzard_HousingControls directory ships exactly 6 entries: the 5 TOC files and \
         the bare TOC. No flavor subdirectory or Localization.lua. Got: {entries:?}"
    );
    assert!(
        entries.contains(&"Blizzard_HousingControls.toc".to_string()),
        "Blizzard_HousingControls directory must contain the bare TOC file"
    );
    assert!(
        entries.contains(&"Blizzard_HousingControls_Bootstrap.lua".to_string()),
        "Blizzard_HousingControls directory must contain the Bootstrap file that publishes \
         HousingControls_LoadUI"
    );
}

#[test]
fn blizzard_housing_controls_excluded_from_all_screen_auto_discovery_passes() {
    let ui = blizzard_ui_dir();
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&ui, screen);
        let discovered = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_HousingControls");
        assert!(
            !discovered,
            "Blizzard_HousingControls MUST NOT appear in {screen:?} auto-discovery — \
             `## LoadOnDemand: 1` keeps it out of every screen pass. Three consumers pull it \
             via explicit `C_AddOns.LoadAddOn(\"Blizzard_HousingControls\")` calls \
             (UIParent.lua:1622, HousingEventHandler.lua:285) or via the LoD-loaded \
             Blizzard_HouseEditor's `## Dependencies` chain — none uses `## RequiredDep:`, so \
             the LoD-pull promotion path in `pull_required_lod_addons` (src/loader/mod.rs:357) \
             does not escalate HousingControls onto any auto-discovery pass"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_housing_controls_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_housing_controls(env);

    let lua_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let related: Vec<&String> = lua_errors
        .iter()
        .filter(|e| {
            e.contains("Blizzard_HousingControls/")
                || e.contains("Blizzard_HousingControls\\")
                || e.contains("HousingControlsMixin")
                || e.contains("VisitorControlFrameMixin")
                || e.contains("BaseHousingControlButtonMixin")
                || e.contains("HouseEditorButtonMixin")
                || e.contains("HouseExitButtonMixin")
                || e.contains("HouseInfoButtonMixin")
                || e.contains("HouseInspectorButtonMixin")
                || e.contains("HouseSettingsButtonMixin")
        })
        .collect();
    assert!(
        related.is_empty(),
        "Blizzard_HousingControls emitted addon-specific Lua errors during explicit LoD load:\n  {}",
        related
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_controls_is_addon_loaded_returns_true_after_explicit_lod_load(env: &WowLuaEnv) {
    load_housing_controls(env);

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingControls')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        loaded,
        "After explicit `load_addon` of Blizzard_HousingControls.toc following Game-screen \
         auto-discovery (which loads Blizzard_HousingTemplates as a normal Game-screen addon \
         but skips HousingControls itself due to LoadOnDemand), \
         `C_AddOns.IsAddOnLoaded('Blizzard_HousingControls')` should return true"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_controls_publishes_housing_controls_frame_global(env: &WowLuaEnv) {
    load_housing_controls(env);

    let exists: bool = env
        .eval(
            "local f = _G['HousingControlsFrame']; return type(f) == 'table' and type(f.GetName) == 'function'",
        )
        .expect("HousingControlsFrame global lookup should succeed");
    assert!(
        exists,
        "After LoD load, `HousingControlsFrame` should publish as a global frame instance — \
         Blizzard_HousingControls.xml line 3 declares `<Frame name=\"HousingControlsFrame\" \
         mixin=\"HousingControlsMixin\" toplevel=\"true\" parent=\"UIParent\">`. The frame \
         self-anchors to TOP of UIParent at offset (0, -30) and sizes itself to 150x50 (the \
         child OwnerControlFrame and VisitorControlFrame swap visibility based on \
         C_HousingNeighborhood.IsPlayerInOtherPlayersPlot or being inside someone else's house)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_controls_visitor_frame_publishes_layout_parent_keys(env: &WowLuaEnv) {
    load_housing_controls(env);

    let contract: (bool, bool, bool, bool) = env
        .eval(
            "local f = HousingControlsFrame and HousingControlsFrame.VisitorControlFrame; \
             return type(f) == 'table', \
                    f and f.OwnerNameText ~= nil, \
                    f and f.Divider ~= nil, \
                    type(f and f.ButtonContainer) == 'table'",
        )
        .expect("VisitorControlFrame layout parentKey probe should succeed");
    assert_eq!(
        contract,
        (true, true, true, true),
        "Current HousingControlsFrame.VisitorControlFrame owns OwnerNameText, Divider, and \
         ButtonContainer."
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_controls_visitor_control_frame_button_container_publishes_four_actions(env: &WowLuaEnv) {
    load_housing_controls(env);

    let actions: (bool, bool, bool, bool) = env
        .eval(
            "local f = HousingControlsFrame and HousingControlsFrame.VisitorControlFrame; \
             local buttons = f and f.ButtonContainer; \
             return buttons and buttons.VisitorInspectorButton ~= nil, \
                    buttons and buttons.BlueprintsButton ~= nil, \
                    buttons and buttons.VisitorHouseInfoButton ~= nil, \
                    buttons and buttons.VisitorExitButton ~= nil",
        )
        .expect("VisitorControlFrame action parentKey probe should succeed");
    assert_eq!(
        actions,
        (true, true, true, true),
        "HousingControlsFrame.VisitorControlFrame.ButtonContainer owns VisitorInspectorButton, \
         BlueprintsButton, VisitorHouseInfoButton, and VisitorExitButton."
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_controls_mixin_publishes_seven_methods(env: &WowLuaEnv) {
    load_housing_controls(env);

    for method in [
        "OnLoad",
        "OnEvent",
        "UpdateControlVisibility",
        "OnShow",
        "OnHide",
        "GetActiveFrame",
        "UpdateButtons",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingControlsMixin['{method}']) == 'function'"
            ))
            .expect("HousingControlsMixin method existence query should succeed");
        assert!(
            exists,
            "HousingControlsMixin must expose `:{method}()` — retail 12.1 updates visibility \
             from C_HouseEditor.GetHouseEditorPlayerType(), selects OwnerControlFrame or \
             VisitorControlFrame directly in UpdateControlVisibility, and updates the selected \
             frame's buttons through GetActiveFrame()"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_visitor_control_frame_layout_mixin_publishes_owner_info_method(env: &WowLuaEnv) {
    load_housing_controls(env);

    let exists: bool = env
        .eval("return type(HousingVisitorControlsLayoutMixin['UpdateOwnerInfomation']) == 'function'")
        .expect("HousingVisitorControlsLayoutMixin method existence query should succeed");
    assert!(
        exists,
        "HousingVisitorControlsLayoutMixin must expose `:UpdateOwnerInfomation()` (Blizzard's \
         `Infomation` spelling) to populate OwnerNameText from C_Housing.GetCurrentHouseInfo()."
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_controls_publishes_six_button_mixins_globally(env: &WowLuaEnv) {
    load_housing_controls(env);

    for mixin in [
        "BaseHousingControlButtonMixin",
        "HouseEditorButtonMixin",
        "HouseExitButtonMixin",
        "HouseInfoButtonMixin",
        "HouseInspectorButtonMixin",
        "HouseSettingsButtonMixin",
    ] {
        let exists: bool = env
            .eval(&format!("return type(_G['{mixin}']) == 'table'"))
            .expect("Mixin global lookup should succeed");
        assert!(
            exists,
            "{mixin} must publish as a global table after LoD load — the 6 button mixins are \
             declared at module scope in Blizzard_HousingControlButton.lua: \
             BaseHousingControlButtonMixin (the shared base — provides GetDefaultTexture, \
             GetIconForState (3-way enabled/pressed/active branch), GetIconColorForState \
             (WHITE_FONT_COLOR vs DARKGRAY_COLOR), IsActive default-not-implemented assert, \
             CheckEnabled with Kiosk + nyiLabel guards, OnClick with Kiosk skip + sound + \
             BaseHousingModeButtonMixin.OnClick chain), HouseEditorButtonMixin (CheckEnabled \
             directly compares C_HouseEditor.GetHouseEditorAvailability with \
             Enum.HousingResult.Success, IsActive queries C_HouseEditor.IsHouseEditorActive, \
             EnterMode uses C_HouseEditor.EnterHouseEditor, and LeaveMode calls \
             HousingFramesUtil.LeaveHouseEditor), \
             HouseExitButtonMixin (OnClick with Kiosk skip + C_Housing.LeaveHouse, IsActive \
             always false, CheckEnabled via IsInsideHouse AND \
             GetActiveHouseEditorMode==Enum.HouseEditorMode.None), HouseInfoButtonMixin \
             (OnClick LoadAddOn-pulls Blizzard_HousingCornerstone then ToggleUIPanel on \
             HousingCornerstoneHouseInfoFrame, CheckEnabled always true, IsActive checks the \
             info frame's IsShown), HouseInspectorButtonMixin (EnterMode LoadAddOn-pulls \
             Blizzard_HousingInspectModeUI then EnterInspectMode, LeaveMode ExitInspectMode, \
             CheckEnabled blocks while editor is active with \
             HOUSING_CONTROLS_INSPECT_UNAVAILABLE_EDITOR_ACTIVE, IsActive via \
             C_HousingInspectMode.IsInInspectMode), HouseSettingsButtonMixin (EnterMode \
             LoadAddOn-pulls Blizzard_HousingHouseSettings then ShowUIPanel, LeaveMode \
             HideUIPanel, IsActive via the settings frame's IsShown, CheckEnabled mirrors \
             HouseEditorButtonMixin's availability logic with \
             HOUSING_CONTROLS_SETTINGS_UNAVAILABLE_FMT)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_controls_does_not_publish_retired_util_helper(env: &WowLuaEnv) {
    load_housing_controls(env);

    let retired: bool = env
        .eval("return HousingControlsUtil == nil")
        .expect("HousingControlsUtil global lookup should succeed");
    assert!(
        retired,
        "Retail 12.1 Blizzard_HousingControls ships only bootstrap, button, and controls modules; \
         it no longer publishes the retired HousingControlsUtil helper table"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_controls_does_not_publish_virtual_button_templates(env: &WowLuaEnv) {
    load_housing_controls(env);

    for template in [
        "BaseHousingControlButtonTemplate",
        "HousingControlActionButtonTemplate",
        "HousingControlModeButtonTemplate",
        "HouseEditorButtonTemplate",
        "HouseSettingsButtonTemplate",
        "HousingExitButtonTemplate",
        "HouseInfoButtonTemplate",
        "HouseInspectorButtonTemplate",
    ] {
        let published: bool = env
            .eval(&format!("return _G['{template}'] ~= nil"))
            .expect("Template global lookup should succeed");
        assert!(
            !published,
            "{template} is declared `virtual=\"true\"` in \
             Blizzard_HousingControlButton.xml — virtual XML templates are NOT instantiated as \
             global frames at load time. They only materialize when a parent frame inherits \
             them. The HousingControlsFrame's OwnerControlFrame instantiates 5 of these (one \
             of each specific-implementation template) per the inherits= attribute in its \
             child Buttons, but the template names themselves stay out of `_G`"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_controls_owner_control_frame_publishes_five_buttons(env: &WowLuaEnv) {
    load_housing_controls(env);

    let owner_frame_exists: bool = env
        .eval(
            "local f = HousingControlsFrame.OwnerControlFrame; return type(f) == 'table' and type(f.GetName) == 'function'",
        )
        .expect("OwnerControlFrame parentKey lookup should succeed");
    assert!(
        owner_frame_exists,
        "HousingControlsFrame.OwnerControlFrame must publish via parentKey — \
         Blizzard_HousingControls.xml line 10 declares `<Frame parentKey=\"OwnerControlFrame\" \
         setAllPoints=\"true\" hidden=\"true\">` as the container for the 5 owner-mode buttons"
    );

    for parent_key in [
        "HouseEditorButton",
        "SettingsButton",
        "ExitButton",
        "BlueprintsButton",
        "InspectorButton",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingControlsFrame.OwnerControlFrame['{parent_key}']) ~= 'nil'"
            ))
            .expect("OwnerControlFrame parentKey child lookup should succeed");
        assert!(
            exists,
            "HousingControlsFrame.OwnerControlFrame.{parent_key} must publish via parentKey — \
             the XML wires 5 button children with `parentKey=` so that the HousingControlsMixin \
             can address them without touching `_G`: HouseEditorButton (68x68 \
             HouseEditorButtonTemplate centered with -12 y-offset), SettingsButton \
             (HouseSettingsButtonTemplate left of editor with 10,3 offset), ExitButton \
             (HousingExitButtonTemplate left of settings), BlueprintsButton \
             (HousingBlueprintActionButtonTemplate right of editor with -10,3 offset), \
             InspectorButton (HouseInspectorButtonTemplate right of BlueprintsButton). All 5 \
             inherit BaseHousingControlButtonTemplate's `parentArray=\"Buttons\"`, so they \
             collect into HousingControlsFrame.OwnerControlFrame.Buttons for the UpdateButtons loop"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_controls_dependency_loads_via_game_screen_pass(env: &WowLuaEnv) {
    load_housing_controls(env);

    let templates_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingTemplates')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        templates_loaded,
        "Blizzard_HousingTemplates must be loaded by the Game-screen auto-discovery pass \
         before the explicit HousingControls LoD load runs. HousingTemplates is the only \
         `## Dependencies` entry on HousingControls's TOC, and the test harness's full \
         Game-screen pass hits it via the normal discovery flow because HousingTemplates is \
         itself non-LoD with `## AllowLoad: Both` semantics"
    );
}
}
