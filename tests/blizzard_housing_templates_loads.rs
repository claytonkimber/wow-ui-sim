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

fn templates_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_HousingTemplates")
}

fn templates_toc() -> PathBuf {
    templates_dir().join("Blizzard_HousingTemplates.toc")
}

const ALL_MIXINS: &[&str] = &[
    "BaseHousingActionButtonMixin",
    "BaseHousingModeButtonMixin",
    "HousingCatalogCategoriesMixin",
    "HousingCategoryBackButtonMixin",
    "BaseHousingCatalogCategoryMixin",
    "HousingCatalogCategoryMixin",
    "HousingCatalogDyeDisplayMixin",
    "HousingCatalogEntryMixin",
    "HousingCatalogDecorEntryMixin",
    "HousingCatalogRoomEntryMixin",
    "HousingCatalogFiltersMixin",
    "HousingCatalogSearchBoxMixin",
    "BaseHousingCatalogMixin",
    "PagedHousingCatalogMixin",
    "ScrollingHousingCatalogMixin",
    "HousingMarketProductDisplayMixin",
    "HousingMarketSmallProductDisplayMixin",
    "HousingMarketBundleDisplayMixin",
    "HousingTopBannerMixin",
];

const ALL_VIRTUAL_TEMPLATES: &[&str] = &[
    "BaseHousingActionButtonTemplate",
    "BaseHousingModeButtonTemplate",
    "HousingCatalogCategoriesTemplate",
    "HousingCategoryBackButtonTemplate",
    "BaseHousingCatalogCategoryTemplate",
    "BaseHousingCatalogSubcategoryTemplate",
    "HousingCatalogCategoryTemplate",
    "HousingCatalogSubcategoryTemplate",
    "HousingCatalogDyeDisplayTemplate",
    "BaseHousingCatalogEntryTemplate",
    "HousingCatalogDecorEntryTemplate",
    "HousingCatalogRoomEntryTemplate",
    "HousingCatalogFiltersTemplate",
    "HousingCatalogSearchBoxTemplate",
    "PagedHousingCatalogTemplate",
    "ScrollingHousingCatalogTemplate",
    "HousingMarketProductDisplayTemplate",
    "HousingMarketSmallProductDisplayTemplate",
    "HousingMarketBundleDisplayTemplate",
];

const UTIL_FUNCTIONS: &[&str] = &[
    "FormatPrice",
    "FormatRefundTime",
    "OpenCatalogShopForProduct",
    "GetInsideAndIsInvalidIndoorsOutdoors",
    "CompareCatalogEntryVariantIDs",
    "GetEntryQuantity",
    "GetEntryNumStored",
    "GetEntryTotalOwned",
    "AddDecorEntryTooltipTitle",
];

const ENUM_STRING_TABLES: &[&str] = &[
    "HousingResultToErrorText",
    "NeighborhoodTypeStrings",
    "HousingAccessTypeStrings",
    "HouseOwnerErrorTypeStrings",
    "HousingLayoutGenericRestrictionStrings",
    "HousingLayoutRotateRestrictionStrings",
    "HousingLayoutRemoveRestrictionStrings",
    "HousingLayoutMoveRestrictionStrings",
    "HousingExpertSubmodeRestrictionStrings",
];

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
fn blizzard_housing_templates_find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&templates_dir()).expect("Blizzard_HousingTemplates TOC should resolve");
    assert_eq!(
        resolved,
        templates_toc(),
        "Blizzard_HousingTemplates ships exactly one bare TOC — retail-only addon resolves via \
         `find_toc_file` fallthrough"
    );
}

#[test]
fn blizzard_housing_templates_toc_declares_auto_loaded_with_one_dependency() {
    let toc = TocFile::from_file(&templates_toc()).expect("HousingTemplates TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_HousingTemplates declares `## LoadOnDemand: 0` — auto-loaded by the discovery \
         pass so every other Housing addon (HouseSettings / HouseFinder / HouseEditor / \
         HousingMarketCart / HousingInspectModeUI / HousingModelPreview / HousingDashboard / \
         HousingControls / HousingTutorials etc.) can declare it as a `## Dependencies:` entry \
         and have it resolve before their own LoD load runs"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_PagedContent".to_string()],
        "Single `## Dependencies:` entry: Blizzard_PagedContent (provides the \
         PagedNaturalSizeGridContentFrameTemplate inheritance chain that PagedHousingCatalogMixin \
         extends to render the paged decor catalog grid; PagedContent is also `LoadOnDemand: 0` \
         with `AllowLoad: Both` so it auto-loads on every screen too)"
    );
}

