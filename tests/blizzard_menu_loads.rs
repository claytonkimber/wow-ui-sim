#![cfg(feature = "client-retail")]
use std::path::PathBuf;


use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::paths::default_blizzard_ui_addons_path;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> std::path::PathBuf {
    default_blizzard_ui_addons_path().expect("Blizzard UI cache should be synced")

}

fn menu_dir() -> std::path::PathBuf {
    blizzard_ui_dir().join("Blizzard_Menu")
}

fn menu_toc() -> std::path::PathBuf {
    menu_dir().join("Blizzard_Menu.toc")
}

const MENU_TOC_RESOLVED_FILES_RETAIL: &[&str] = &[
    "MenuConstants.lua",
    "Mainline/MenuConstants.lua",
    "MenuVariants.lua",
    "Mainline/MenuVariants.lua",
    "Compositor.lua",
    "Menu.lua",
    "Menu.xml",
    "DropdownButton.lua",
    "DropdownButton.xml",
    "MenuTemplates.lua",
    "MenuTemplates.xml",
    "Mainline/MenuTemplates.lua",
    "Mainline/MenuTemplates.xml",
    "MenuUtil.lua",
];

const MENU_NAMESPACE_TABLES: &[&str] = &[
    "MenuConstants",
    "MenuResponse",
    "MenuInputContext",
    "MenuCloseReason",
    "MenuVariants",
    "MenuTemplates",
    "MenuUtil",
    "Menu",
];

const SHARED_BEHAVIOR_MIXINS: &[&str] = &[
    "DropdownButtonMixin",
    "DropdownButtonProxyMixin",
    "CompositorMixin",
    "DropdownTextMixin",
    "DropdownSelectionTextMixin",
    "WowDropdownFilterBehaviorMixin",
    "WowFilterButtonMixin",
    "MenuStyleMixin",
    "RandomColorStyleMenuMixin",
    "BlackColorStyleMenuMixin",
    "MenuStyle2Mixin",
    "WowStyle2DropdownMixin",
    "WowStyle2IconButtonMixin",
];

const RETAIL_DROPDOWN_MIXINS: &[&str] = &[
    "WowStyle1DropdownMixin",
    "WowStyle1FilterDropdownMixin",
    "WowStyle1ArrowDropdownMixin",
    "MenuStyle1Mixin",
];

const MENU_UTIL_PUBLIC_FUNCTIONS: &[&str] = &[
    "TraverseMenu",
    "GetSelections",
    "ShowTooltip",
    "HideTooltip",
    "HookTooltipScripts",
    "CreateRootMenuDescription",
    "CreateContextMenu",
    "CreateFrame",
    "CreateTemplate",
    "CreateTitle",
    "CreateButton",
    "CreateCheckbox",
    "CreateRadio",
    "CreateColorSwatch",
    "CreateButtonMenu",
    "CreateButtonContextMenu",
    "CreateCheckboxMenu",
    "CreateCheckboxContextMenu",
    "CreateRadioMenu",
    "CreateRadioContextMenu",
];

const MENU_PUBLIC_FUNCTIONS: &[&str] = &[
    "GetManager",
    "CreateRootMenuDescription",
    "CreateMenuElementDescription",
    "PopulateDescription",
    "ModifyMenu",
    "GetOpenMenuTags",
    "PrintOpenMenuTags",
];

const MENU_RESPONSE_VALUES: &[(&str, i64)] =
    &[("Open", 1), ("Refresh", 2), ("Close", 3), ("CloseAll", 4)];

const RETAIL_FILTER_DROPDOWN_ATLAS_NAMES: &[&str] = &[
    "WowStyle1FilterDropdownStateDownOver",
    "WowStyle1FilterDropdownStateOver",
    "WowStyle1FilterDropdownStateDown",
    "WowStyle1FilterDropdownStateOpen",
    "WowStyle1FilterDropdownStateEnabled",
    "WowStyle1FilterDropdownStateDisabled",
];

