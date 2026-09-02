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

fn deprecated_battle_net_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeprecatedBattleNet/Blizzard_DeprecatedBattleNet.toc")
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
fn blizzard_deprecated_battle_net_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_battle_net_toc())
        .expect("Blizzard_DeprecatedBattleNet TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedBattleNet declares `## LoadOnDemand: 0` — the BNSendGameData / \
         BNSendWhisper / BNSetCustomMessage globals must install before any chat / messaging \
         addon Lua executes that calls them"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedBattleNet does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedBattleNet declares NO dependencies — every shim simply forwards \
         to the always-loaded C_BattleNet namespace (src/lua_api/globals/missing_surface/\
         battle_net.rs)"
    );

    let toc_text = std::fs::read_to_string(deprecated_battle_net_toc())
        .expect("Blizzard_DeprecatedBattleNet TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedBattleNet omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311), matching the legacy Battle.net messaging in-game-only API surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedBattleNet omits `## AllowLoadGameType:` so the shims install on \
         every game type without restriction"
    );
}

#[test]
fn blizzard_deprecated_battle_net_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedBattleNet");
    assert!(
        in_game,
        "Blizzard_DeprecatedBattleNet (no AllowLoad flag, defaults to Game-only) should \
         appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedBattleNet");
    assert!(
        !in_login,
        "Blizzard_DeprecatedBattleNet should NOT appear on the Login / glue screens — \
         Battle.net messaging is an in-game concept (the glue-screen friends panel uses a \
         separate code path)"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_battle_net_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedBattleNet") || message.contains("Deprecated_BattleNet")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedBattleNet emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_battle_net_installs_function_shims(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return type(BNSendGameData) == 'function' \
                and type(BNSendWhisper) == 'function' \
                and type(BNSetCustomMessage) == 'function'",
        )
        .expect("BN* function-shim installation query should succeed");
    assert!(
        installed,
        "Deprecated_BattleNet.lua line 8-19 should publish 3 forwarding global functions: \
         BNSendGameData(gameAccountID, prefix, data) → C_BattleNet.SendGameData (drops the \
         new API's result-code return value to match the legacy void-return shape — see the \
         documentation comment on line 9); BNSendWhisper(bnetAccountID, text) → \
         C_BattleNet.SendWhisper; BNSetCustomMessage(text) → C_BattleNet.SetCustomMessage"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_battle_net_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_BattleNet.lua:4 doesn't bail before the \
         BN* shims are defined. If this CVar flips to false, all 3 globals are skipped and \
         any legacy chat addon calling BNSendGameData / BNSendWhisper / BNSetCustomMessage \
         blows up with `attempt to call a nil value`"
    );
}
}

#[test]
fn blizzard_deprecated_battle_net_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedBattleNet");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedBattleNet dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedBattleNet has NO XML files — pure Lua function-shim definitions \
         only. Got entries: {entries:?}"
    );

    let has_runtime_shims = entries.iter().any(|n| n == "Deprecated_BattleNet.lua");
    assert!(
        has_runtime_shims,
        "Blizzard_DeprecatedBattleNet should ship `Deprecated_BattleNet.lua` (the runtime \
         shim definitions for BNSendGameData / BNSendWhisper / BNSetCustomMessage)"
    );
}