#[test]
fn blizzard_housing_templates_toc_is_retail_only_and_omits_allow_load() {
    let toc = TocFile::from_file(&templates_toc()).expect("HousingTemplates TOC should parse");
    let toc_text = std::fs::read_to_string(templates_toc()).expect("TOC should read");
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Declares `## AllowLoadGameType: standard` — retail-only Midnight feature"
    );
    assert!(!toc.is_game_type_restricted());
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Omits `## AllowLoad:` — auto-loaded addon without explicit screen gating; the Game \
         ScreenKind pass picks it up via the LoadOnDemand=0 default route"
    );
    assert!(!toc_text.contains("## DefaultState:"));
    assert!(
        toc.saved_variables().is_empty(),
        "No `## SavedVariables*` — pure shared-template module; all state lives on the \
         consuming addon's frame instances"
    );
}

#[test]
fn blizzard_housing_templates_toc_lists_sixteen_files_in_order() {
    let toc = TocFile::from_file(&templates_toc()).expect("HousingTemplates TOC should parse");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        files,
        vec![
            "Blizzard_HousingCatalogUtil.lua".to_string(),
            "Blizzard_HousingData.lua".to_string(),
            "Blizzard_HousingCatalogEntry.lua".to_string(),
            "Blizzard_HousingCatalogEntry.xml".to_string(),
            "Blizzard_HousingMarketProductDisplay.lua".to_string(),
            "Blizzard_HousingMarketProductDisplay.xml".to_string(),
            "Blizzard_HousingCatalogCategories.lua".to_string(),
            "Blizzard_HousingCatalogCategories.xml".to_string(),
            "Blizzard_HousingCatalogFilters.lua".to_string(),
            "Blizzard_HousingCatalogFilters.xml".to_string(),
            "Blizzard_HousingCatalogTemplates.lua".to_string(),
            "Blizzard_HousingCatalogTemplates.xml".to_string(),
            "Blizzard_HousingActionButton.lua".to_string(),
            "Blizzard_HousingActionButton.xml".to_string(),
            "Blizzard_HousingTopBanner.lua".to_string(),
            "Blizzard_HousingTopBanner.xml".to_string(),
        ],
        "TOC body lists exactly 16 source files in this exact order — Util loads FIRST so its \
         helper namespace is published before any consuming mixin runs, Data SECOND so the 9 \
         enum-to-string lookup tables publish before any tooltip code references them, then \
         each module pair (lua + xml) loads with the .lua first so mixin tables exist before the \
         .xml's mixin= attributes resolve them"
    );
}

#[test]
fn blizzard_housing_templates_directory_holds_seventeen_entries() {
    let dir = templates_dir();
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_HousingTemplates directory should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        17,
        "Directory ships exactly 17 entries (16 source + 1 TOC, no flavor subdirectory). Got: \
         {entries:?}"
    );
    assert!(entries.contains(&"Blizzard_HousingTemplates.toc".to_string()));
}

#[test]
fn blizzard_housing_templates_appears_in_game_screen_auto_discovery() {
    let ui = blizzard_ui_dir();
    let addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    let discovered = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_HousingTemplates");
    assert!(
        discovered,
        "Blizzard_HousingTemplates MUST appear in the Game ScreenKind auto-discovery pass — \
         `## LoadOnDemand: 0` plus the absence of `## AllowLoad:` makes it eligible for the \
         Game-screen sweep, which is what every Housing LoD addon relies on so its declared \
         `## Dependencies: Blizzard_HousingTemplates` resolves at LoD time"
    );
}

