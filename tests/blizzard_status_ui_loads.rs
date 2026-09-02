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

fn status_ui_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_StatusUI")
}

fn status_ui_toc() -> PathBuf {
    status_ui_dir().join("Blizzard_StatusUI.toc")
}

const STATUS_UI_MIXIN_METHODS: &[&str] = &["OnLoad", "OnShow", "OnHide"];

const GLUE_SCREENS: &[ScreenKind] = &[
    ScreenKind::Login,
    ScreenKind::CharacterSelect,
    ScreenKind::CharacterCreate,
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
    let resolved = find_toc_file(&status_ui_dir()).expect("StatusUI TOC resolves");
    assert_eq!(resolved, status_ui_toc(), "Bare TOC — no flavor suffixes");
}

#[test]
fn toc_is_load_on_demand_with_no_dependencies() {
    let toc = TocFile::from_file(&status_ui_toc()).expect("TOC parses");

    assert!(
        toc.is_load_on_demand(),
        "`## LoadOnDemand: 1` — StatusUI is the GM/Survey notification frame, \
         only loaded when GMChatUI or WowSurveyUI explicitly demands it"
    );
    assert!(
        toc.dependencies().is_empty(),
        "No `## Dependencies:` — StatusUI is leaf-level. Got: {:?}",
        toc.dependencies()
    );
    assert!(toc.optional_deps().is_empty());
    assert!(toc.saved_variables().is_empty());
}

#[test]
fn toc_has_no_allow_load_or_game_type_restriction() {
    let toc = TocFile::from_file(&status_ui_toc()).expect("TOC parses");

    assert!(
        !toc.is_game_type_restricted(),
        "No `## AllowLoadGameType` — StatusUI loads on every flavor that \
         needs the GM chat surface (mainline + classic-style products)"
    );
    assert!(!toc.is_secure_env());
    assert!(!toc.is_load_first());
    assert!(toc.default_enabled());
}

#[test]
fn allow_load_absent_restricts_to_game_screen_only() {
    let toc = TocFile::from_file(&status_ui_toc()).expect("TOC parses");

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "`## AllowLoad` absent → None branch in toc.rs:305-313 defaults to \
         Game-only — GM chat alerts only appear in-world"
    );
    for screen in GLUE_SCREENS {
        assert!(
            !toc.allows_screen(*screen),
            "Glue screen {screen:?} must be excluded — no AllowLoad means \
             Game-only by default"
        );
    }
}

#[test]
fn toc_raw_bytes_pin_three_metadata_directives() {
    let raw = std::fs::read_to_string(status_ui_toc()).expect("TOC reads utf-8");

    let expected_directives = [
        "## Title: Blizzard_StatusUI",
        "## LoadOnDemand: 1",
        "Blizzard_StatusUI.lua",
        "Blizzard_StatusUI.xml",
    ];

    for directive in expected_directives {
        assert!(
            raw.contains(directive),
            "Raw TOC must pin `{directive}` — minimal 4-line TOC: title + \
             LoadOnDemand flag + 2 body files"
        );
    }

    assert!(!raw.contains("## Dependencies"));
    assert!(!raw.contains("## OptionalDeps"));
    assert!(!raw.contains("## AllowLoad"));
    assert!(!raw.contains("## SavedVariables"));
    assert!(!raw.contains("## UseSecureEnvironment"));
    assert!(!raw.contains("## LoadFirst"));
}

#[test]
fn body_orders_lua_before_xml() {
    let toc = TocFile::from_file(&status_ui_toc()).expect("TOC parses");

    let body: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    assert_eq!(
        body.len(),
        2,
        "Two body entries — Lua publishes StatusUIMixin then XML registers \
         StatusUIFrame template that consumes it. Got: {body:?}"
    );
    assert_eq!(body[0], "Blizzard_StatusUI.lua");
    assert_eq!(
        body[1], "Blizzard_StatusUI.xml",
        "Lua must precede XML — `mixin=\"StatusUIMixin\"` on the virtual \
         template resolves at template-registration time"
    );
}

#[test]
fn absent_from_game_eager_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let found = addons.iter().any(|(name, _)| name == "Blizzard_StatusUI");
    assert!(
        !found,
        "`## LoadOnDemand: 1` must exclude StatusUI from Game eager sweep \
         — its 2 dependents (Blizzard_GMChatUI, Blizzard_WowSurveyUI) are \
         themselves LoD, so pull_required_lod_addons does NOT promote it"
    );
}

