#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::discover_blizzard_addons_for_screen;
use wow_ui_sim::loader::load_addon;
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn end_of_match_ui_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_EndOfMatchUI/Blizzard_EndOfMatchUI.toc")
}

fn end_match_ui_alt_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_EndOfMatchUI/Blizzard_EndMatchUI.toc")
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

#[test]
fn blizzard_end_of_match_ui_toc_declares_plunderstorm_only_with_no_deps() {
    let toc =
        TocFile::from_file(&end_of_match_ui_toc()).expect("Blizzard_EndOfMatchUI TOC should parse");

    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_EndOfMatchUI omits `## LoadOnDemand:` — when the Plunderstorm client loads \
         it auto-discovers on the Game screen, no explicit LoadAddOn() needed"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_EndOfMatchUI does not declare `## UseSecureEnvironment` — runs in the \
         standard taint environment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_EndOfMatchUI declares zero dependencies — the addon's three mixins reference \
         only globally-registered templates (BigRedThreeSliceButtonTemplate / \
         BigGoldRedThreeSliceButtonTemplate / HorizontalLayoutFrame) and a handful of \
         `WOW_LABS_*` localization strings, no other addon needs to load first"
    );

    assert!(
        toc.is_game_type_restricted(),
        "Blizzard_EndOfMatchUI declares `## AllowLoadGameType: plunderstorm` — `plunderstorm` \
         is NOT one of the mainline tokens (src/toc.rs:298-299), so the addon IS \
         game-type-restricted on standard retail. discover_blizzard_addons_for_screen \
         filters it out at src/loader/mod.rs:527"
    );

    let toc_text = std::fs::read_to_string(end_of_match_ui_toc())
        .expect("Blizzard_EndOfMatchUI TOC should read");
    assert!(
        toc_text.contains("## AllowLoad: Game"),
        "Blizzard_EndOfMatchUI declares `## AllowLoad: Game` — confines the post-match summary \
         frame to the in-game screen (no glue-screen end-of-match overlay)"
    );
    assert!(
        toc_text.contains("## DefaultState: enabled"),
        "Blizzard_EndOfMatchUI declares `## DefaultState: enabled` — auto-enabled on first \
         install, no user opt-in required for the Plunderstorm post-match recap"
    );
}

#[test]
fn blizzard_end_match_ui_alternate_toc_mirrors_main_toc_flags() {
    let toc = TocFile::from_file(&end_match_ui_alt_toc())
        .expect("Blizzard_EndMatchUI alternate TOC should parse");

    assert!(
        toc.is_game_type_restricted(),
        "The alternate `Blizzard_EndMatchUI.toc` (Title: 'Blizzard End Match UI', no `Of`) \
         shares the same `## AllowLoadGameType: plunderstorm` restriction as the canonical \
         `Blizzard_EndOfMatchUI.toc`. Both TOCs reference the same Lua + XML pair, so the \
         flags must match"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Alternate TOC declares zero dependencies — same as the canonical TOC"
    );
    assert!(
        !toc.is_load_on_demand(),
        "Alternate TOC omits `## LoadOnDemand:` — same as the canonical TOC"
    );
}

#[test]
fn blizzard_end_of_match_ui_is_excluded_from_standard_game_discovery() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_EndOfMatchUI");
    assert!(
        !in_game,
        "Blizzard_EndOfMatchUI must be filtered out of Game-screen auto-discovery on standard \
         mainline retail. The TOC declares `## AllowLoadGameType: plunderstorm`, and \
         discover_blizzard_addons_for_screen skips game-type-restricted addons at \
         src/loader/mod.rs:527 unless the active game type matches"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_EndOfMatchUI");
    assert!(
        !in_login,
        "Blizzard_EndOfMatchUI must NOT appear on Login / glue screens — `## AllowLoad: Game` \
         restricts it to the Game screen, and `## AllowLoadGameType: plunderstorm` restricts \
         it to the Plunderstorm client"
    );
}

