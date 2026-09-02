use std::path::PathBuf;

use wow_ui_sim::loader::load_addon;
use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn text_status_bar_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_TextStatusBar")
}

fn text_status_bar_toc() -> PathBuf {
    text_status_bar_dir().join("Blizzard_TextStatusBar.toc")
}

const ALL_FOUR_SCREENS: &[ScreenKind] = &[
    ScreenKind::Game,
    ScreenKind::Login,
    ScreenKind::CharacterSelect,
    ScreenKind::CharacterCreate,
];

const TEXT_STATUS_BAR_MIXIN_METHODS: &[&str] = &[
    "InitializeTextStatusBar",
    "SetBarText",
    "TextStatusBarOnEvent",
    "UpdateTextString",
    "GetNumericDisplay",
    "UpdateTextStringWithValues",
    "OnStatusBarEnter",
    "OnStatusBarLeave",
    "OnStatusBarValueChanged",
    "OnStatusBarMinMaxChanged",
    "SetBarTextPrefix",
    "SetBarTextZeroText",
    "ShowStatusBarText",
    "HideStatusBarText",
];

const SPARK_MIXIN_METHODS: &[&str] = &[
    "Initialize",
    "SetVisuals",
    "GetIsActive",
    "OnBarValuesUpdated",
    "UpdateShown",
    "UpdateSize",
];

const STATUS_TEXT_DISPLAY_MODES: &[&str] = &["NUMERIC", "PERCENT", "BOTH", "NONE"];

const MAINLINE_DEPENDENTS: &[&str] = &[
    "Blizzard_ActionBar",
    "Blizzard_UnitFrame",
    "Blizzard_SettingsDefinitions_Shared",
];

fn fresh_game_env() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);
    env.set_screen_mode(ScreenKind::Game);
    {
        let mut state = env.state().borrow_mut();
        state.addon_base_paths = vec![blizzard_ui_dir()];
    }
    wow_ui_sim::xml::register_intrinsic_templates();
    env
}

fn load_full_game_ui() -> WowLuaEnv {
    let env = fresh_game_env();

    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("[load {name}] FAILED: {err}"));
    }

    env.apply_post_load_workarounds();
    fire_startup_events_for_screen(&env, ScreenKind::Game);
    env
}

#[test]
fn find_toc_file_resolves_bare_toc() {
    let resolved = find_toc_file(&text_status_bar_dir()).expect("TextStatusBar TOC resolves");
    assert_eq!(
        resolved,
        text_status_bar_toc(),
        "Bare TOC — no flavor suffix; foundational shared template \
         addon ships universally"
    );
}

#[test]
fn toc_is_eager_with_single_shared_xml_dependency() {
    let toc = TocFile::from_file(&text_status_bar_toc()).expect("TOC parses");

    assert!(
        !toc.is_load_on_demand(),
        "No `## LoadOnDemand` — TextStatusBar publishes the \
         StatusBar virtual template consumed at template-registration \
         time by ActionBar/UnitFrame/etc., so it must load before any \
         dependent's XML is parsed"
    );
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_SharedXML".to_string()],
        "`## Dependencies: Blizzard_SharedXML` — single hard dep on \
         SharedXML for Settings.SetOnValueChangedCallback / \
         TextureKitConstants.UseAtlasSize. Got: {:?}",
        toc.dependencies()
    );
    assert!(toc.optional_deps().is_empty());
    assert!(toc.saved_variables().is_empty());
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert!(!toc.is_game_type_restricted());
    assert!(toc.default_enabled());
}

#[test]
fn allow_load_both_surfaces_on_all_four_screens() {
    let toc = TocFile::from_file(&text_status_bar_toc()).expect("TOC parses");

    for screen in ALL_FOUR_SCREENS {
        assert!(
            toc.allows_screen(*screen),
            "`## AllowLoad: Both` (case-insensitive at toc.rs:307) \
             must permit screen {screen:?} — text-status-bar templates \
             are consumed by frames on glue too (e.g. character-list \
             status indicators)"
        );
    }
}

