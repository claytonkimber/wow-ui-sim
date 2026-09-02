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

fn create_neighborhood_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_HousingCreateNeighborhood")
}

fn create_neighborhood_toc() -> PathBuf {
    create_neighborhood_dir().join("Blizzard_HousingCreateNeighborhood.toc")
}

fn load_housing_create_neighborhood(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &create_neighborhood_toc())
        .expect("Blizzard_HousingCreateNeighborhood should load via explicit Rust loader call");
}

#[test]
fn blizzard_housing_create_neighborhood_find_toc_resolves_bare_variant() {
    let resolved = find_toc_file(&create_neighborhood_dir())
        .expect("Blizzard_HousingCreateNeighborhood TOC should resolve");
    assert_eq!(
        resolved,
        create_neighborhood_toc(),
        "Blizzard_HousingCreateNeighborhood ships exactly one bare TOC — retail-only addon \
         resolves via `find_toc_file` fallthrough"
    );
}

#[test]
fn blizzard_housing_create_neighborhood_toc_declares_lod_with_one_dependency() {
    let toc = TocFile::from_file(&create_neighborhood_toc())
        .expect("HousingCreateNeighborhood TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_HousingCreateNeighborhood declares `## LoadOnDemand: 1` — pulled via explicit \
         LoadAddOn from Blizzard_HousingCharter.lua:27 and Blizzard_HousingEventHandler.lua:346/\
         354/362 (3 separate event paths inside HousingEventHandlerMixin)"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_HousingTemplates".to_string()],
        "Single `## Dependencies:` entry: Blizzard_HousingTemplates (provides housing-wood-frame \
         / house-drawing-stone-bg / housing-basic-panel-gradient-header-bg / \
         housing-basic-horizontal-gradient-black / housing-basic-vertical-divider / \
         housing-decorative-foliage-left atlas families plus HousingTopBannerFrame + \
         TopBannerManager_Show + HousingResultToErrorText referenced by the base mixin's \
         CREATE_NEIGHBORHOOD_RESULT handler)"
    );
}

#[test]
fn blizzard_housing_create_neighborhood_toc_is_retail_only_and_omits_allow_load() {
    let toc = TocFile::from_file(&create_neighborhood_toc())
        .expect("HousingCreateNeighborhood TOC should parse");
    let toc_text = std::fs::read_to_string(create_neighborhood_toc()).expect("TOC should read");
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
        "No `## SavedVariables*` — neighborhood creation results are server-driven via \
         CREATE_NEIGHBORHOOD_RESULT / NEIGHBORHOOD_NAME_VALIDATED / \
         NEIGHBORHOOD_GUILD_SIZE_VALIDATED events"
    );
}

#[test]
fn blizzard_housing_create_neighborhood_toc_lists_three_files_in_order() {
    let toc = TocFile::from_file(&create_neighborhood_toc())
        .expect("HousingCreateNeighborhood TOC should parse");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        files,
        vec![
            "Blizzard_HousingCreateNeighborhood.lua".to_string(),
            "Blizzard_HousingCreateNeighborhood.xml".to_string(),
            "Blizzard_HousingCreateNeighborhoodRegistration.lua".to_string(),
        ],
        "TOC body lists exactly 3 source files in this order: .lua (6 mixins) before .xml \
         (mixin= references) before Registration.lua (RegisterUIPanel tail file)"
    );
}

#[test]
fn blizzard_housing_create_neighborhood_directory_holds_four_entries() {
    let dir = create_neighborhood_dir();
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_HousingCreateNeighborhood directory should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        4,
        "Directory ships exactly 4 entries (3 source files + 1 TOC). Got: {entries:?}"
    );
    assert!(entries.contains(&"Blizzard_HousingCreateNeighborhood.toc".to_string()));
    assert!(entries.contains(&"Blizzard_HousingCreateNeighborhoodRegistration.lua".to_string()));
}

