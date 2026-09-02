#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn frame_stack_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_FrameStack/Blizzard_FrameStack.toc")
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
fn blizzard_frame_stack_toc_is_eager_minimal_two_line_header() {
    let toc = TocFile::from_file(&frame_stack_toc()).expect("Blizzard_FrameStack TOC should parse");

    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_FrameStack has no `## LoadOnDemand` line — the addon must auto-load \
         at startup so its TOGGLE_FRAMESTACK event listener is wired up before the user \
         hits the framestack hotkey. The actual heavy debug-tools machinery is \
         lazy-loaded via C_AddOns.LoadAddOn(\"Blizzard_DebugTools\") only when the event \
         fires"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_FrameStack does not declare `## UseSecureEnvironment` — debug \
         tooling runs in the standard taint environment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_FrameStack declares no `## Dependencies` line — the only globals \
         referenced at load time are CreateFrame and the event-listener wiring; \
         FrameStackTooltip_Toggle and Blizzard_DebugTools are referenced lazily inside \
         the OnEvent handler which only fires when the user toggles framestack"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_FrameStack declares no `## AllowLoadGameType:` line, so \
         `is_game_type_restricted()` returns false"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_FrameStack declares no `## SavedVariables` — the framestack overlay \
         is purely transient debug state"
    );

    let toc_text =
        std::fs::read_to_string(frame_stack_toc()).expect("Blizzard_FrameStack TOC should read");
    assert!(
        toc_text.contains("## AllowLoad: Both"),
        "Blizzard_FrameStack declares `## AllowLoad: Both` — the framestack debug \
         overlay is useful on glue screens too (login/character-select frame inspection)"
    );
    assert!(
        !toc_text.contains("## LoadFirst"),
        "Blizzard_FrameStack does NOT declare `## LoadFirst:` — the loader runs it in \
         the standard tier; nothing else depends on it being early"
    );
}

#[test]
fn blizzard_frame_stack_allows_all_screens_including_glue() {
    let toc = TocFile::from_file(&frame_stack_toc()).expect("Blizzard_FrameStack TOC should parse");

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "`## AllowLoad: Both` must allow the Game screen (src/toc.rs:307)"
    );
    assert!(
        toc.allows_screen(ScreenKind::Login),
        "`## AllowLoad: Both` must allow the Login screen"
    );
    assert!(
        toc.allows_screen(ScreenKind::CharacterSelect),
        "`## AllowLoad: Both` must allow CharacterSelect"
    );
}

#[test]
fn blizzard_frame_stack_auto_loads_on_game_and_login_screens() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_FrameStack");
    assert!(
        in_game,
        "Blizzard_FrameStack has no `## LoadOnDemand` line and `## AllowLoad: Both`, \
         so it MUST appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_FrameStack");
    assert!(
        in_login,
        "`## AllowLoad: Both` plus no LoadOnDemand means Blizzard_FrameStack MUST \
         appear in Login-screen auto-discovery as well"
    );
}

prefork_full_ui_case! {
fn blizzard_frame_stack_loads_via_full_game_ui_without_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("FrameStack") || message.contains("Blizzard_FrameStack"))
        .cloned()
        .collect();

    assert!(
        load_errors.is_empty(),
        "Blizzard_FrameStack emitted Lua errors during the full Game-screen load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_stack_is_addon_loaded_returns_true_after_full_game_ui_load(env: &WowLuaEnv) {

    let post_load: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_FrameStack') and true or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        post_load,
        "After full Game-screen load, IsAddOnLoaded('Blizzard_FrameStack') must return \
         true — auto-discovery picks up the addon (no LoadOnDemand) and \
         `mark_addon_loaded` registers it"
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_stack_publishes_zero_globals_keeps_loader_local(env: &WowLuaEnv) {

    let zero_globals: (bool, bool) = env
        .eval(
            "return _G.frameStackLoader == nil, \
                    _G.FrameStackLoader == nil",
        )
        .expect("FrameStack global leak probe should succeed");
    assert_eq!(
        zero_globals,
        (true, true),
        "FrameStack.lua wraps the loader in a `do ... end` block (lua:1) so \
         `frameStackLoader` is a block-scoped local — neither the lowercased local \
         nor a TitleCase capture should leak into _G. The addon's only side effect is \
         registering the TOGGLE_FRAMESTACK event listener; it intentionally publishes \
         no globals to keep the surface minimal"
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_stack_does_not_eager_load_blizzard_debug_tools(env: &WowLuaEnv) {

    let debug_tools_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_DebugTools') and true or false")
        .expect("Blizzard_DebugTools loaded probe should succeed");
    assert!(
        !debug_tools_loaded,
        "Blizzard_DebugTools is referenced by FrameStack.lua:7 inside the \
         TOGGLE_FRAMESTACK OnEvent handler via `C_AddOns.LoadAddOn(...)` — it is a \
         lazy load that fires only when the user toggles framestack. Just loading \
         Blizzard_FrameStack at startup must NOT cascade-load Blizzard_DebugTools, \
         otherwise the lazy-load contract would be broken and the heavy debug-tools \
         machinery would always be resident even when unused"
    );
}
}

prefork_full_ui_case! {
fn blizzard_frame_stack_loads_after_full_game_ui_emits_no_pending_lua_errors(env: &WowLuaEnv) {

    let total_errors: usize = env.state().borrow().lua_errors.len();
    let frame_stack_errors: usize = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("FrameStack"))
        .count();

    assert_eq!(
        frame_stack_errors, 0,
        "Blizzard_FrameStack must contribute zero entries to the Lua error log even \
         when other addons in the load order produce errors. Total error count is {} \
         but framestack-mentioning entries should be 0",
        total_errors
    );
}
}