#[test]
fn toc_raw_bytes_pin_five_metadata_directives() {
    let raw = std::fs::read_to_string(text_status_bar_toc()).expect("TOC reads utf-8");

    let expected_directives = [
        "## Title: Blizzard_TextStatusBar",
        "## Author: Blizzard Entertainment",
        "## DefaultState: enabled",
        "## Dependencies: Blizzard_SharedXML",
        "## AllowLoad: Both",
        "TextStatusBar.lua",
        "TextStatusBar.xml",
    ];

    for directive in expected_directives {
        assert!(
            raw.contains(directive),
            "Raw TOC must pin `{directive}` — 5 metadata directives + \
             2 body files (lua then xml)"
        );
    }

    assert!(!raw.contains("## LoadOnDemand"));
    assert!(!raw.contains("## RequiredDep"));
    assert!(!raw.contains("## OptionalDep"));
    assert!(!raw.contains("## AllowLoadGameType"));
    assert!(!raw.contains("## SavedVariables"));
    assert!(!raw.contains("## UseSecureEnvironment"));
    assert!(!raw.contains("## LoadFirst"));
}

#[test]
fn body_orders_lua_before_xml() {
    let toc = TocFile::from_file(&text_status_bar_toc()).expect("TOC parses");

    let body: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    assert_eq!(
        body,
        vec![
            "TextStatusBar.lua".to_string(),
            "TextStatusBar.xml".to_string(),
        ],
        "Body must be 2 entries in lua-then-xml order — Lua publishes \
         TextStatusBarMixin + TextStatusBarSparkMixin, then XML \
         registers the virtual templates that consume them. Got: {body:?}"
    );
}

#[test]
fn present_in_every_screen_eager_discovery() {
    for screen in ALL_FOUR_SCREENS {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), *screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_TextStatusBar");
        assert!(
            found,
            "Blizzard_TextStatusBar must appear in {screen:?} eager \
             discovery — AllowLoad: Both + no LoadOnDemand"
        );
    }
}

prefork_full_ui_case! {
fn status_text_display_mode_publishes_with_four_entries(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(STATUS_TEXT_DISPLAY_MODE)")
        .expect("STATUS_TEXT_DISPLAY_MODE probe");
    assert_eq!(
        kind, "table",
        "STATUS_TEXT_DISPLAY_MODE = table — TextStatusBar.lua line 24 \
         publishes the global mode-name enum consumed by the Settings \
         dropdown for status-text-display-mode"
    );

    for mode in STATUS_TEXT_DISPLAY_MODES {
        let value: String = env
            .eval(&format!("return STATUS_TEXT_DISPLAY_MODE.{mode}"))
            .unwrap_or_else(|err| panic!("STATUS_TEXT_DISPLAY_MODE.{mode} probe failed: {err}"));
        assert_eq!(
            value, *mode,
            "STATUS_TEXT_DISPLAY_MODE.{mode} must equal the literal \
             string \"{mode}\" — used as both key and value (string-id \
             pattern) since CVar serialization stores the mode name"
        );
    }
}
}

