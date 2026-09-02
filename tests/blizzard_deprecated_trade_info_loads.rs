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

fn deprecated_trade_info_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeprecatedTradeInfo/Blizzard_DeprecatedTradeInfo.toc")
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
fn blizzard_deprecated_trade_info_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_trade_info_toc())
        .expect("Blizzard_DeprecatedTradeInfo TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedTradeInfo declares `## LoadOnDemand: 0` — the single \
         PickupTradeMoney global must install before any trade-frame addon Lua executes \
         that calls it"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedTradeInfo does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedTradeInfo declares NO dependencies — the single shim forwards \
         to C_TradeInfo, which is bootstrapped at env init via \
         runtime_surface_bootstrap.lua:9491-9498 (creates the namespace if missing, \
         explicitly stubs only ShouldShowTradeOfferWarning — PickupTradeMoney is NOT \
         pre-stubbed and resolves via `__wow_namespace_mt.__index` to a cached no-op)"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_DeprecatedTradeInfo declares no `## AllowLoadGameType:` filter — loads on \
         every game type without restriction"
    );

    let toc_text = std::fs::read_to_string(deprecated_trade_info_toc())
        .expect("Blizzard_DeprecatedTradeInfo TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedTradeInfo omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311), matching the legacy trade API in-game-only surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedTradeInfo omits `## AllowLoadGameType:` — single-line shim is \
         universal across game types"
    );
}

#[test]
fn blizzard_deprecated_trade_info_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedTradeInfo");
    assert!(
        in_game,
        "Blizzard_DeprecatedTradeInfo (no AllowLoad flag, defaults to Game-only) should \
         appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedTradeInfo");
    assert!(
        !in_login,
        "Blizzard_DeprecatedTradeInfo should NOT appear on the Login / glue screens — \
         player-to-player trade is an in-game concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_trade_info_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedTradeInfo") || message.contains("Deprecated_TradeInfo")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedTradeInfo emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_trade_info_installs_pickup_trade_money_as_function(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(PickupTradeMoney)")
        .expect("type(PickupTradeMoney) query should succeed");
    assert_eq!(
        kind, "function",
        "Deprecated_TradeInfo.lua line 8 reads `PickupTradeMoney = function(amount) \
         C_TradeInfo.PickupTradeMoney(amount); end`. Note that this addon is unique among \
         the deprecated shims in two ways: (1) it uses a function-expression-assignment \
         pattern instead of `function GlobalName(...) ... end` syntax; (2) it has NO \
         `do ... end` wrapper around the assignment (the single statement is at file \
         top-level). The result is the same: a single global function value"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_trade_info_pickup_trade_money_is_not_identity_equal_to_c_trade_info(env: &WowLuaEnv) {

    let not_identity_equal: bool = env
        .eval("return PickupTradeMoney ~= C_TradeInfo.PickupTradeMoney")
        .expect("non-identity-equality query should succeed");
    assert!(
        not_identity_equal,
        "PickupTradeMoney must NOT be identity-equal to C_TradeInfo.PickupTradeMoney. The \
         shim defines a fresh wrapper closure that delegates to C_TradeInfo.PickupTradeMoney \
         (which itself resolves via `__wow_namespace_mt.__index` to a cached no-op closure). \
         The wrapper and the underlying namespace lookup are distinct function values"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_trade_info_pickup_trade_money_swallows_return_value(env: &WowLuaEnv) {

    let returns_nothing: bool = env
        .eval(
            "do \
                local ret = PickupTradeMoney(0) \
                return ret == nil \
            end",
        )
        .expect("PickupTradeMoney invocation query should succeed");
    assert!(
        returns_nothing,
        "Calling PickupTradeMoney(0) must return nil because the wrapper at line 9 \
         intentionally OMITS the `return` keyword (`C_TradeInfo.PickupTradeMoney(amount);` \
         — no `return` prefix). Even if the underlying no-op closure returned a value, the \
         wrapper would discard it. This matches the real API which returns nothing"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_trade_info_pickup_trade_money_calls_without_error(env: &WowLuaEnv) {

    let no_error: bool = env
        .eval("return pcall(PickupTradeMoney, 1000)")
        .expect("pcall probe should succeed");
    assert!(
        no_error,
        "Calling PickupTradeMoney(1000) must NOT throw. The wrapper delegates to \
         C_TradeInfo.PickupTradeMoney which resolves via `__wow_namespace_mt.__index` \
         (runtime_surface_bootstrap.lua:2019-2028) to a no-op `function() return nil end` \
         closure. pcall returns ok=true"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_trade_info_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_TradeInfo.lua:4 doesn't bail before \
         PickupTradeMoney is defined. If this CVar flips to false, the global is skipped \
         and any legacy trade-frame addon calling it blows up with `attempt to call a nil \
         value`"
    );
}
}

#[test]
fn blizzard_deprecated_trade_info_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedTradeInfo");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedTradeInfo dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedTradeInfo has NO XML files — pure Lua single-line shim only. \
         Got entries: {entries:?}"
    );

    let has_runtime_shims = entries.iter().any(|n| n == "Deprecated_TradeInfo.lua");
    assert!(
        has_runtime_shims,
        "Blizzard_DeprecatedTradeInfo should ship `Deprecated_TradeInfo.lua` (the runtime \
         shim definition for the single deprecated PickupTradeMoney global)"
    );
}
