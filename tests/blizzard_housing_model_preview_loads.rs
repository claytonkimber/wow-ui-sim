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

fn preview_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_HousingModelPreview")
}

fn preview_toc() -> PathBuf {
    preview_dir().join("Blizzard_HousingModelPreview.toc")
}

fn load_housing_model_preview(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &preview_toc())
        .expect("Blizzard_HousingModelPreview should load via explicit Rust loader call");
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
fn blizzard_housing_model_preview_find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&preview_dir()).expect("Blizzard_HousingModelPreview TOC should resolve");
    assert_eq!(
        resolved,
        preview_toc(),
        "Blizzard_HousingModelPreview ships exactly one bare TOC — retail-only addon resolves \
         via `find_toc_file` fallthrough"
    );
}

#[test]
fn blizzard_housing_model_preview_toc_declares_lod_with_one_dependency() {
    let toc = TocFile::from_file(&preview_toc()).expect("HousingModelPreview TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_HousingModelPreview declares `## LoadOnDemand: 1` — pulled via explicit \
         LoadAddOn from the housing catalog flow when the player clicks `Preview` on a catalog \
         decor entry; also embedded as `HousingModelPreviewTemplate` inside other LoD housing \
         frames once the TOC has been loaded"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_HousingTemplates".to_string()],
        "Single `## Dependencies:` entry: Blizzard_HousingTemplates (provides the housing atlas \
         surface plus the shared HousingCatalogDyeDisplayTemplate / RewardTrackArtButtonTemplate \
         / PanningModelSceneMixinTemplate templates the preview consumes from XML, plus the \
         Blizzard_HousingCatalogUtil helper module the lua queries for owned/stored counts)"
    );
}

#[test]
fn blizzard_housing_model_preview_toc_is_retail_only_and_omits_allow_load() {
    let toc = TocFile::from_file(&preview_toc()).expect("HousingModelPreview TOC should parse");
    let toc_text = std::fs::read_to_string(preview_toc()).expect("TOC should read");
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
        "No `## SavedVariables*` — preview is server-driven via C_HousingCatalog query surface; \
         no client-side persistence"
    );
}

#[test]
fn blizzard_housing_model_preview_toc_lists_three_files_in_order() {
    let toc = TocFile::from_file(&preview_toc()).expect("HousingModelPreview TOC should parse");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        files,
        vec![
            "Blizzard_HousingModelPreview.lua".to_string(),
            "Blizzard_HousingModelPreview.xml".to_string(),
            "Blizzard_HousingModelPreviewFrameRegistration.lua".to_string(),
        ],
        "TOC body lists exactly 3 source files in this exact order — the .lua defines \
         HousingModelPreviewMixin (12 methods on the embeddable template) and \
         HousingModelPreviewFrameMixin (4 methods on the standalone container), the .xml declares \
         the virtual HousingModelPreviewTemplate plus the named non-virtual \
         HousingModelPreviewFrame container, then the Registration tail loads LAST so the \
         HousingModelPreviewFrame `_G` reference is published when RegisterUIPanel runs"
    );
}

#[test]
fn blizzard_housing_model_preview_directory_holds_four_entries() {
    let dir = preview_dir();
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_HousingModelPreview directory should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        4,
        "Directory ships exactly 4 entries (3 source + 1 TOC, no flavor subdirectory). Got: \
         {entries:?}"
    );
    assert!(entries.contains(&"Blizzard_HousingModelPreview.toc".to_string()));
    assert!(entries.contains(&"Blizzard_HousingModelPreview.lua".to_string()));
    assert!(entries.contains(&"Blizzard_HousingModelPreview.xml".to_string()));
    assert!(entries.contains(&"Blizzard_HousingModelPreviewFrameRegistration.lua".to_string()));
}

