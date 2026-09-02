#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::paths::default_blizzard_ui_addons_path;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn deprecated_pvp_script_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeprecatedPvpScript/Blizzard_DeprecatedPvpScript.toc")
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
fn blizzard_deprecated_pvp_script_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_pvp_script_toc())
        .expect("Blizzard_DeprecatedPvpScript TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedPvpScript declares `## LoadOnDemand: 0` — the IsSubZonePVPPOI / \
         GetZonePVPInfo / TogglePVP / SetPVP globals must install before any PVP-frame \
         addon Lua executes that calls them"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedPvpScript does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedPvpScript declares NO dependencies — every shim simply forwards \
         to the C_PvP namespace, which is bootstrapped at env init by \
         src/lua_api/globals/missing_surface/small_namespaces.rs:30 (registers ~24 explicit \
         methods via C_PVP_METHODS) plus src/lua_api/globals/zone_text.rs:131 (GetZonePVPInfo \
         backed by SimState::world)"
    );

    let toc_text = std::fs::read_to_string(deprecated_pvp_script_toc())
        .expect("Blizzard_DeprecatedPvpScript TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedPvpScript omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311), matching the legacy PVP API in-game-only surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedPvpScript omits `## AllowLoadGameType:` so the shims install on \
         every game type without restriction"
    );
}

#[test]
fn blizzard_deprecated_pvp_script_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedPvpScript");
    assert!(
        in_game,
        "Blizzard_DeprecatedPvpScript (no AllowLoad flag, defaults to Game-only) should \
         appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedPvpScript");
    assert!(
        !in_login,
        "Blizzard_DeprecatedPvpScript should NOT appear on the Login / glue screens — PVP \
         management is an in-game concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_pvp_script_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedPvpScript") || message.contains("Deprecated_PvpScript")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedPvpScript emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_pvp_script_installs_four_function_shims(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return type(IsSubZonePVPPOI) == 'function' \
                and type(GetZonePVPInfo) == 'function' \
                and type(TogglePVP) == 'function' \
                and type(SetPVP) == 'function'",
        )
        .expect("4-shim function-installation query should succeed");
    assert!(
        installed,
        "Deprecated_PvpScript.lua line 9-12 should publish 4 forwarding global functions: \
         IsSubZonePVPPOI → C_PvP.IsSubZonePVPPOI (NOT in C_PVP_METHODS — note the POI \
         suffix; `IsSubZonePVP` IS registered but `IsSubZonePVPPOI` is NOT — resolves via \
         namespace `__index` to a no-op closure); GetZonePVPInfo → C_PvP.GetZonePVPInfo \
         (real Rust impl at src/lua_api/globals/zone_text.rs:131 returning the (pvpType, \
         isSubZonePvP, factionName) 3-tuple from SimState::world); TogglePVP → \
         C_PvP.TogglePVP (NOT registered, resolves to no-op closure); SetPVP → C_PvP.SetPVP \
         (NOT registered, resolves to no-op closure). All 4 install as functions"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_pvp_script_globals_alias_c_pvp_methods_by_identity(env: &WowLuaEnv) {

    let aliases_match: bool = env
        .eval(
            "return IsSubZonePVPPOI == C_PvP.IsSubZonePVPPOI \
                and GetZonePVPInfo == C_PvP.GetZonePVPInfo \
                and TogglePVP == C_PvP.TogglePVP \
                and SetPVP == C_PvP.SetPVP",
        )
        .expect("identity-equality query for all 4 aliases should succeed");
    assert!(
        aliases_match,
        "Each deprecated global must reference identity-equal values to its backing C_PvP \
         method. For `GetZonePVPInfo` (registered Rust impl), both sides see the same \
         function value. For the 3 unstubbed methods (IsSubZonePVPPOI, TogglePVP, SetPVP), \
         the namespace `__index` caches the no-op closure via `rawset(t, key, fn)` so \
         subsequent reads return the SAME function. Identity equality confirms both sides \
         see the same value object"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_pvp_script_get_zone_pvp_info_returns_real_three_tuple(env: &WowLuaEnv) {

    let three_tuple_returned: bool = env
        .eval(
            "do \
                local pvpType, isSubZonePvP, factionName = GetZonePVPInfo() \
                return pvpType ~= nil and type(pvpType) == 'string' \
            end",
        )
        .expect("GetZonePVPInfo invocation query should succeed");
    assert!(
        three_tuple_returned,
        "GetZonePVPInfo() (aliased to C_PvP.GetZonePVPInfo) must return a real 3-tuple \
         from src/lua_api/globals/zone_text.rs:101+ — pvpType (string from \
         SimState.world.zone_pvp_type), isSubZonePvP (bool), factionName (string|nil). \
         pvpType is non-nil and a string in the default character world state. This \
         confirms the alias correctly forwards through to the Rust-implemented method, NOT \
         to a synthesized no-op closure"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_pvp_script_unstubbed_shims_call_without_error(env: &WowLuaEnv) {

    let no_error: bool = env
        .eval(
            "do \
                local ok1 = pcall(IsSubZonePVPPOI) \
                local ok2 = pcall(TogglePVP) \
                local ok3 = pcall(SetPVP, false) \
                return ok1 and ok2 and ok3 \
            end",
        )
        .expect("pcall probe of unstubbed shims should succeed");
    assert!(
        no_error,
        "Calling the 3 unstubbed shims (IsSubZonePVPPOI, TogglePVP, SetPVP) must NOT throw \
         — they resolve through `__wow_namespace_mt.__index` to no-op closures \
         `function() return nil end` (runtime_surface_bootstrap.lua:2019-2028). All 3 \
         pcalls return ok=true with nil result"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_pvp_script_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_PvpScript.lua:4 doesn't bail before \
         the 4 PVP globals are defined. If this CVar flips to false, all 4 are skipped and \
         any legacy PVP-frame addon calling GetZonePVPInfo / TogglePVP / SetPVP / \
         IsSubZonePVPPOI blows up with `attempt to call a nil value`"
    );
}
}

#[test]
fn blizzard_deprecated_pvp_script_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedPvpScript");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedPvpScript dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedPvpScript has NO XML files — pure Lua function-shim \
         definitions only. Got entries: {entries:?}"
    );

    let has_runtime_shims = entries.iter().any(|n| n == "Deprecated_PvpScript.lua");
    assert!(
        has_runtime_shims,
        "Blizzard_DeprecatedPvpScript should ship `Deprecated_PvpScript.lua` (the runtime \
         shim definitions for the 4 deprecated PVP globals)"
    );
}
