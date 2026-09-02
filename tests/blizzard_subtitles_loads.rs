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

fn subtitles_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_Subtitles")
}

fn subtitles_toc() -> PathBuf {
    subtitles_dir().join("Blizzard_Subtitles.toc")
}

const ALL_FOUR_SCREENS: &[ScreenKind] = &[
    ScreenKind::Game,
    ScreenKind::Login,
    ScreenKind::CharacterSelect,
    ScreenKind::CharacterCreate,
];

const GLUE_SCREENS: &[ScreenKind] = &[
    ScreenKind::Login,
    ScreenKind::CharacterSelect,
    ScreenKind::CharacterCreate,
];

const SUBTITLES_FRAME_MIXIN_METHODS: &[&str] = &[
    "OnLoad",
    "OnEvent",
    "OnMovieCinematicPlay",
    "OnMovieCinematicStop",
    "AddSubtitle",
    "HideSubtitles",
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
    let resolved = find_toc_file(&subtitles_dir()).expect("Subtitles TOC resolves");
    assert_eq!(
        resolved,
        subtitles_toc(),
        "Bare TOC — no flavor suffix; movie-subtitles addon ships universally"
    );
}

#[test]
fn toc_is_eager_with_no_dependencies() {
    let toc = TocFile::from_file(&subtitles_toc()).expect("TOC parses");

    assert!(
        !toc.is_load_on_demand(),
        "No `## LoadOnDemand` — Subtitles must be loaded before any \
         cinematic plays so OnLoad can register SHOW_SUBTITLE / \
         HIDE_SUBTITLE events at startup"
    );
    assert!(
        toc.dependencies().is_empty(),
        "No `## Dependencies` — leaf addon. Got: {:?}",
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
    let toc = TocFile::from_file(&subtitles_toc()).expect("TOC parses");

    for screen in ALL_FOUR_SCREENS {
        assert!(
            toc.allows_screen(*screen),
            "`## AllowLoad: Both` (case-insensitive at toc.rs:307) must \
             permit screen {screen:?} — cinematics play on glue screens \
             too (intro movie, char-create reveal) so subtitles must \
             render there"
        );
    }
}

#[test]
fn toc_raw_bytes_pin_three_metadata_directives() {
    let raw = std::fs::read_to_string(subtitles_toc()).expect("TOC reads utf-8");

    let expected_directives = [
        "## Title: Blizzard_Subtitles",
        "## Version: 1.0",
        "## AllowLoad: Both",
        "Blizzard_Subtitles.lua",
        "Blizzard_Subtitles.xml",
    ];

    for directive in expected_directives {
        assert!(
            raw.contains(directive),
            "Raw TOC must pin `{directive}` — minimal 5-line TOC: title \
             + version + AllowLoad: Both + 2 body files (lua then xml)"
        );
    }

    assert!(!raw.contains("## Dependencies"));
    assert!(!raw.contains("## RequiredDep"));
    assert!(!raw.contains("## OptionalDep"));
    assert!(!raw.contains("## AllowLoadGameType"));
    assert!(!raw.contains("## SavedVariables"));
    assert!(!raw.contains("## UseSecureEnvironment"));
    assert!(!raw.contains("## LoadFirst"));
    assert!(!raw.contains("## LoadOnDemand"));
}

#[test]
fn body_orders_lua_before_xml() {
    let toc = TocFile::from_file(&subtitles_toc()).expect("TOC parses");

    let body: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    assert_eq!(
        body.len(),
        2,
        "Two body entries — Lua publishes SubtitlesFrameMixin then XML \
         applies it via mixin=SubtitlesFrameMixin on the named frame. \
         Got: {body:?}"
    );
    assert_eq!(body[0], "Blizzard_Subtitles.lua");
    assert_eq!(
        body[1], "Blizzard_Subtitles.xml",
        "Lua must precede XML — mixin attribute resolves at frame \
         materialization time which happens during XML parsing"
    );
}

#[test]
fn present_in_every_screen_eager_discovery() {
    for screen in ALL_FOUR_SCREENS {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), *screen);
        let found = addons.iter().any(|(name, _)| name == "Blizzard_Subtitles");
        assert!(
            found,
            "Blizzard_Subtitles must appear in eager discovery on \
             {screen:?} — AllowLoad: Both + no LoadOnDemand means it \
             loads on every screen at startup"
        );
    }
}