#[test]
fn absent_from_every_glue_screen_discovery() {
    for screen in GLUE_SCREENS {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), *screen);
        let found = addons.iter().any(|(name, _)| name == "Blizzard_StatusUI");
        assert!(
            !found,
            "StatusUI must not appear on glue screen {screen:?} — \
             AllowLoad absent + LoadOnDemand both exclude it"
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

    load_addon(&env.loader_env(), &status_ui_toc())
        .expect("Blizzard_StatusUI must load via Rust loader");

    let errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let needles = [
        "Blizzard_StatusUI.lua",
        "Blizzard_StatusUI.xml",
        "StatusUIMixin",
        "StatusUIFrame",
        "Blizzard_StatusUI",
    ];

    let matched: Vec<&String> = errors
        .iter()
        .filter(|e| needles.iter().any(|n| e.contains(n)))
        .collect();

    assert!(
        matched.is_empty(),
        "Explicit load_addon for StatusUI must emit zero addon-specific \
         Lua errors. Found {} matching: {:#?}",
        matched.len(),
        matched
    );
}
}

prefork_full_ui_case! {
fn is_addon_loaded_reports_true_after_explicit_load(env: &WowLuaEnv) {

    let before: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_StatusUI')")
        .expect("IsAddOnLoaded probe");
    assert!(
        !before,
        "Before explicit load, IsAddOnLoaded must report false — \
         eager Game sweep excludes LoadOnDemand addons"
    );

    load_addon(&env.loader_env(), &status_ui_toc())
        .expect("Blizzard_StatusUI must load via Rust loader");

    let after: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_StatusUI')")
        .expect("IsAddOnLoaded probe");
    assert!(
        after,
        "After explicit load_addon, IsAddOnLoaded must report true"
    );
}
}

prefork_full_ui_case! {
fn status_ui_mixin_publishes_with_three_lifecycle_methods(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &status_ui_toc())
        .expect("Blizzard_StatusUI must load via Rust loader");

    let kind: String = env
        .eval("return type(StatusUIMixin)")
        .expect("StatusUIMixin probe");
    assert_eq!(
        kind, "table",
        "StatusUIMixin = table — Blizzard_StatusUI.lua line 1 publishes \
         the mixin consumed by the virtual StatusUIFrame template"
    );

    for method in STATUS_UI_MIXIN_METHODS {
        let kind: String = env
            .eval(&format!("return type(StatusUIMixin['{method}'])"))
            .unwrap_or_else(|err| panic!("StatusUIMixin.{method} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "StatusUIMixin.{method} = function — OnLoad applies nine-slice \
             gmglow corners + ADD blendMode + sizes from text + tooltip bg \
             color, OnShow/OnHide both call UIParent_UpdateTopFramePositions"
        );
    }
}
}

prefork_full_ui_case! {
fn status_ui_frame_template_materializes_via_create_frame(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &status_ui_toc())
        .expect("Blizzard_StatusUI must load via Rust loader");

    let probe = "local ok, frame = pcall(function() \
                    return CreateFrame('Button', nil, UIParent, 'StatusUIFrame') \
                  end) \
                  return ok and frame ~= nil";

    let result: bool = env.eval(probe).expect("template probe");
    assert!(
        result,
        "StatusUIFrame must materialize via CreateFrame as a Button — \
         virtual=true, toplevel=true, parent=UIParent, mixin=StatusUIMixin, \
         hidden=true, frameStrata=FULLSCREEN, enableMouse=true. Provides \
         TitleText/SubtitleText FontStrings + Icon Texture + NineSlice + \
         Pulse animation child frames"
    );
}
}

#[test]
fn declared_dependents_pin_to_load_on_demand_addons() {
    let gm_chat_toc = blizzard_ui_dir().join("Blizzard_GMChatUI/Blizzard_GMChatUI.toc");
    let survey_toc = blizzard_ui_dir().join("Blizzard_WowSurveyUI/Blizzard_WowSurveyUI.toc");

    let gm_chat = TocFile::from_file(&gm_chat_toc).expect("GMChatUI TOC parses");
    let survey = TocFile::from_file(&survey_toc).expect("WowSurveyUI TOC parses");

    assert!(
        gm_chat
            .dependencies()
            .contains(&"Blizzard_StatusUI".to_string()),
        "Blizzard_GMChatUI must declare StatusUI as a dependency"
    );
    assert!(
        survey
            .dependencies()
            .contains(&"Blizzard_StatusUI".to_string()),
        "Blizzard_WowSurveyUI must declare StatusUI as a dependency"
    );

    assert!(
        gm_chat.is_load_on_demand(),
        "Blizzard_GMChatUI is LoadOnDemand — keeps StatusUI off the \
         eager Game sweep (pull_required_lod_addons only promotes LoD \
         addons declared by NON-LoD addons)"
    );
    assert!(
        survey.is_load_on_demand(),
        "Blizzard_WowSurveyUI is LoadOnDemand — same exclusion rule"
    );
}