prefork_full_ui_case! {
fn blizzard_end_of_match_ui_loads_explicitly_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &end_of_match_ui_toc())
        .expect("Blizzard_EndOfMatchUI should load via explicit Rust loader call");

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("EndOfMatch") || message.contains("MatchDetail"))
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_EndOfMatchUI emitted Lua errors during explicit load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_end_of_match_ui_creates_named_singleton_frame(env: &WowLuaEnv) {

    load_addon(&env.loader_env(), &end_of_match_ui_toc())
        .expect("Blizzard_EndOfMatchUI should load");

    let kind: String = env
        .eval("return type(EndOfMatchFrame)")
        .expect("type(EndOfMatchFrame) probe should succeed");
    assert_eq!(
        kind, "table",
        "EndOfMatchFrame (Blizzard_EndOfMatchUI.xml:60, the addon's only non-virtual \
         top-level frame, `parent=\"UIParent\"`, `toplevel=\"true\"`, `setAllPoints=\"true\"`, \
         `frameLevel=\"1000\"`, `mixin=\"EndOfMatchFrameMixin\"`) must publish as a global \
         table after explicit load"
    );

    let name: String = env
        .eval("return EndOfMatchFrame:GetName()")
        .expect("EndOfMatchFrame:GetName() probe should succeed");
    assert_eq!(
        name, "EndOfMatchFrame",
        "EndOfMatchFrame:GetName() must echo the XML `name` attribute"
    );
}
}