prefork_full_ui_case! {
fn subtitles_frame_mixin_publishes_with_six_methods(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(SubtitlesFrameMixin)")
        .expect("SubtitlesFrameMixin probe");
    assert_eq!(
        kind, "table",
        "SubtitlesFrameMixin = table — Blizzard_Subtitles.lua line 6 \
         publishes the mixin consumed by the named SubtitlesFrame"
    );

    for method in SUBTITLES_FRAME_MIXIN_METHODS {
        let kind: String = env
            .eval(&format!("return type(SubtitlesFrameMixin['{method}'])"))
            .unwrap_or_else(|err| panic!("SubtitlesFrameMixin.{method} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "SubtitlesFrameMixin.{method} must be a function — OnLoad \
             registers SHOW_SUBTITLE/HIDE_SUBTITLE + Subtitles.* \
             EventRegistry callbacks; OnEvent dispatches between \
             AddSubtitle and HideSubtitles; OnMovieCinematicPlay/Stop \
             toggle visibility around movies; AddSubtitle scrolls the \
             FontString array; HideSubtitles clears all entries"
        );
    }
}
}

prefork_full_ui_case! {
fn subtitles_frame_materializes_via_eager_load(env: &WowLuaEnv) {

    let probe = "return SubtitlesFrame ~= nil";
    let present: bool = env.eval(probe).expect("SubtitlesFrame probe");
    assert!(
        present,
        "SubtitlesFrame must materialize at eager load — XML declares \
         a non-virtual named Frame (frameLevel=100, clampedToScreen, \
         enableKeyboard/Mouse, hidden, mixin=SubtitlesFrameMixin) \
         anchored full-width at the bottom of the screen"
    );

    let hidden: bool = env
        .eval("return SubtitlesFrame:IsShown() == false")
        .expect("hidden probe");
    assert!(
        hidden,
        "SubtitlesFrame must start hidden — `hidden=true` in XML; only \
         shown on cinematic-play via OnMovieCinematicPlay"
    );
}
}

prefork_full_ui_case! {
fn subtitles_frame_exposes_subtitles_array_and_background(env: &WowLuaEnv) {

    let array_size: i64 = env
        .eval("return #SubtitlesFrame.Subtitles")
        .expect("Subtitles array probe");
    assert!(
        array_size >= 1,
        "SubtitlesFrame.Subtitles must hold at least 1 FontString — \
         XML declares Subtitle1 with parentArray=\"Subtitles\" so the \
         array exposes the single declared entry; AddSubtitle's scroll \
         loop and HideSubtitles' clear loop iterate `#self.Subtitles` \
         and remain correct regardless of array size. Got: {array_size}"
    );

    let first_is_subtitle1: bool = env
        .eval("return SubtitlesFrame.Subtitles[1] == SubtitlesFrame.Subtitle1")
        .expect("Subtitles[1] identity probe");
    assert!(
        first_is_subtitle1,
        "Subtitles[1] must be the same FontString as the .Subtitle1 \
         parentKey — both refer to the lone declared <FontString \
         parentKey=\"Subtitle1\" parentArray=\"Subtitles\"/> entry"
    );

    let subtitle1_kind: String = env
        .eval("return type(SubtitlesFrame.Subtitle1)")
        .expect("Subtitle1 probe");
    assert_eq!(
        subtitle1_kind, "table",
        "SubtitlesFrame.Subtitle1 must be the FontString — also \
         reachable as SubtitlesFrame.Subtitles[1] via parentArray"
    );

    let background_kind: String = env
        .eval("return type(SubtitlesFrame.SubtitleBackground)")
        .expect("SubtitleBackground probe");
    assert_eq!(
        background_kind, "table",
        "SubtitlesFrame.SubtitleBackground must be the ARTWORK Texture \
         used as the dark/light backdrop behind movie subtitles when \
         the movieSubtitleBackground CVar > 1 (NONE)"
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

    load_addon(&env.loader_env(), &subtitles_toc())
        .expect("Blizzard_Subtitles must load via Rust loader");

    let errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let needles = [
        "Blizzard_Subtitles.lua",
        "Blizzard_Subtitles.xml",
        "SubtitlesFrameMixin",
        "SubtitlesFrame",
        "Blizzard_Subtitles",
    ];

    let matched: Vec<&String> = errors
        .iter()
        .filter(|e| needles.iter().any(|n| e.contains(n)))
        .collect();

    assert!(
        matched.is_empty(),
        "Explicit re-load_addon for Subtitles must emit zero \
         addon-specific Lua errors. Found {} matching: {:#?}",
        matched.len(),
        matched
    );
}
}

prefork_full_ui_case! {
fn is_addon_loaded_reports_true_after_eager_sweep(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_Subtitles')")
        .expect("IsAddOnLoaded probe");
    assert!(
        loaded,
        "After full Game eager sweep, IsAddOnLoaded must report true \
         — Subtitles is non-LoD so it's pulled in by the eager pass"
    );
}
}

#[test]
fn glue_screen_eager_discovery_includes_subtitles() {
    for screen in GLUE_SCREENS {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), *screen);
        let found = addons.iter().any(|(name, _)| name == "Blizzard_Subtitles");
        assert!(
            found,
            "Glue screen {screen:?} must include Subtitles in eager \
             discovery — AllowLoad: Both surfaces it on all 4 screens, \
             enabling subtitle rendering for the intro movie + \
             char-create reveal cinematics that play during the glue \
             flow"
        );
    }
}