#[test]
fn blizzard_housing_create_neighborhood_excluded_from_all_screen_auto_discovery_passes() {
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
            .any(|(name, _)| name == "Blizzard_HousingCreateNeighborhood");
        assert!(
            !discovered,
            "Must NOT appear in {screen:?} auto-discovery — `## LoadOnDemand: 1` keeps it out \
             of every screen pass; consumers pull via explicit LoadAddOn"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_housing_create_neighborhood_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    let lua_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let related: Vec<&String> = lua_errors
        .iter()
        .filter(|e| {
            e.contains("Blizzard_HousingCreateNeighborhood/")
                || e.contains("Blizzard_HousingCreateNeighborhood\\")
                || e.contains("HousingCreateNeighborhoodMixin")
                || e.contains("HousingCreateNeighborhoodConfirmationMixin")
                || e.contains("HousingCreateCharterNeighborhoodConfirmationMixin")
                || e.contains("HousingCreateGuildNeighborhoodConfirmationMixin")
                || e.contains("HousingCreateGuildNeighborhoodMixin")
                || e.contains("HousingCreateNeighborhoodCharterMixin")
        })
        .collect();
    assert!(
        related.is_empty(),
        "Blizzard_HousingCreateNeighborhood emitted addon-specific Lua errors during explicit \
         LoD load:\n  {}",
        related
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_neighborhood_is_addon_loaded_returns_true_after_explicit_lod_load(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingCreateNeighborhood')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        loaded,
        "After explicit LoD load, `IsAddOnLoaded('Blizzard_HousingCreateNeighborhood')` must \
         return true"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_neighborhood_dependency_loads_via_game_screen_pass(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    let templates_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingTemplates')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        templates_loaded,
        "Blizzard_HousingTemplates must be auto-loaded — sole dependency on CreateNeighborhood's \
         TOC, must be loaded before the explicit LoD load runs"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_neighborhood_publishes_three_named_frames_globally(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for frame_name in [
        "HousingCreateGuildNeighborhoodFrame",
        "HousingCreateNeighborhoodCharterFrame",
        "HousingCreateCharterNeighborhoodConfirmationFrame",
    ] {
        let exists: bool = env
            .eval(&format!(
                "local f = _G['{frame_name}']; return type(f) == 'table' and type(f.GetName) == 'function'"
            ))
            .expect("Named frame global lookup should succeed");
        assert!(
            exists,
            "After LoD load, `{frame_name}` should publish as a global frame instance — \
             CreateNeighborhood XML declares 3 named non-virtual frames at file scope: the \
             guild-neighborhood creation frame, the charter creation frame, and the charter \
             confirmation dialog"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_neighborhood_does_not_publish_virtual_templates(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for template_name in [
        "HousingCreateNeighborhoodTemplate",
        "HousingCreateNeighborhoodConfirmationTemplate",
    ] {
        let published: bool = env
            .eval(&format!("return _G['{template_name}'] ~= nil"))
            .expect("Template global lookup should succeed");
        assert!(
            !published,
            "{template_name} is `virtual=\"true\"` — virtual XML templates are not instantiated \
             as global frames; only inheriting frames materialize"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_neighborhood_base_mixin_publishes_three_methods(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for method in [
        "CreateNeighborhoodBaseOnLoad",
        "CreateNeighborhoodBaseOnEvent",
        "CreateNeighborhoodBaseOnShow",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCreateNeighborhoodMixin['{method}']) == 'function'"
            ))
            .expect("HousingCreateNeighborhoodMixin method existence query should succeed");
        assert!(
            exists,
            "HousingCreateNeighborhoodMixin must expose `:{method}()` — base shared by Guild + \
             Charter via HousingCreateNeighborhoodTemplate inheritance; Base prefix avoids \
             collision with subclass OnLoad/OnEvent/OnShow handlers chained via inherit=append; \
             CreateNeighborhoodBaseOnLoad sets NeighborhoodNameEditBox max letters; \
             CreateNeighborhoodBaseOnEvent dispatches CREATE_NEIGHBORHOOD_RESULT routing success \
             to HousingTopBannerFrame:SetBannerText + TopBannerManager_Show vs failure to \
             UIErrorsFrame via HousingResultToErrorText, then unregisters the event"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_neighborhood_confirmation_base_mixin_publishes_one_method(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    let exists: bool = env
        .eval(
            "return type(HousingCreateNeighborhoodConfirmationMixin['CreateNeighborhoodConfirmationBaseOnLoad']) == 'function'",
        )
        .expect("HousingCreateNeighborhoodConfirmationMixin method existence query should succeed");
    assert!(
        exists,
        "HousingCreateNeighborhoodConfirmationMixin must expose \
         `:CreateNeighborhoodConfirmationBaseOnLoad()` — base shared by both confirmation \
         dialogs (Charter and Guild ConfirmationFrame nested child); only sets CancelButton \
         text via HOUSING_CREATENEIGHBORHOOD_CANCELBUTTON"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_charter_confirmation_mixin_publishes_five_methods(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for method in ["OnLoad", "SetCharterInfo", "OnShow", "OnHide", "OnEvent"] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCreateCharterNeighborhoodConfirmationMixin['{method}']) == 'function'"
            ))
            .expect(
                "HousingCreateCharterNeighborhoodConfirmationMixin method existence query should succeed",
            );
        assert!(
            exists,
            "HousingCreateCharterNeighborhoodConfirmationMixin must expose `:{method}()` — \
             charter-confirmation dialog mixin inheriting the confirmation base; OnLoad wires \
             ConfirmButton:OnClick to register CREATE_NEIGHBORHOOD_RESULT on the charter frame \
             plus C_Housing.OnCharterConfirmationAccepted + HideUIPanel + sound; SetCharterInfo \
             populates LocationText + NeighborhoodNameText; OnShow registers \
             CharterConfirmationFrameShowingEvents (1 event CLOSE_CHARTER_CONFIRMATION_UI); \
             OnHide unregisters + C_Housing.OnCharterConfirmationClosed; OnEvent dispatches \
             CLOSE_CHARTER_CONFIRMATION_UI → HideUIPanel"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_guild_confirmation_mixin_publishes_two_methods(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for method in ["OnLoad", "OnShow"] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCreateGuildNeighborhoodConfirmationMixin['{method}']) == 'function'"
            ))
            .expect(
                "HousingCreateGuildNeighborhoodConfirmationMixin method existence query should succeed",
            );
        assert!(
            exists,
            "HousingCreateGuildNeighborhoodConfirmationMixin must expose `:{method}()` — nested \
             ConfirmationFrame child of HousingCreateGuildNeighborhoodFrame; OnLoad wires \
             ConfirmButton:OnClick to register CREATE_NEIGHBORHOOD_RESULT + \
             C_Housing.CreateGuildNeighborhood with the EditBox text + Hide ConfirmationFrame \
             + sound + HideUIPanel of the parent guild frame; OnShow copies LocationText / \
             GuildText / NeighborhoodNameText from the parent HousingCreateGuildNeighborhoodFrame"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_guild_neighborhood_mixin_publishes_six_methods(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for method in [
        "OnCreateNeighborhoodClicked",
        "OnLoad",
        "OnShow",
        "OnEvent",
        "OnHide",
        "SetActiveLocationAndGuild",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCreateGuildNeighborhoodMixin['{method}']) == 'function'"
            ))
            .expect("HousingCreateGuildNeighborhoodMixin method existence query should succeed");
        assert!(
            exists,
            "HousingCreateGuildNeighborhoodMixin must expose `:{method}()` — guild-neighborhood \
             creation flow attached via inherit=append on top of base mixin; OnLoad calls \
             C_Housing.ValidateNeighborhoodName via OnCreateNeighborhoodClicked; OnShow \
             registers CreateGuildNeighborhoodFrameShowingEvents (3 events: \
             CLOSE_CREATE_GUILD_NEIGHBORHOOD_UI / NEIGHBORHOOD_GUILD_SIZE_VALIDATED / \
             NEIGHBORHOOD_NAME_VALIDATED) and calls C_Housing.ValidateCreateGuildNeighborhoodSize; \
             OnEvent dispatches name-validated → show ConfirmationFrame on success or \
             NeighborhoodNameError otherwise, size-validated → show NeighborhoodRequirementsError \
             with errorStrings table mapped from Enum.CreateNeighborhoodErrorType.UndersizedGuild; \
             OnHide unregisters + C_Housing.OnCreateGuildNeighborhoodClosed; \
             SetActiveLocationAndGuild populates LocationText + GuildText via GetGuildInfo"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_neighborhood_charter_mixin_publishes_seven_methods(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for method in [
        "SetCharterInfo",
        "OnConfirmClicked",
        "OnLoad",
        "OnShow",
        "OnEvent",
        "OnHide",
        "SetActiveLocation",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCreateNeighborhoodCharterMixin['{method}']) == 'function'"
            ))
            .expect("HousingCreateNeighborhoodCharterMixin method existence query should succeed");
        assert!(
            exists,
            "HousingCreateNeighborhoodCharterMixin must expose `:{method}()` — charter creation \
             flow with edit-existing-charter mode toggled via SetCharterInfo's neighborhoodName \
             arg (truthy → edit mode shows CharterSettingsWarning + ConfirmButton text \
             HOUSING_CREATENEIGHBORHOOD_CONFIRMBUTTON, falsy → fresh mode hides warning + uses \
             HOUSING_CREATENEIGHBORHOOD_CHARTER_CONFIRMBUTTON); OnEvent dispatches \
             NEIGHBORHOOD_NAME_VALIDATED branching on isEditingCharter to call either \
             C_Housing.EditNeighborhoodCharter or C_Housing.CreateNeighborhoodCharter; OnHide \
             calls C_Housing.OnCreateCharterNeighborhoodClosed"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_guild_frame_inherits_template_children(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for parent_key in [
        "Border",
        "Background",
        "Header",
        "DetailsBackground",
        "DetailsDivider",
        "PlantDecoLeft",
        "Title",
        "LocationLabel",
        "LocationText",
        "NeighborhoodNameLabel",
        "NeighborhoodNameError",
        "NeighborhoodInfoLabel",
        "NeighborhoodInfoText",
        "NeighborhoodNameEditBox",
        "ConfirmButton",
        "CancelButton",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCreateGuildNeighborhoodFrame['{parent_key}']) ~= 'nil'"
            ))
            .expect("HousingCreateGuildNeighborhoodFrame parentKey lookup should succeed");
        assert!(
            exists,
            "HousingCreateGuildNeighborhoodFrame.{parent_key} must publish via parentKey from \
             inherited HousingCreateNeighborhoodTemplate — shared with \
             HousingCreateNeighborhoodCharterFrame which inherits the same template"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_guild_frame_adds_guild_specific_children(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for parent_key in [
        "GuildLabel",
        "GuildText",
        "NeighborhoodRequirementsError",
        "ConfirmationFrame",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCreateGuildNeighborhoodFrame['{parent_key}']) ~= 'nil'"
            ))
            .expect("HousingCreateGuildNeighborhoodFrame parentKey lookup should succeed");
        assert!(
            exists,
            "HousingCreateGuildNeighborhoodFrame.{parent_key} must publish via parentKey — \
             guild-specific children added on top of the inherited template: GuildLabel + \
             GuildText FontStrings (populated from GetGuildInfo), NeighborhoodRequirementsError \
             FontString (PURE_RED_COLOR, anchored BOTTOM y=57), and the nested ConfirmationFrame \
             which inherits HousingCreateNeighborhoodConfirmationTemplate + the \
             HousingCreateGuildNeighborhoodConfirmationMixin"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_guild_frame_nested_confirmation_publishes_subtree(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    let confirmation_exists: bool = env
        .eval(
            "local f = HousingCreateGuildNeighborhoodFrame.ConfirmationFrame; return type(f) == 'table' and type(f.GetName) == 'function'",
        )
        .expect("ConfirmationFrame parentKey lookup should succeed");
    assert!(
        confirmation_exists,
        "HousingCreateGuildNeighborhoodFrame.ConfirmationFrame must publish via parentKey — \
         DIALOG-strata nested child inheriting HousingCreateNeighborhoodConfirmationTemplate + \
         the HousingCreateGuildNeighborhoodConfirmationMixin (the only confirmation mixin that \
         is a nested child rather than a top-level frame)"
    );

    for parent_key in [
        "Title",
        "ConfirmationText",
        "LocationLabel",
        "LocationText",
        "NeighborhoodNameLabel",
        "NeighborhoodNameText",
        "GuildLabel",
        "GuildText",
        "ConfirmButton",
        "CancelButton",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCreateGuildNeighborhoodFrame.ConfirmationFrame['{parent_key}']) ~= 'nil'"
            ))
            .expect("Nested ConfirmationFrame parentKey lookup should succeed");
        assert!(
            exists,
            "HousingCreateGuildNeighborhoodFrame.ConfirmationFrame.{parent_key} must publish \
             via parentKey — confirmation template children plus guild-specific GuildLabel + \
             GuildText nested layer added on top"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_charter_frame_inherits_template_and_adds_warning(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for parent_key in [
        "Border",
        "Background",
        "Header",
        "Title",
        "LocationLabel",
        "LocationText",
        "NeighborhoodNameLabel",
        "NeighborhoodNameError",
        "NeighborhoodInfoLabel",
        "NeighborhoodInfoText",
        "NeighborhoodNameEditBox",
        "ConfirmButton",
        "CancelButton",
        "CharterSettingsWarning",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCreateNeighborhoodCharterFrame['{parent_key}']) ~= 'nil'"
            ))
            .expect("HousingCreateNeighborhoodCharterFrame parentKey lookup should succeed");
        assert!(
            exists,
            "HousingCreateNeighborhoodCharterFrame.{parent_key} must publish via parentKey — \
             inherited HousingCreateNeighborhoodTemplate children plus the charter-specific \
             CharterSettingsWarning FontString (PURE_RED_COLOR, anchored BOTTOM y=65, hidden by \
             default) toggled by SetCharterInfo's edit-mode branch"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_charter_confirmation_frame_publishes_template_children(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for parent_key in [
        "Border",
        "Background",
        "Header",
        "DetailsBackground",
        "DetailsDivider",
        "PlantDecoLeft",
        "Title",
        "ConfirmationText",
        "LocationLabel",
        "LocationText",
        "NeighborhoodNameLabel",
        "NeighborhoodNameText",
        "ConfirmButton",
        "CancelButton",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCreateCharterNeighborhoodConfirmationFrame['{parent_key}']) ~= 'nil'"
            ))
            .expect(
                "HousingCreateCharterNeighborhoodConfirmationFrame parentKey lookup should succeed",
            );
        assert!(
            exists,
            "HousingCreateCharterNeighborhoodConfirmationFrame.{parent_key} must publish via \
             parentKey — DIALOG-strata top-level frame inheriting \
             HousingCreateNeighborhoodConfirmationTemplate; unlike the guild ConfirmationFrame \
             which is nested, this charter confirmation is a sibling top-level frame so the \
             confirmation template children include LocationLabel + LocationText + \
             NeighborhoodNameLabel + NeighborhoodNameText with no guild-specific extras"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_neighborhood_registers_three_named_panels_via_register_ui_panel(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    for frame_name in [
        "HousingCreateGuildNeighborhoodFrame",
        "HousingCreateNeighborhoodCharterFrame",
        "HousingCreateCharterNeighborhoodConfirmationFrame",
    ] {
        let registered: bool = env
            .eval(&format!(
                "return type(UIPanelWindows['{frame_name}']) == 'table'"
            ))
            .expect("UIPanelWindows registration query should succeed");
        assert!(
            registered,
            "UIPanelWindows['{frame_name}'] must publish after Registration.lua tail file — 3 \
             RegisterUIPanel calls sharing one local attributes table {{ area=\"center\", \
             pushable=2 }} (note pushable=2, distinct from HousingCornerstone's pushable=0; \
             omits allowOtherPanels which the loader treats as falsy)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_create_neighborhood_registration_uses_center_area_with_pushable_two(env: &WowLuaEnv) {
    load_housing_create_neighborhood(env);

    let area: String = env
        .eval("return UIPanelWindows['HousingCreateGuildNeighborhoodFrame'].area")
        .expect("UIPanelWindows.area query should succeed");
    assert_eq!(
        area, "center",
        "Registration.lua attributes.area = \"center\" — neighborhood creation flow uses \
         central panel layout"
    );

    let pushable: i64 = env
        .eval("return UIPanelWindows['HousingCreateGuildNeighborhoodFrame'].pushable")
        .expect("UIPanelWindows.pushable query should succeed");
    assert_eq!(
        pushable, 2,
        "Registration.lua attributes.pushable = 2 — these panels can be pushed aside by other \
         center-area windows (matches HousingCharter's pushable=2; distinct from \
         HousingCornerstone's pushable=0 which is non-pushable)"
    );

    let omits_allow_other: bool = env
        .eval(
            "return UIPanelWindows['HousingCreateGuildNeighborhoodFrame'].allowOtherPanels == nil",
        )
        .expect("UIPanelWindows.allowOtherPanels query should succeed");
    assert!(
        omits_allow_other,
        "Registration.lua omits allowOtherPanels — falls back to falsy default; only area and \
         pushable are explicitly declared in the shared attributes table"
    );
}
}
