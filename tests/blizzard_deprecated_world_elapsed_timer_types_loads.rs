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

fn deprecated_world_elapsed_timer_types_toc() -> PathBuf {
    blizzard_ui_dir().join(
        "Blizzard_DeprecatedWorldElapsedTimerTypes/Blizzard_DeprecatedWorldElapsedTimerTypes.toc",
    )
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
fn blizzard_deprecated_world_elapsed_timer_types_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_world_elapsed_timer_types_toc())
        .expect("Blizzard_DeprecatedWorldElapsedTimerTypes TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedWorldElapsedTimerTypes declares `## LoadOnDemand: 0` — the three \
         LE_WORLD_ELAPSED_TIMER_TYPE_* globals must install before any timer-frame addon Lua \
         executes that reads them"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedWorldElapsedTimerTypes does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedWorldElapsedTimerTypes declares NO dependencies — the three aliases \
         forward to Enum.WorldElapsedTimerTypes (registered at \
         src/lua_api/globals/enum_data/widget.rs:60-63), bootstrapped at env init"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_DeprecatedWorldElapsedTimerTypes declares no `## AllowLoadGameType:` filter — \
         loads on every game type without restriction"
    );

    let toc_text = std::fs::read_to_string(deprecated_world_elapsed_timer_types_toc())
        .expect("Blizzard_DeprecatedWorldElapsedTimerTypes TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedWorldElapsedTimerTypes omits `## AllowLoad:` — defaults to \
         Game-screen-only (src/toc.rs:311), matching the in-game-only timer-frame surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedWorldElapsedTimerTypes omits `## AllowLoadGameType:` — three-line \
         alias shim is universal across game types"
    );
}

