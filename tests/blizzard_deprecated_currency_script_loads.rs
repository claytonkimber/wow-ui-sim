#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path()
        .expect("Blizzard UI cache should be available")
}

fn deprecated_currency_script_toc() -> PathBuf {
    blizzard_ui_dir()
        .join("Blizzard_DeprecatedCurrencyScript/Blizzard_DeprecatedCurrencyScript.toc")
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
fn blizzard_deprecated_currency_script_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_currency_script_toc())
        .expect("Blizzard_DeprecatedCurrencyScript TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedCurrencyScript declares `## LoadOnDemand: 0` — the GetCoinIcon / \
         GetCoinText / GetCoinTextureString globals must install before any tooltip / vendor / \
         money-frame addon Lua executes that calls them"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedCurrencyScript does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedCurrencyScript declares NO dependencies — every shim simply \
         forwards to the always-loaded C_CurrencyInfo namespace (src/c_api/item_spell/\
         c_currency.rs)"
    );

    let toc_text = std::fs::read_to_string(deprecated_currency_script_toc())
        .expect("Blizzard_DeprecatedCurrencyScript TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedCurrencyScript omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311), matching the legacy coin-text in-game-only API surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedCurrencyScript omits `## AllowLoadGameType:` so the shims install \
         on every game type without restriction"
    );
}

#[test]
fn blizzard_deprecated_currency_script_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedCurrencyScript");
    assert!(
        in_game,
        "Blizzard_DeprecatedCurrencyScript (no AllowLoad flag, defaults to Game-only) should \
         appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedCurrencyScript");
    assert!(
        !in_login,
        "Blizzard_DeprecatedCurrencyScript should NOT appear on the Login / glue screens — coin \
         text is an in-game tooltip / vendor concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_currency_script_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedCurrencyScript")
                || message.contains("Deprecated_CurrencyScript")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedCurrencyScript emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_currency_script_installs_coin_texture_string_global(env: &WowLuaEnv) {

    let installed: bool = env
        .eval("return type(GetCoinTextureString) == 'function'")
        .expect("GetCoinTextureString global query should succeed");
    assert!(
        installed,
        "Deprecated_CurrencyScript.lua line 11 should publish `GetCoinTextureString` as a \
         direct alias of `C_CurrencyInfo.GetCoinTextureString` (src/c_api/item_spell/\
         c_currency.rs:18). This is the only one of the 3 shims whose backing C-API \
         implementation exists in the simulator — the other two (GetCoinIcon / GetCoinText) \
         are unstubbed and will alias to nil"
    );

    let result: String = env
        .eval("return GetCoinTextureString(12345)")
        .expect("GetCoinTextureString(12345) should succeed");
    assert_eq!(
        result, "12345",
        "GetCoinTextureString should return the amount as a string per the simulator's stub \
         implementation (src/c_api/item_spell/c_currency.rs:77)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_currency_script_globals_alias_c_currency_info_methods(env: &WowLuaEnv) {

    let coin_icon_matches: bool = env
        .eval("return GetCoinIcon == C_CurrencyInfo.GetCoinIcon")
        .expect("GetCoinIcon == C_CurrencyInfo.GetCoinIcon query should succeed");
    assert!(
        coin_icon_matches,
        "Deprecated_CurrencyScript.lua line 9 should set `GetCoinIcon = C_CurrencyInfo.\
         GetCoinIcon` — both must reference the same value (nil-equals-nil included if the \
         backing method is unstubbed; this is intentional — the shim must not error at load \
         time even when C_CurrencyInfo.GetCoinIcon is missing)"
    );

    let coin_text_matches: bool = env
        .eval("return GetCoinText == C_CurrencyInfo.GetCoinText")
        .expect("GetCoinText == C_CurrencyInfo.GetCoinText query should succeed");
    assert!(
        coin_text_matches,
        "Deprecated_CurrencyScript.lua line 10 should set `GetCoinText = C_CurrencyInfo.\
         GetCoinText` — both must reference the same value (nil-equals-nil included if the \
         backing method is unstubbed)"
    );

    let coin_texture_string_matches: bool = env
        .eval("return GetCoinTextureString == C_CurrencyInfo.GetCoinTextureString")
        .expect("GetCoinTextureString == C_CurrencyInfo.GetCoinTextureString query should succeed");
    assert!(
        coin_texture_string_matches,
        "Deprecated_CurrencyScript.lua line 11 should set `GetCoinTextureString = \
         C_CurrencyInfo.GetCoinTextureString` — both must point to the same Rust function"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_currency_script_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_CurrencyScript.lua:4 doesn't bail \
         before the GetCoinIcon / GetCoinText / GetCoinTextureString aliases are defined. \
         If this CVar flips to false, the 3 globals are skipped and any legacy money-frame / \
         tooltip addon calling GetCoinTextureString blows up with `attempt to call a nil value`"
    );
}
}

#[test]
fn blizzard_deprecated_currency_script_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedCurrencyScript");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedCurrencyScript dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedCurrencyScript has NO XML files — pure Lua function-shim \
         definitions only. Got entries: {entries:?}"
    );

    let has_runtime_shims = entries.iter().any(|n| n == "Deprecated_CurrencyScript.lua");
    assert!(
        has_runtime_shims,
        "Blizzard_DeprecatedCurrencyScript should ship `Deprecated_CurrencyScript.lua` (the \
         runtime shim definitions for GetCoinIcon / GetCoinText / GetCoinTextureString)"
    );
}