const VIRTUAL_TEMPLATES_BASE: &[&str] = &[
    "MenuTemplateBase",
    "WowStyle2DropdownTemplate",
    "WowStyle2IconButtonTemplate",
    "WowMenuAutoHideButtonTemplate",
    "WowMenuDropdownHighlightButtonTemplate",
];

const VIRTUAL_TEMPLATES_RETAIL: &[&str] = &[
    "WowStyle1DropdownTemplate",
    "WowStyle1ArrowDropdownTemplate",
    "WowStyle1FilterDropdownTemplate",
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
fn blizzard_menu_find_toc_resolves_bare_variant() {
    let resolved = find_toc_file(&menu_dir()).expect("Blizzard_Menu TOC should resolve");
    assert_eq!(
        resolved,
        menu_toc(),
        "Blizzard_Menu ships exactly one bare TOC. The flavor split is handled inline within the \
         single TOC body via `[Family]\\` / `[AllowLoadGameType ...]` per-line annotations \
         (Mainline / Cata / Vanilla files all share one TOC), so no flavor-suffixed TOC variants \
         exist — `find_toc_file` falls through to the bare `.toc` after the `_Mainline.toc` \
         lookup misses"
    );
}

#[test]
fn blizzard_menu_toc_declares_load_first_eager_with_shared_xml_base_dep() {
    let toc = TocFile::from_file(&menu_toc()).expect("Blizzard_Menu TOC parses");
    assert!(
        toc.is_load_first(),
        "Blizzard_Menu declares `## LoadFirst: 1` — the menu / dropdown infrastructure must be \
         live before any addon (or any other Blizzard addon) attempts to instantiate \
         DropdownButton-derived frames or call MenuUtil.* / Menu.* / MenuTemplates.*. The \
         LoadFirst flag triggers the eager pre-pass walk before the dependency-graph load loop"
    );
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_Menu does NOT declare `## LoadOnDemand:` — it's eager-load (combined with \
         LoadFirst, the dropdown infra is foundational and must be ready before any consumer)"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_Menu does NOT declare `## UseSecureEnvironment` — runs in the standard Lua \
         environment. Note: per-element security is enforced internally via SecureTypes \
         (CreateSecureMap / CreateSecureArray / CreateSecureNumber / CreateSecureFunction at \
         Menu.lua:1-4), but the addon as a whole is loaded into the normal environment"
    );
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_SharedXMLBase".to_string()],
        "Blizzard_Menu declares exactly one `## Dependencies: Blizzard_SharedXMLBase`. \
         SharedXMLBase publishes the foundational primitives the menu surface consumes: \
         CreateAnchor (used by MenuVariants.GearButtonAnchor / CancelButtonAnchor), \
         CallbackRegistryMixin (DropdownButtonMixin extends it), ButtonStateBehaviorMixin \
         (WowStyle1/2 dropdown / icon button mixins extend it), ProxyUtil (CreateProxy / \
         CreateProxyMixin / SetPrivateReference / ReleasePrivateReference at Menu.lua:5-9), \
         SecureTypes (CreateSecureMap / Array / Function / Number at Menu.lua:1-4), \
         ResizeLayoutFrame (MenuTemplateBase inherits)"
    );
    assert!(toc.optional_deps().is_empty());
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_Menu declares NO `## SavedVariables*` — menu state is per-session ephemeral \
         (open menus close on screen change, descriptions are reconstructed from generators on \
         every CreateContextMenu call)"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_Menu omits the top-level `## AllowLoadGameType:` directive — the addon is \
         universal across every flavor. Per-line `[AllowLoadGameType ...]` annotations gate \
         individual file entries (Cata\\MenuVariants.lua only loads on cata/mists, \
         Vanilla\\MenuVariants.lua only on vanilla/tbc/wrath, [Family]\\MenuVariants.lua on \
         mainline), but the addon as a whole loads everywhere"
    );
}