#[test]
fn blizzard_deprecated_world_elapsed_timer_types_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedWorldElapsedTimerTypes");
    assert!(
        in_game,
        "Blizzard_DeprecatedWorldElapsedTimerTypes (no AllowLoad flag, defaults to Game-only) \
         should appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedWorldElapsedTimerTypes");
    assert!(
        !in_login,
        "Blizzard_DeprecatedWorldElapsedTimerTypes should NOT appear on the Login / glue \
         screens — challenge-mode / proving-ground timers are an in-game concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_world_elapsed_timer_types_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedWorldElapsedTimerTypes")
                || message.contains("Deprecated_WorldElapsedTimerTypes")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedWorldElapsedTimerTypes emitted Lua errors during Game-screen load:\n  \
         {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_world_elapsed_timer_types_installs_le_globals_as_numbers(env: &WowLuaEnv) {

    let kinds: (String, String, String) = env
        .eval(
            "return type(LE_WORLD_ELAPSED_TIMER_TYPE_NONE), \
                    type(LE_WORLD_ELAPSED_TIMER_TYPE_CHALLENGE_MODE), \
                    type(LE_WORLD_ELAPSED_TIMER_TYPE_PROVING_GROUND)",
        )
        .expect("type(LE_WORLD_ELAPSED_TIMER_TYPE_*) query should succeed");
    assert_eq!(
        kinds,
        (
            "number".to_string(),
            "number".to_string(),
            "number".to_string()
        ),
        "Deprecated_WorldElapsedTimerTypes.lua lines 9-11 assign \
         LE_WORLD_ELAPSED_TIMER_TYPE_{{NONE,CHALLENGE_MODE,PROVING_GROUND}} = \
         Enum.WorldElapsedTimerTypes.{{None,ChallengeMode,ProvingGround}} — the underlying \
         enum (src/lua_api/globals/enum_data/widget.rs:60-63) registers these as integer \
         values, so all three globals must be of type `number`"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_world_elapsed_timer_types_le_constants_match_enum_values(env: &WowLuaEnv) {

    let values: (i64, i64, i64) = env
        .eval(
            "return LE_WORLD_ELAPSED_TIMER_TYPE_NONE, \
                    LE_WORLD_ELAPSED_TIMER_TYPE_CHALLENGE_MODE, \
                    LE_WORLD_ELAPSED_TIMER_TYPE_PROVING_GROUND",
        )
        .expect("LE_WORLD_ELAPSED_TIMER_TYPE_* values query should succeed");
    assert_eq!(
        values,
        (0, 1, 2),
        "The three deprecated LE_* constants must equal Enum.WorldElapsedTimerTypes.None=0, \
         ChallengeMode=1, ProvingGround=2 (src/lua_api/globals/enum_data/widget.rs:62). The \
         shim's `do ... end` block performs three direct assignments — so the LE_* globals \
         must be identity-equal to the enum table reads"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_world_elapsed_timer_types_le_constants_equal_enum_table_lookups(env: &WowLuaEnv) {

    let all_equal: bool = env
        .eval(
            "return LE_WORLD_ELAPSED_TIMER_TYPE_NONE == Enum.WorldElapsedTimerTypes.None \
                and LE_WORLD_ELAPSED_TIMER_TYPE_CHALLENGE_MODE == Enum.WorldElapsedTimerTypes.ChallengeMode \
                and LE_WORLD_ELAPSED_TIMER_TYPE_PROVING_GROUND == Enum.WorldElapsedTimerTypes.ProvingGround",
        )
        .expect("LE_* vs Enum.WorldElapsedTimerTypes equality query should succeed");
    assert!(
        all_equal,
        "Each deprecated LE_* constant must equal its Enum.WorldElapsedTimerTypes.<Value> \
         counterpart — the shim does direct value assignment (not a wrapper closure), so the \
         globals are simple numeric copies of the enum integer values"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_world_elapsed_timer_types_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_WorldElapsedTimerTypes.lua:4 doesn't \
         bail before the three LE_* globals are reassigned. If this CVar flips to false, \
         the addon's `do ... end` block is skipped and any legacy timer-frame addon reading \
         these constants observes the fallback values seeded at startup by \
         missing_constants.lua:43-44 + 1379"
    );
}
}

#[test]
fn blizzard_deprecated_world_elapsed_timer_types_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedWorldElapsedTimerTypes");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedWorldElapsedTimerTypes dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedWorldElapsedTimerTypes has NO XML files — pure Lua constant-alias \
         shim only. Got entries: {entries:?}"
    );

    let has_runtime_shim = entries
        .iter()
        .any(|n| n == "Deprecated_WorldElapsedTimerTypes.lua");
    assert!(
        has_runtime_shim,
        "Blizzard_DeprecatedWorldElapsedTimerTypes should ship `Deprecated_WorldElapsedTimerTypes.lua` \
         (the runtime shim defining the three LE_WORLD_ELAPSED_TIMER_TYPE_* aliases). Got \
         entries: {entries:?}"
    );
}

#[test]
fn blizzard_deprecated_world_elapsed_timer_types_enum_table_is_pre_populated_at_env_init() {
    let env = WowLuaEnv::new().expect("fresh Lua env should construct");

    let pre_populated: bool = env
        .eval(
            "return type(Enum) == 'table' \
                and type(Enum.WorldElapsedTimerTypes) == 'table' \
                and Enum.WorldElapsedTimerTypes.None == 0 \
                and Enum.WorldElapsedTimerTypes.ChallengeMode == 1 \
                and Enum.WorldElapsedTimerTypes.ProvingGround == 2",
        )
        .expect("Enum.WorldElapsedTimerTypes pre-population probe should succeed");
    assert!(
        pre_populated,
        "Enum.WorldElapsedTimerTypes must be pre-populated at env init (before any addon \
         loads) by the static enum registry at src/lua_api/globals/enum_data/widget.rs:60-63. \
         If this enum isn't seeded, the shim's three lookups all resolve to nil and \
         LE_WORLD_ELAPSED_TIMER_TYPE_* would be set to nil, breaking any consumer reading them"
    );
}