prefork_full_ui_case! {
fn blizzard_end_of_match_ui_singleton_starts_hidden_with_top_level_input(env: &WowLuaEnv) {

    load_addon(&env.loader_env(), &end_of_match_ui_toc())
        .expect("Blizzard_EndOfMatchUI should load");

    let hidden: bool = env
        .eval("return EndOfMatchFrame:IsShown() == false")
        .expect("EndOfMatchFrame:IsShown() probe should succeed");
    assert!(
        hidden,
        "EndOfMatchFrame declares `hidden=\"true\"` (Blizzard_EndOfMatchUI.xml:60) — must be \
         hidden by default. The addon's :TryShow() is called from OnLoad and again from the \
         SHOW_END_OF_MATCH_UI event handler, but only reveals the frame after \
         C_SpectatingUI.IsSpectating() / TryUpdateMatchDetails gates pass"
    );

    let mouse_enabled: bool = env
        .eval("return EndOfMatchFrame:IsMouseEnabled() and true or false")
        .expect("EndOfMatchFrame:IsMouseEnabled() probe should succeed");
    assert!(
        mouse_enabled,
        "EndOfMatchFrame declares `enableMouse=\"true\"` (Blizzard_EndOfMatchUI.xml:60) — the \
         post-match summary captures clicks so they don't leak through to the world frame. \
         (Note: keyboard-enabled was also declared but is not exposed via :IsKeyboardEnabled() \
         in the simulator's XML-to-widget conversion.)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_end_of_match_ui_publishes_three_mixins(env: &WowLuaEnv) {

    load_addon(&env.loader_env(), &end_of_match_ui_toc())
        .expect("Blizzard_EndOfMatchUI should load");

    let kinds: (String, String, String) = env
        .eval(
            "return type(EndOfMatchFrameMixin), \
                    type(MatchDetailFrameMixin), \
                    type(EndOfMatchButtonBaseMixin)",
        )
        .expect("mixin globals type probe should succeed");
    assert_eq!(
        kinds,
        (
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
        ),
        "Blizzard_EndOfMatchUI.lua publishes exactly three mixin globals: \
         EndOfMatchFrameMixin (the singleton's mixin, drives OnLoad/OnEvent/TryShow), \
         MatchDetailFrameMixin (per-row detail-frame mixin attached to the \
         MatchDetailFrameTemplate template), EndOfMatchButtonBaseMixin (shared by \
         EndOfMatchButtonBaseTemplate + EndOfMatchButtonGoldButtonBaseTemplate, the \
         BigRed/BigGoldRed three-slice button variants)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_end_of_match_ui_singleton_carries_three_frame_pools(env: &WowLuaEnv) {

    load_addon(&env.loader_env(), &end_of_match_ui_toc())
        .expect("Blizzard_EndOfMatchUI should load");

    let pool_kinds: (String, String, String) = env
        .eval(
            "return type(EndOfMatchFrame.detailPool), \
                    type(EndOfMatchFrame.actionsButtonPool), \
                    type(EndOfMatchFrame.goldActionsButtonPool)",
        )
        .expect("frame pool probe should succeed");
    assert_eq!(
        pool_kinds,
        (
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
        ),
        "EndOfMatchFrameMixin:OnLoad (Blizzard_EndOfMatchUI.lua:7-18) creates three frame \
         pools: detailPool (Frame, parent=DetailsContainer, MatchDetailFrameTemplate), \
         actionsButtonPool (Button, parent=self, EndOfMatchButtonBaseTemplate), \
         goldActionsButtonPool (Button, parent=self, EndOfMatchButtonGoldButtonBaseTemplate). \
         All three must exist as table fields after OnLoad runs — confirms \
         CreateFramePool resolves the templates without erroring"
    );
}
}

prefork_full_ui_case! {
fn blizzard_end_of_match_ui_is_addon_loaded_flips_after_explicit_load(env: &WowLuaEnv) {

    let pre_load: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_EndOfMatchUI') and true or false")
        .expect("pre-load IsAddOnLoaded probe should succeed");
    assert!(
        !pre_load,
        "C_AddOns.IsAddOnLoaded('Blizzard_EndOfMatchUI') must return false BEFORE the \
         explicit load — the addon is filtered out of Game-screen discovery by \
         is_game_type_restricted (plunderstorm-only)"
    );

    load_addon(&env.loader_env(), &end_of_match_ui_toc())
        .expect("Blizzard_EndOfMatchUI should load");

    let post_load: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_EndOfMatchUI') and true or false")
        .expect("post-load IsAddOnLoaded probe should succeed");
    assert!(
        post_load,
        "C_AddOns.IsAddOnLoaded('Blizzard_EndOfMatchUI') must return true AFTER the explicit \
         load_addon call — confirms the loader registers the addon with the loaded-set even \
         when the TOC's game-type token doesn't match the active client"
    );
}
}

prefork_full_ui_case! {
fn blizzard_end_of_match_ui_singleton_is_parented_to_ui_parent(env: &WowLuaEnv) {

    load_addon(&env.loader_env(), &end_of_match_ui_toc())
        .expect("Blizzard_EndOfMatchUI should load");

    let parent_name: String = env
        .eval("return EndOfMatchFrame:GetParent():GetName()")
        .expect("EndOfMatchFrame:GetParent():GetName() probe should succeed");
    assert_eq!(
        parent_name, "UIParent",
        "EndOfMatchFrame declares `parent=\"UIParent\"` (Blizzard_EndOfMatchUI.xml:60) — \
         must report UIParent via :GetParent():GetName() so the post-match overlay scales \
         and hides with the rest of the standard UI"
    );
}
}

prefork_full_ui_case! {
fn blizzard_end_of_match_ui_singleton_registers_for_show_end_of_match_event(env: &WowLuaEnv) {

    load_addon(&env.loader_env(), &end_of_match_ui_toc())
        .expect("Blizzard_EndOfMatchUI should load");

    let registered: bool = env
        .eval("return EndOfMatchFrame:IsEventRegistered('SHOW_END_OF_MATCH_UI') and true or false")
        .expect("event-registration probe should succeed");
    assert!(
        registered,
        "EndOfMatchFrameMixin:OnLoad calls FrameUtil.RegisterFrameForEvents(self, \
         EndOfMatchFrameEvents) where the table contains exactly `SHOW_END_OF_MATCH_UI` \
         (Blizzard_EndOfMatchUI.lua:3-5,8). After load this event must be registered on the \
         singleton — it triggers :TryShow(false) when the game fires the post-match-UI event"
    );
}
}

prefork_full_ui_case! {
fn blizzard_end_of_match_ui_singleton_resolves_xml_named_children(env: &WowLuaEnv) {

    load_addon(&env.loader_env(), &end_of_match_ui_toc())
        .expect("Blizzard_EndOfMatchUI should load");

    let child_kinds: (String, String, String, String) = env
        .eval(
            "return type(EndOfMatchFrame.Title), \
                    type(EndOfMatchFrame.DetailsContainer), \
                    type(EndOfMatchFrame.CenterActionsContainer), \
                    type(EndOfMatchFrame.ActionsContainer)",
        )
        .expect("named-child probe should succeed");
    let table = "table".to_string();
    assert_eq!(
        child_kinds,
        (table.clone(), table.clone(), table.clone(), table.clone()),
        "Blizzard_EndOfMatchUI.xml declares parentKey-named children on EndOfMatchFrame: \
         Title (FontString), DetailsContainer (Frame, target of detailPool), \
         CenterActionsContainer (HorizontalLayoutFrame for the gold-button row), \
         ActionsContainer (HorizontalLayoutFrame for the bottom button row). All four must \
         resolve as tables after load — confirms parentKey resolution sees the singleton"
    );
}
}