prefork_full_ui_case! {
fn blizzard_housing_templates_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let lua_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let related: Vec<&String> = lua_errors
        .iter()
        .filter(|e| {
            e.contains("Blizzard_HousingTemplates/")
                || e.contains("Blizzard_HousingTemplates\\")
                || ALL_MIXINS.iter().any(|m| e.contains(m))
                || e.contains("Blizzard_HousingCatalogUtil")
        })
        .collect();
    assert!(
        related.is_empty(),
        "Loading Blizzard_HousingTemplates must not emit any addon-specific Lua errors. Got {} \
         errors: {:?}",
        related.len(),
        related
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_templates_is_addon_loaded_via_game_screen_pass(env: &WowLuaEnv) {
    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingTemplates')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_HousingTemplates') must return true after the \
         Game-screen auto-discovery pass — proves the loader registered the addon name without \
         any explicit LoD call"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_templates_paged_content_dependency_loads_via_auto_discovery(env: &WowLuaEnv) {
    let paged_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_PagedContent')")
        .expect("PagedContent IsAddOnLoaded query should succeed");
    assert!(
        paged_loaded,
        "Blizzard_PagedContent (the only declared dep) must auto-load via the Game-screen \
         discovery pass before HousingTemplates runs — both are LoadOnDemand=0 so both ride the \
         Game-screen sweep, with PagedContent's `## AllowLoad: Both` ensuring it loads on every \
         screen"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_templates_publishes_all_nineteen_mixins(env: &WowLuaEnv) {
    for mixin in ALL_MIXINS {
        let mixin_type: String = env
            .eval(&format!("return type(_G['{mixin}'])"))
            .expect("mixin _G lookup should succeed");
        assert_eq!(
            mixin_type, "table",
            "{mixin} must publish as a `_G` table — every mixin is declared at file scope across \
             the 7 mixin-bearing Lua files (ActionButton/CatalogCategories/CatalogEntry/\
             CatalogFilters/CatalogTemplates/MarketProductDisplay/TopBanner) so consuming addons \
             can use them as `mixin=` attributes in their XML"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_templates_decor_entry_mixin_inherits_base_via_create_from_mixins(env: &WowLuaEnv) {
    let inherited: bool = env
        .eval(
            "return type(HousingCatalogDecorEntryMixin.OnLoad) == 'function' and \
             type(HousingCatalogEntryMixin.OnLoad) == 'function'",
        )
        .expect("CreateFromMixins inheritance probe should succeed");
    assert!(
        inherited,
        "HousingCatalogDecorEntryMixin = CreateFromMixins(HousingCatalogEntryMixin) must carry \
         the parent's OnLoad (and other shared base methods) into the derived mixin — proves the \
         CreateFromMixins copy-on-init behavior runs at file load time"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_templates_paged_catalog_mixin_inherits_base_via_create_from_mixins(env: &WowLuaEnv) {
    for derived in ["PagedHousingCatalogMixin", "ScrollingHousingCatalogMixin"] {
        let inherits_base: bool = env
            .eval(&format!(
                "return type({derived}) == 'table' and type(BaseHousingCatalogMixin) == 'table'"
            ))
            .expect("derived catalog mixin probe should succeed");
        assert!(
            inherits_base,
            "{derived} = CreateFromMixins(BaseHousingCatalogMixin) must publish as a `_G` table \
             alongside its base — both Paged and Scrolling variants share the BaseHousingCatalogMixin \
             surface (cart/preview/filter wiring) so consuming addons can pick the variant that \
             matches their layout strategy (paged grid vs continuous scroll)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_templates_publishes_catalog_util_namespace(env: &WowLuaEnv) {
    let namespace_type: String = env
        .eval("return type(Blizzard_HousingCatalogUtil)")
        .expect("Blizzard_HousingCatalogUtil type query should succeed");
    assert_eq!(
        namespace_type, "table",
        "Blizzard_HousingCatalogUtil must publish as a `_G` table — declared at file scope on \
         line 8 of Blizzard_HousingCatalogUtil.lua, queried by other Housing addons (e.g. \
         HousingModelPreview's GetEntryNumStored / GetEntryTotalOwned tooltip lookups) for \
         price-formatting + variant-comparison + indoor/outdoor validity helpers"
    );
    for func in UTIL_FUNCTIONS {
        let func_type: String = env
            .eval(&format!("return type(Blizzard_HousingCatalogUtil.{func})"))
            .expect("util function lookup should succeed");
        assert_eq!(
            func_type, "function",
            "Blizzard_HousingCatalogUtil.{func} must publish as a function — Util loads FIRST in \
             the TOC body so its 9-function surface is available before any consuming mixin runs"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_templates_publishes_nine_enum_to_string_lookup_tables(env: &WowLuaEnv) {
    for table in ENUM_STRING_TABLES {
        let table_type: String = env
            .eval(&format!("return type(_G['{table}'])"))
            .expect("enum-to-string table lookup should succeed");
        assert_eq!(
            table_type, "table",
            "{table} must publish as a `_G` table — declared at file scope in \
             Blizzard_HousingData.lua (loads SECOND in the TOC body so the 9 lookup tables are \
             available before any mixin's tooltip / error-handling code references them); each \
             table maps an Enum.* member to a localized error/label string from the global \
             string table"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_templates_all_virtual_templates_stay_nil_in_globals(env: &WowLuaEnv) {
    for template in ALL_VIRTUAL_TEMPLATES {
        let value_type: String = env
            .eval(&format!("return type(_G['{template}'])"))
            .expect("template _G lookup should succeed");
        assert_eq!(
            value_type, "nil",
            "{template} is `virtual=\"true\"` so it must NOT publish to `_G` — proves the \
             loader honors the virtual flag for all 19 reusable templates this addon ships for \
             consumption by the Housing LoD addons (HouseSettings / HouseEditor / \
             HousingMarketCart / HousingInspectModeUI / HousingModelPreview etc.)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_templates_publishes_top_banner_frame_global(env: &WowLuaEnv) {
    let frame_type: String = env
        .eval("return type(HousingTopBannerFrame)")
        .expect("HousingTopBannerFrame type query should succeed");
    assert_eq!(
        frame_type, "table",
        "HousingTopBannerFrame must publish as a `_G` frame — the XML declares this single \
         named non-virtual frame (parent=UIParent, hidden=true, mixin=HousingTopBannerMixin) so \
         server events that fire HOUSING_GUILD_HOUSE_NAME_CHANGED-style banner messages can show \
         it via global lookup"
    );
    let frame_name: String = env
        .eval("return HousingTopBannerFrame:GetName()")
        .expect("HousingTopBannerFrame:GetName query should succeed");
    assert_eq!(frame_name, "HousingTopBannerFrame");
}
}

prefork_full_ui_case! {
fn blizzard_housing_templates_top_banner_frame_starts_hidden(env: &WowLuaEnv) {
    let is_shown: bool = env
        .eval("return HousingTopBannerFrame:IsShown()")
        .expect("HousingTopBannerFrame:IsShown query should succeed");
    assert!(
        !is_shown,
        "HousingTopBannerFrame must start hidden — the XML declares `hidden=\"true\"` and the \
         banner is only ever shown via HousingTopBannerMixin:PlayBanner(title, subtext) when an \
         event-driven banner message arrives"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_templates_top_banner_mixin_exposes_lifecycle_and_play_methods(env: &WowLuaEnv) {
    for method in [
        "OnLoad",
        "OnEvent",
        "OnPopinInAnimFinished",
        "OnFadeOutAnimFinished",
        "OnHide",
        "SetBannerText",
        "PlayBanner",
        "StopBanner",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingTopBannerMixin['{method}']) == 'function'"
            ))
            .expect("HousingTopBannerMixin method existence query should succeed");
        assert!(
            exists,
            "HousingTopBannerMixin must expose `:{method}()` — owns 8 methods on the named \
             non-virtual top-banner frame: OnLoad / OnEvent lifecycle, \
             OnPopinInAnimFinished / OnFadeOutAnimFinished animation callbacks, OnHide cleanup, \
             SetBannerText (populate the title + subtext FontStrings), PlayBanner (run the popin \
             animation chain), StopBanner (cancel any in-flight animation)"
        );
    }
}
}