#[test]
fn blizzard_housing_model_preview_excluded_from_all_screen_auto_discovery() {
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
            .any(|(name, _)| name == "Blizzard_HousingModelPreview");
        assert!(
            !discovered,
            "Must NOT appear in {screen:?} auto-discovery — `## LoadOnDemand: 1` excludes the \
             addon from every ScreenKind pass; it is only ever pulled via explicit LoadAddOn \
             from the housing catalog flow"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_housing_model_preview_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_housing_model_preview(env);

    let lua_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let related: Vec<&String> = lua_errors
        .iter()
        .filter(|e| {
            e.contains("Blizzard_HousingModelPreview/")
                || e.contains("Blizzard_HousingModelPreview\\")
                || e.contains("HousingModelPreviewMixin")
                || e.contains("HousingModelPreviewFrameMixin")
        })
        .collect();
    assert!(
        related.is_empty(),
        "Loading Blizzard_HousingModelPreview must not emit any addon-specific Lua errors. Got \
         {} errors: {:?}",
        related.len(),
        related
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_model_preview_is_addon_loaded_via_explicit_lod_call(env: &WowLuaEnv) {
    load_housing_model_preview(env);
    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingModelPreview')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_HousingModelPreview') must return true after the \
         explicit LoD load — proves the loader registered the addon name"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_model_preview_template_dependency_loads_via_game_screen_pass(env: &WowLuaEnv) {
    load_housing_model_preview(env);
    let templates_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingTemplates')")
        .expect("HousingTemplates IsAddOnLoaded query should succeed");
    assert!(
        templates_loaded,
        "Blizzard_HousingTemplates (the only declared dep) must auto-load via the Game-screen \
         discovery pass before the explicit Model Preview LoD load runs — it is NOT \
         LoadOnDemand so the auto-discovery sweep includes it"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_model_preview_publishes_main_frame_global(env: &WowLuaEnv) {
    load_housing_model_preview(env);
    let frame_type: String = env
        .eval("return type(HousingModelPreviewFrame)")
        .expect("HousingModelPreviewFrame type query should succeed");
    assert_eq!(
        frame_type, "table",
        "HousingModelPreviewFrame must publish as a `_G` frame — the XML declares the named \
         non-virtual frame at line 135 (mixin HousingModelPreviewFrameMixin, toplevel, \
         frameStrata=FULLSCREEN_DIALOG, parent=UIParent, hidden, inherits=ButtonFrameTemplate)"
    );
    let frame_name: String = env
        .eval("return HousingModelPreviewFrame:GetName()")
        .expect("HousingModelPreviewFrame:GetName query should succeed");
    assert_eq!(frame_name, "HousingModelPreviewFrame");
}
}

prefork_full_ui_case! {
fn blizzard_housing_model_preview_main_frame_is_fullscreen_dialog_strata(env: &WowLuaEnv) {
    load_housing_model_preview(env);
    let strata: String = env
        .eval("return HousingModelPreviewFrame:GetFrameStrata()")
        .expect("HousingModelPreviewFrame:GetFrameStrata query should succeed");
    assert_eq!(
        strata, "FULLSCREEN_DIALOG",
        "HousingModelPreviewFrame must publish frameStrata=\"FULLSCREEN_DIALOG\" — the XML \
         declares it explicitly so the preview floats above the catalog flow it was opened from"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_model_preview_template_stays_nil_in_globals(env: &WowLuaEnv) {
    load_housing_model_preview(env);
    let value_type: String = env
        .eval("return type(_G['HousingModelPreviewTemplate'])")
        .expect("template _G lookup should succeed");
    assert_eq!(
        value_type, "nil",
        "HousingModelPreviewTemplate is `virtual=\"true\"` so it must NOT publish to `_G` — \
         proves the loader honors the virtual flag (the embeddable preview body is consumed by \
         HousingModelPreviewFrame's ModelPreview parentKey child via `inherits=` only)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_model_preview_mixin_publishes_twelve_methods(env: &WowLuaEnv) {
    load_housing_model_preview(env);
    assert_mixin_methods(
        &env,
        "HousingModelPreviewMixin",
        &[
            "OnLoad",
            "PreviewCatalogEntryInfo",
            "GetSortedVariantInfosWithBase",
            "FindVariantIndex",
            "CycleVariant",
            "UpdateVariantButtons",
            "GetCurrentDyeNames",
            "ApplyCurrentVariant",
            "ClearPreviewData",
            "HasValidData",
            "SetupTextTooltip",
            "SetTextOrHide",
        ],
        "HousingModelPreviewMixin owns 12 methods on the embeddable preview template: OnLoad \
         (transition the ModelScene to the default decor scene + wire variant button OnClicks + \
         install 7 text-tooltip handlers), PreviewCatalogEntryInfo (entry-point that resets state \
         and applies the chosen variant), GetSortedVariantInfosWithBase (pull all variants from \
         C_HousingCatalog.GetAllVariantInfosForEntry, synthesize a base entry if absent, then \
         sort by variantIdentifier so base appears first), FindVariantIndex (locate the matching \
         variant in the sorted list), CycleVariant (clamped left/right step), UpdateVariantButtons \
         (show/hide the cycle buttons based on remaining variants), GetCurrentDyeNames (concat \
         dye names via C_DyeColor.GetDyeColorInfo), ApplyCurrentVariant (transition ModelScene + \
         set actor model + apply gradient mask dyes via SetGradientMaskWithDyes + refresh the \
         text container's source/collection-bonus/dye/owned rows), ClearPreviewData (nil out all \
         tracked state + Hide the variant buttons + ClearModel on the actor), HasValidData (truthy \
         check on catalogEntryInfo used by the tooltip handlers), SetupTextTooltip / SetTextOrHide \
         (helpers driving the 7 tooltip wirings and the show-or-hide text rows)",
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_model_preview_frame_mixin_publishes_four_methods(env: &WowLuaEnv) {
    load_housing_model_preview(env);
    assert_mixin_methods(
        &env,
        "HousingModelPreviewFrameMixin",
        &["OnLoad", "ShowCatalogEntryInfo", "OnShow", "OnHide"],
        "HousingModelPreviewFrameMixin owns 4 methods on the standalone ButtonFrameTemplate \
         container: OnLoad (HidePortrait + HideAttic + SetTitle to PREVIEW), \
         ShowCatalogEntryInfo (forward the (catalogEntry, variant) pair to the embedded \
         ModelPreview child then ShowUIPanel if hidden), OnShow / OnHide (play the standard \
         IG_CHARACTER_INFO_OPEN / IG_CHARACTER_INFO_CLOSE soundkit clips to match the rest of the \
         character-info-style panels)",
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_model_preview_frame_publishes_model_preview_child(env: &WowLuaEnv) {
    load_housing_model_preview(env);
    let key_type: String = env
        .eval("return type(HousingModelPreviewFrame.ModelPreview)")
        .expect("HousingModelPreviewFrame.ModelPreview lookup should succeed");
    assert_eq!(
        key_type, "table",
        "HousingModelPreviewFrame.ModelPreview must publish as a child frame — the XML declares \
         it as the single top-level parentKey under HousingModelPreviewFrame's <Frames>, \
         inheriting HousingModelPreviewTemplate so it carries HousingModelPreviewMixin's methods \
         into the instance"
    );
    let inherits_method: bool = env
        .eval("return type(HousingModelPreviewFrame.ModelPreview.PreviewCatalogEntryInfo) == 'function'")
        .expect("ModelPreview.PreviewCatalogEntryInfo lookup should succeed");
    assert!(
        inherits_method,
        "HousingModelPreviewFrame.ModelPreview must inherit HousingModelPreviewMixin methods — \
         the XML declares `inherits=\"HousingModelPreviewTemplate\"` which carries the mixin into \
         the instance, so PreviewCatalogEntryInfo (the entry-point used by ShowCatalogEntryInfo) \
         must resolve to a function"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_model_preview_registers_ui_panel_with_left_area_and_pushable_two(env: &WowLuaEnv) {
    load_housing_model_preview(env);
    let area: String = env
        .eval("return UIPanelWindows['HousingModelPreviewFrame'].area")
        .expect("UIPanelWindows area lookup should succeed");
    let pushable: i64 = env
        .eval("return UIPanelWindows['HousingModelPreviewFrame'].pushable")
        .expect("UIPanelWindows pushable lookup should succeed");
    assert_eq!(
        area, "left",
        "Blizzard_HousingModelPreviewFrameRegistration.lua must register HousingModelPreviewFrame \
         with area=\"left\" — the panel docks to the left side of the screen alongside other \
         catalog-flow panels (distinct from the center-area HouseSettings)"
    );
    assert_eq!(
        pushable, 2,
        "Blizzard_HousingModelPreviewFrameRegistration.lua must register HousingModelPreviewFrame \
         with pushable=2 — middle displacement priority gets pushed by higher-rank dialogs but \
         pushes lower-rank panels off-screen"
    );
}
}