#[test]
fn blizzard_menu_toc_declares_allow_load_both_screen_routing() {
    let toc = TocFile::from_file(&menu_toc()).expect("Blizzard_Menu TOC parses");
    let raw = std::fs::read_to_string(menu_toc()).expect("Blizzard_Menu TOC reads");
    assert!(
        raw.contains("## AllowLoad: Both"),
        "Blizzard_Menu must declare `## AllowLoad: Both` exactly — every screen needs the menu \
         infrastructure (login screen language picker, character-select realm dropdown, \
         character-create class / race / customization dropdowns, in-game UI menus). The \
         capitalized `Both` literal normalizes through `eq_ignore_ascii_case` at src/toc.rs:307"
    );
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            toc.allows_screen(screen),
            "Blizzard_Menu must allow {screen:?} — `## AllowLoad: Both` makes `allows_screen` \
             return true for every ScreenKind. The dropdown / context-menu surface is shared \
             infrastructure consumed on every screen"
        );
    }
}

#[test]
fn blizzard_menu_toc_resolves_family_and_game_type_annotations_for_retail() {
    let toc = TocFile::from_file(&menu_toc()).expect("Blizzard_Menu TOC parses");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        files, MENU_TOC_RESOLVED_FILES_RETAIL,
        "Blizzard_Menu TOC body resolves to exactly 14 files in retail mode. The TOC uses 2 \
         template-style placeholders that the simulator's parser substitutes at parse time: \
         `[Family]\\` → `Mainline/` (src/toc.rs:145, retail flavor) and `[AllowLoadGameType ...]` \
         per-line annotations gate-keep individual entries — `Cata\\MenuVariants.lua \
         [AllowLoadGameType cata, mists]` and `Vanilla\\MenuVariants.lua [AllowLoadGameType \
         vanilla, tbc, wrath]` are filtered out via `is_allowed_game_type` at src/toc.rs:141 \
         (mainline isn't in either of those lists), while `[Family]\\MenuVariants.lua \
         [AllowLoadGameType mainline]` resolves to `Mainline/MenuVariants.lua` and survives the \
         gate. The classic-only files do NOT appear in the retail file list, but the [Family] \
         pair (MenuConstants / MenuVariants / MenuTemplates lua + xml) does"
    );
}

#[test]
fn blizzard_menu_directory_holds_eight_top_level_entries_plus_flavor_subdirs() {
    let dir = menu_dir();
    let mut entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_Menu directory reads")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec![
            "11_0_0_MenuImplementationGuide.lua".to_string(),
            "Blizzard_Menu.toc".to_string(),
            "Cata".to_string(),
            "Classic".to_string(),
            "Compositor.lua".to_string(),
            "DropdownButton.lua".to_string(),
            "DropdownButton.xml".to_string(),
            "Mainline".to_string(),
            "Menu.lua".to_string(),
            "Menu.xml".to_string(),
            "MenuConstants.lua".to_string(),
            "MenuTemplates.lua".to_string(),
            "MenuTemplates.xml".to_string(),
            "MenuUtil.lua".to_string(),
            "MenuVariants.lua".to_string(),
            "Vanilla".to_string(),
        ],
        "Blizzard_Menu directory ships 16 entries — 4 flavor subdirectories (Cata / Classic / \
         Mainline / Vanilla, each holding flavor-specific MenuConstants / MenuVariants / \
         MenuTemplates), 1 TOC, 1 documentation lua (11_0_0_MenuImplementationGuide.lua — NOT \
         listed in the TOC body, intentionally excluded as documentation-only), 7 cross-flavor \
         lua files (Compositor / DropdownButton / Menu / MenuConstants / MenuTemplates / \
         MenuUtil / MenuVariants), and 3 cross-flavor XML files (DropdownButton / Menu / \
         MenuTemplates)"
    );
}