prefork_full_ui_case! {
fn text_status_bar_mixin_publishes_with_fourteen_methods(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(TextStatusBarMixin)")
        .expect("TextStatusBarMixin probe");
    assert_eq!(
        kind, "table",
        "TextStatusBarMixin = table — TextStatusBar.lua line 31 \
         publishes the mixin attached to the virtual TextStatusBar \
         StatusBar template at xml line 3"
    );

    for method in TEXT_STATUS_BAR_MIXIN_METHODS {
        let kind: String = env
            .eval(&format!("return type(TextStatusBarMixin['{method}'])"))
            .unwrap_or_else(|err| panic!("TextStatusBarMixin.{method} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "TextStatusBarMixin.{method} must be a function — covers \
             InitializeTextStatusBar (registers CVAR_UPDATE + Settings \
             callback), SetBarText/Prefix/ZeroText helpers, OnEnter/\
             OnLeave/OnValueChanged/OnMinMaxChanged script handlers, \
             ShowStatusBarText/HideStatusBarText visibility toggle, \
             UpdateTextString + UpdateTextStringWithValues + \
             GetNumericDisplay (the central NUMERIC/PERCENT/BOTH \
             rendering path)"
        );
    }
}
}

prefork_full_ui_case! {
fn spark_mixin_publishes_with_six_methods(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(TextStatusBarSparkMixin)")
        .expect("TextStatusBarSparkMixin probe");
    assert_eq!(
        kind, "table",
        "TextStatusBarSparkMixin = table — TextStatusBar.lua line 282 \
         publishes the optional spark-end-cap mixin attached to the \
         virtual TextStatusBarSparkTemplate Texture at xml line 15"
    );

    for method in SPARK_MIXIN_METHODS {
        let kind: String = env
            .eval(&format!("return type(TextStatusBarSparkMixin['{method}'])"))
            .unwrap_or_else(|err| panic!("TextStatusBarSparkMixin.{method} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "TextStatusBarSparkMixin.{method} must be a function — \
             covers Initialize (captures self.statusBar back-ref), \
             SetVisuals (atlas + xOffset + barHeight + showAtMax), \
             GetIsActive predicate, OnBarValuesUpdated callback, \
             UpdateShown (visibility per current/min/max + showAtMax), \
             UpdateSize (height-ratio scale)"
        );
    }
}
}

prefork_full_ui_case! {
fn text_status_bar_template_materializes_via_create_frame(env: &WowLuaEnv) {

    let probe = "local ok, frame = pcall(function() \
                    return CreateFrame('StatusBar', nil, UIParent, 'TextStatusBar') \
                  end) \
                  return ok and frame ~= nil";

    let result: bool = env.eval(probe).expect("template probe");
    assert!(
        result,
        "TextStatusBar must materialize via CreateFrame as a \
         StatusBar — virtual=true, mixin=TextStatusBarMixin, \
         enableMouseMotion=true. Provides 6 wired scripts: OnLoad → \
         InitializeTextStatusBar, OnEvent → TextStatusBarOnEvent, \
         OnEnter/OnLeave (autoEnableInput=false), OnValueChanged, \
         OnMinMaxChanged"
    );
}
}

prefork_full_ui_case! {
fn spark_template_materializes_via_create_frame(env: &WowLuaEnv) {

    let probe = "local parent = CreateFrame('StatusBar', nil, UIParent, 'TextStatusBar') \
                  local ok, tex = pcall(function() \
                    return parent:CreateTexture(nil, 'OVERLAY', 'TextStatusBarSparkTemplate') \
                  end) \
                  return ok and tex ~= nil";

    let result: bool = env.eval(probe).expect("spark template probe");
    assert!(
        result,
        "TextStatusBarSparkTemplate must materialize via \
         CreateTexture as a Texture — virtual=true, \
         mixin=TextStatusBarSparkMixin, alphaMode=ADD, \
         texelSnappingBias=0.0, snapToPixelGrid=false, \
         anchored RIGHT (positions itself at the bar's fill edge)"
    );
}
}

prefork_full_ui_case! {
fn explicit_load_addon_emits_no_addon_specific_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &text_status_bar_toc())
        .expect("Blizzard_TextStatusBar must load via Rust loader");

    let errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let needles = [
        "TextStatusBar.lua",
        "TextStatusBar.xml",
        "TextStatusBarMixin",
        "TextStatusBarSparkMixin",
        "STATUS_TEXT_DISPLAY_MODE",
        "Blizzard_TextStatusBar",
    ];

    let matched: Vec<&String> = errors
        .iter()
        .filter(|e| needles.iter().any(|n| e.contains(n)))
        .collect();

    assert!(
        matched.is_empty(),
        "Explicit re-load_addon for TextStatusBar must emit zero \
         addon-specific Lua errors. Found {} matching: {:#?}",
        matched.len(),
        matched
    );
}
}

prefork_full_ui_case! {
fn is_addon_loaded_reports_true_after_eager_sweep(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_TextStatusBar')")
        .expect("IsAddOnLoaded probe");
    assert!(
        loaded,
        "After full Game eager sweep, IsAddOnLoaded must report true \
         — TextStatusBar is non-LoD, eagerly pulled"
    );
}
}

#[test]
fn three_mainline_addons_declare_text_status_bar_as_dependency() {
    for declarer in MAINLINE_DEPENDENTS {
        let toc_path = blizzard_ui_dir()
            .join(declarer)
            .join(format!("{}_Mainline.toc", declarer));
        let bare_path = blizzard_ui_dir()
            .join(declarer)
            .join(format!("{}.toc", declarer));

        let toc_path = if toc_path.exists() {
            toc_path
        } else {
            bare_path
        };
        let toc = TocFile::from_file(&toc_path)
            .unwrap_or_else(|err| panic!("{declarer} TOC parses: {err}"));

        assert!(
            toc.dependencies()
                .iter()
                .any(|d| d == "Blizzard_TextStatusBar"),
            "{declarer} must declare Blizzard_TextStatusBar as a hard \
             dependency — its frames consume the TextStatusBar virtual \
             template at template-registration time. Got deps: {:?}",
            toc.dependencies()
        );
    }
}
