#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::discover_blizzard_addons_for_screen;
use wow_ui_sim::loader::load_addon;
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
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
fn blizzard_command_line_util_appears_in_both_screens_discovery() {
    let ui = blizzard_ui_dir();

    let game_addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CommandLineUtil");
    assert!(
        in_game,
        "Blizzard_CommandLineUtil (`## AllowLoad: both`, `## DefaultState: enabled`, non-LOD) \
         should appear in Game-screen discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CommandLineUtil");
    assert!(
        in_login,
        "Blizzard_CommandLineUtil (`## AllowLoad: both`) should also appear in Login-screen \
         discovery"
    );
}

prefork_full_ui_case! {
fn blizzard_command_line_util_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("Blizzard_CommandLineUtil"))
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_CommandLineUtil emitted Lua errors during load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_command_line_util_namespace_is_defined(env: &WowLuaEnv) {

    let helpers_present: bool = env
        .eval(
            "return type(CommandLineUtil) == 'table' \
                and type(CommandLineUtil.ScoreStrings) == 'function' \
                and type(CommandLineUtil.BinaryInsert) == 'function'",
        )
        .expect("CommandLineUtil query should succeed");
    assert!(
        helpers_present,
        "CommandLineUtil and its two helpers (ScoreStrings/BinaryInsert) should be defined \
         after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_command_line_util_score_strings_prefers_substring_match(env: &WowLuaEnv) {

    let exact_match_is_best: bool = env
        .eval(
            "local exact = CommandLineUtil.ScoreStrings('reload', 'reload'); \
             local prefix_match = CommandLineUtil.ScoreStrings('reload', 'reloadui'); \
             local far = CommandLineUtil.ScoreStrings('xyz', 'qqq'); \
             return exact < prefix_match and prefix_match < far and far == 100",
        )
        .expect("ScoreStrings query should succeed");
    assert!(
        exact_match_is_best,
        "ScoreStrings should rank exact match (lowest score) better than prefix match, and \
         return the sentinel 100 when neither substring nor edit distance can find a match \
         (edit distance == math.max(#a, #b) when no character overlaps)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_command_line_util_binary_insert_keeps_table_sorted_by_score(env: &WowLuaEnv) {

    let stays_sorted: bool = env
        .eval(
            "local t = {}; \
             CommandLineUtil.BinaryInsert(t, {score = 5, name = 'b'}); \
             CommandLineUtil.BinaryInsert(t, {score = 1, name = 'a'}); \
             CommandLineUtil.BinaryInsert(t, {score = 9, name = 'd'}); \
             CommandLineUtil.BinaryInsert(t, {score = 7, name = 'c'}); \
             return #t == 4 \
                and t[1].score == 1 and t[1].name == 'a' \
                and t[2].score == 5 and t[2].name == 'b' \
                and t[3].score == 7 and t[3].name == 'c' \
                and t[4].score == 9 and t[4].name == 'd'",
        )
        .expect("BinaryInsert query should succeed");
    assert!(
        stays_sorted,
        "BinaryInsert should preserve ascending-score order across successive inserts \
         (4 inserts produce a sorted-by-score table of 4 entries)"
    );
}
}