#[test]
fn blizzard_menu_appears_on_every_screen_auto_discovery() {
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let count = addons
            .iter()
            .filter(|(name, _)| name == "Blizzard_Menu")
            .count();
        assert_eq!(
            count, 1,
            "Blizzard_Menu must auto-discover EXACTLY ONCE on {screen:?} — `## AllowLoad: Both` \
             plus a single bare TOC routes the addon into every screen's discovery sweep without \
             flavor-variant duplication. (Screen tested: {screen:?})"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_menu_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_Menu/")
                || message.contains("Blizzard_Menu\\")
                || message.contains("MenuConstants.lua")
                || message.contains("MenuVariants.lua")
                || message.contains("MenuTemplates.lua")
                || message.contains("MenuUtil.lua")
                || message.contains("Compositor.lua")
                || message.contains("DropdownButton.lua")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_Menu emitted addon-specific Lua errors during Game-screen auto-load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_menu_is_addon_loaded_returns_true_after_load_first_pass(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_Menu')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_Menu') must return true after the eager LoadFirst \
         pre-pass — proves the menu infrastructure registers with the loaded-set before the \
         normal dependency-graph load loop, ensuring downstream consumers find the dropdown \
         intrinsic / Menu.* / MenuUtil.* surface live by the time their own load runs"
    );
}
}

prefork_full_ui_case! {
fn blizzard_menu_publishes_eight_namespace_tables(env: &WowLuaEnv) {

    for table in MENU_NAMESPACE_TABLES {
        let kind: String = env
            .eval(&format!("return type(_G['{table}'])"))
            .expect("namespace table probe succeeds");
        assert_eq!(
            kind, "table",
            "`{table}` must publish at `_G` as a table after Blizzard_Menu loads. The 8 namespace \
             tables are the addon's public API roots: MenuConstants (4 enum-like fields), \
             MenuResponse (Open=1 / Refresh=2 / Close=3 / CloseAll=4 — return values for \
             description handlers driving menu lifecycle), MenuInputContext (None=1 / \
             MouseButton=2 / MouseWheel=3 — passed to element handlers), MenuCloseReason \
             (Unspecified=1 / CloseAll=2), MenuVariants (overridable per-flavor texture / sound \
             helpers), MenuTemplates (cross-flavor template helpers — CreateSelectionTextures, \
             CreateDivider, CreateSpacer, etc.), MenuUtil (the high-level public API — \
             CreateContextMenu, CreateRootMenuDescription, CreateButton/Checkbox/Radio, etc.), \
             and Menu (the manager API — Menu.GetManager / ModifyMenu / PopulateDescription)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_menu_publishes_thirteen_shared_behavior_mixins(env: &WowLuaEnv) {

    for mixin in SHARED_BEHAVIOR_MIXINS {
        let kind: String = env
            .eval(&format!("return type(_G['{mixin}'])"))
            .expect("shared behavior mixin probe succeeds");
        assert_eq!(
            kind, "table",
            "`{mixin}` must publish at `_G` as a mixin table after Blizzard_Menu loads. The 13 \
             shared (cross-flavor) mixins span: DropdownButtonMixin (DropdownButton.lua:1, \
             CreateFromMixins(CallbackRegistryMixin) — drives the intrinsic dropdown widget), \
             DropdownButtonProxyMixin (DropdownButton.lua:397, the proxy-pattern indirection), \
             CompositorMixin (Compositor.lua:301, secure / proxy composition), \
             DropdownTextMixin (MenuTemplates.lua:518), \
             DropdownSelectionTextMixin (MenuTemplates.lua:570, extends DropdownTextMixin), \
             WowDropdownFilterBehaviorMixin (MenuTemplates.lua:689), WowFilterButtonMixin \
             (MenuTemplates.lua:749, extends WowDropdownFilterBehaviorMixin), MenuStyleMixin \
             (MenuTemplates.lua:908), RandomColorStyleMenuMixin / BlackColorStyleMenuMixin / \
             MenuStyle2Mixin (MenuTemplates.lua:945/956/965, all extend MenuStyleMixin), \
             WowStyle2DropdownMixin / WowStyle2IconButtonMixin (MenuTemplates.lua:840/989, \
             cross-flavor Style2 dropdown variants extending ButtonStateBehaviorMixin)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_menu_publishes_four_retail_specific_dropdown_mixins(env: &WowLuaEnv) {

    for mixin in RETAIL_DROPDOWN_MIXINS {
        let kind: String = env
            .eval(&format!("return type(_G['{mixin}'])"))
            .expect("retail dropdown mixin probe succeeds");
        assert_eq!(
            kind, "table",
            "`{mixin}` must publish at `_G` as a retail-specific mixin table. \
             WowStyle1DropdownMixin (MenuTemplates.lua:761, extends ButtonStateBehaviorMixin + \
             DropdownSelectionTextMixin), WowStyle1FilterDropdownMixin (MenuTemplates.lua:784, \
             extends ButtonStateBehaviorMixin + DropdownTextMixin + WowFilterButtonMixin), and \
             WowStyle1ArrowDropdownMixin (MenuTemplates.lua:900, extends \
             ButtonStateBehaviorMixin) are declared in the cross-flavor MenuTemplates.lua but \
             instantiated through retail-only XML templates (Mainline/MenuTemplates.xml). \
             MenuStyle1Mixin (Mainline/MenuTemplates.lua:51) is retail-only — it overrides \
             MenuVariants.GetDefaultMenuMixin / GetDefaultContextMenuMixin to return \
             MenuStyle1Mixin in retail (Mainline/MenuVariants.lua:1-7) where the cross-flavor \
             stubs at MenuVariants.lua:76-82 raise `Requires implementation in game specific \
             version` errors"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_menu_response_table_carries_canonical_lifecycle_values(env: &WowLuaEnv) {

    for (key, expected) in MENU_RESPONSE_VALUES {
        let actual: i64 = env
            .eval(&format!("return MenuResponse['{key}']"))
            .expect("MenuResponse value probe succeeds");
        assert_eq!(
            actual, *expected,
            "MenuResponse.{key} must equal {expected} — MenuConstants.lua:17-23 declares \
             MenuResponse with 4 canonical lifecycle values returned from element description \
             handlers: Open=1 (menu remains open and unchanged — the no-op default), \
             Refresh=2 (all frames in the menu are reinitialized — common for checkboxes / \
             radios that visually update on toggle), Close=3 (parent menus remain open but \
             this menu closes — common for one-shot button activations inside nested submenus), \
             CloseAll=4 (close the entire menu chain). Element callbacks drive their parent \
             menu's lifecycle by returning one of these 4 values"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_menu_constants_table_exposes_layout_direction_enum_values(env: &WowLuaEnv) {

    for (key, expected) in [
        ("VerticalLinearDirection", 1),
        ("VerticalGridDirection", 2),
        ("HorizontalGridDirection", 3),
    ] {
        let actual: i64 = env
            .eval(&format!("return MenuConstants['{key}']"))
            .expect("MenuConstants value probe succeeds");
        assert_eq!(
            actual, expected,
            "MenuConstants.{key} must equal {expected} — MenuConstants.lua:1-9 declares the \
             menu layout-direction enum with 3 modes: VerticalLinearDirection (one column \
             top-to-bottom — the default dropdown / context menu shape), VerticalGridDirection \
             (multi-column stacked top-to-bottom in column-major order), HorizontalGridDirection \
             (multi-row arranged left-to-right in row-major order — used by the icon-grid menu \
             variants). Plus AutoCalculateColumns=nil (sentinel) / ElementPollFrequencySeconds=0.2 \
             / PrintSecure=false in the same table"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_menu_util_exposes_twenty_public_factory_functions(env: &WowLuaEnv) {

    for func in MENU_UTIL_PUBLIC_FUNCTIONS {
        let kind: String = env
            .eval(&format!("return type(MenuUtil['{func}'])"))
            .expect("MenuUtil function probe succeeds");
        assert_eq!(
            kind, "function",
            "MenuUtil.{func} must publish as a function. MenuUtil.lua declares 20+ public \
             factory functions — the high-level menu-building API: traversal helpers \
             (TraverseMenu / GetSelections), tooltip integration (ShowTooltip / HideTooltip / \
             HookTooltipScripts), description constructors (CreateRootMenuDescription / \
             CreateContextMenu / CreateFrame / CreateTemplate), element constructors (CreateTitle \
             / CreateButton / CreateCheckbox / CreateRadio / CreateColorSwatch), and the \
             multi-element shortcuts (CreateButtonMenu / CreateButtonContextMenu / \
             CreateCheckboxMenu / CreateCheckboxContextMenu / CreateRadioMenu / \
             CreateRadioContextMenu) that wrap a generator pattern around the description \
             builders to publish complete menus in one call"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_menu_namespace_exposes_seven_manager_functions(env: &WowLuaEnv) {

    for func in MENU_PUBLIC_FUNCTIONS {
        let kind: String = env
            .eval(&format!("return type(Menu['{func}'])"))
            .expect("Menu manager function probe succeeds");
        assert_eq!(
            kind, "function",
            "Menu.{func} must publish as a function. Menu.lua publishes 7 manager-tier public \
             functions: Menu.GetManager (returns the singleton MenuMixin instance — opens / \
             closes / tracks the active menu chain), Menu.CreateRootMenuDescription (builds a \
             RootMenuDescriptionMixin entry — the head of a menu hierarchy carrying a menuMixin \
             style binding), Menu.CreateMenuElementDescription (builds a leaf \
             MenuElementDescriptionMixin entry), Menu.PopulateDescription (drives the \
             menuGenerator callback to fill an empty description), Menu.ModifyMenu (registers a \
             tag-keyed callback fired via EventRegistry's `Menu.OpenMenuTag` event for \
             cross-addon menu modification — the addon-extensibility hook), Menu.GetOpenMenuTags \
             / Menu.PrintOpenMenuTags (debug introspection)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_menu_retail_filter_dropdown_atlas_constants_publish_as_strings(env: &WowLuaEnv) {

    for atlas in RETAIL_FILTER_DROPDOWN_ATLAS_NAMES {
        let kind: String = env
            .eval(&format!("return type(_G['{atlas}'])"))
            .expect("retail filter atlas constant probe succeeds");
        assert_eq!(
            kind, "string",
            "`{atlas}` must publish at `_G` as a string atlas-key constant. \
             Mainline/MenuConstants.lua publishes 6 retail-only filter-dropdown atlas-key \
             string globals — the per-state texture lookup keys consumed by \
             WowStyle1FilterDropdownMixin's button-state-behavior dispatch: \
             *StateEnabled = `common-dropdown-b-button` (the resting / idle state), *StateOver = \
             `common-dropdown-b-button-hover` (mouse-over-while-closed), *StateDown = \
             `common-dropdown-b-button-pressed` (mouse-down-while-closed), *StateDownOver = \
             `common-dropdown-b-button-pressedhover` (down + hover), *StateOpen = \
             `common-dropdown-b-button-open` (menu-currently-shown), *StateDisabled = \
             `common-dropdown-b-button-disabled`. Each maps a button-state to an atlas key in \
             the common-dropdown texture sheet"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_menu_variants_overrides_menu_mixin_to_retail_style1(env: &WowLuaEnv) {

    let style1_active: bool = env
        .eval(
            "return MenuVariants.GetDefaultMenuMixin() == MenuStyle1Mixin \
             and MenuVariants.GetDefaultContextMenuMixin() == MenuStyle1Mixin",
        )
        .expect("MenuVariants default-mixin probe succeeds");
    assert!(
        style1_active,
        "MenuVariants.GetDefaultMenuMixin and GetDefaultContextMenuMixin must both return \
         MenuStyle1Mixin on retail. The cross-flavor MenuVariants.lua:76-82 stubs the two \
         functions to raise `Requires implementation in game specific version of \
         MenuVariants.lua`; Mainline/MenuVariants.lua:1-7 then overwrites both with closures \
         that return MenuStyle1Mixin. Order-dependence: MenuVariants.lua MUST execute before \
         Mainline/MenuVariants.lua, which it does since the TOC body lists `MenuVariants.lua` \
         on line 8 before `[Family]\\MenuVariants.lua` on line 11"
    );
}
}

prefork_full_ui_case! {
fn blizzard_menu_xml_registers_eight_virtual_templates_for_retail(env: &WowLuaEnv) {
    let _env = env;

    let mut all_templates: Vec<&str> = Vec::new();
    all_templates.extend_from_slice(VIRTUAL_TEMPLATES_BASE);
    all_templates.extend_from_slice(VIRTUAL_TEMPLATES_RETAIL);

    for template in all_templates {
        let registered = wow_ui_sim::xml::get_template(template).is_some();
        assert!(
            registered,
            "Virtual template `{template}` must register with the XML template registry. \
             Blizzard_Menu publishes 8 virtual templates across 3 XML files: Menu.xml declares \
             MenuTemplateBase (Frame mixin=MenuProxyMixin, inherits=ResizeLayoutFrame, \
             enableMouse=true, flattenRenderLayers=true, with KeyValues minimumElementWidth=50 \
             / ignoreAllChildren=true / skipChildLayout=true and OnLoad / OnUpdate / nop \
             OnMouseWheel scripts); MenuTemplates.xml declares 4 cross-flavor templates \
             (WowStyle2DropdownTemplate, WowStyle2IconButtonTemplate, \
             WowMenuAutoHideButtonTemplate (propagateMouseInput=Motion), \
             WowMenuDropdownHighlightButtonTemplate (inherits DarkMenuElementTemplate, \
             motionScriptsWhileDisabled=true)); Mainline/MenuTemplates.xml declares 3 \
             retail-only templates (WowStyle1DropdownTemplate, WowStyle1ArrowDropdownTemplate, \
             WowStyle1FilterDropdownTemplate — all DropdownButton-derived intrinsics consuming \
             the matching WowStyle1*Mixin)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_menu_dropdown_button_intrinsic_registers_with_template_lookup(env: &WowLuaEnv) {
    let _env = env;

    let registered = wow_ui_sim::xml::get_template("DropdownButton").is_some();
    assert!(
        registered,
        "Intrinsic widget `DropdownButton` must register with the XML template registry. \
         DropdownButton.xml:3 declares `<Button name=\"DropdownButton\" mixin=\"DropdownButtonMixin\" \
         intrinsic=\"true\">` — this is NOT a virtual=\"true\" template, it's an `intrinsic=\"true\"` \
         widget type that publishes a new XML element name (so other XML files can write \
         `<DropdownButton ... />` directly instead of `<Button inherits=\"DropdownButton\" />`). \
         The simulator stores intrinsics in the same template registry under their declared name, \
         so `get_template(\"DropdownButton\")` resolves the entry. The intrinsic carries 5 \
         KeyValues seeding the menu-anchor defaults (menuPoint=TOPLEFT / menuPointX=0 / \
         menuPointY=0 / menuRelativePoint=BOTTOMLEFT) plus 4 Intrinsic-suffixed script handlers \
         (OnLoad_Intrinsic / OnEnter_Intrinsic / OnMouseDown_Intrinsic / OnMouseWheel_Intrinsic) \
         that fire BEFORE the consumer's own scripts via the precompiled intrinsic-dispatch \
         pattern at src/loader/precompiled.rs:14-17"
    );
}
}

prefork_full_ui_case! {
fn blizzard_menu_util_create_function_helpers_alias_menu_templates(env: &WowLuaEnv) {

    let aliased: bool = env
        .eval(
            "return MenuUtil.CreateDivider == MenuTemplates.CreateDivider \
             and MenuUtil.CreateSpacer == MenuTemplates.CreateSpacer",
        )
        .expect("MenuUtil alias probe succeeds");
    assert!(
        aliased,
        "MenuUtil.CreateDivider must be the same function reference as \
         MenuTemplates.CreateDivider, and MenuUtil.CreateSpacer the same as \
         MenuTemplates.CreateSpacer. MenuUtil.lua:255-256 declares two reference aliases \
         (`MenuUtil.CreateDivider = MenuTemplates.CreateDivider; MenuUtil.CreateSpacer = \
         MenuTemplates.CreateSpacer;`) so the two namespaces share identical function pointers \
         for the divider / spacer factories — load-order constraint: MenuTemplates.lua must \
         execute BEFORE MenuUtil.lua, which it does (TOC body lines 17 vs 21)"
    );
}
}
