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

fn deprecated_lfg_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeprecatedLFG/Blizzard_DeprecatedLFG.toc")
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
fn blizzard_deprecated_lfg_toc_is_minimal_with_no_flags_or_deps() {
    let toc =
        TocFile::from_file(&deprecated_lfg_toc()).expect("Blizzard_DeprecatedLFG TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedLFG declares `## LoadOnDemand: 0` — the in-place namespace \
         mutations on C_LFGInfo / C_LFGList must run before any group-finder addon Lua \
         executes that calls C_LFGInfo.IsPremadeGroupEnabled or \
         C_LFGList.GetSearchResultMemberInfo"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedLFG does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedLFG declares NO dependencies — both target namespaces \
         (C_LFGInfo, C_LFGList) are bootstrapped at env init via runtime_surface_bootstrap.lua \
         and the explicit Rust registrations at src/c_api/c_lfg_info.rs and \
         src/lua_api/globals/lfg_list.rs"
    );

    let toc_text = std::fs::read_to_string(deprecated_lfg_toc())
        .expect("Blizzard_DeprecatedLFG TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedLFG omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311), matching the legacy LFG / Group Finder in-game-only API surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedLFG omits `## AllowLoadGameType:` so the shims install on every \
         game type without restriction"
    );
}

#[test]
fn blizzard_deprecated_lfg_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedLFG");
    assert!(
        in_game,
        "Blizzard_DeprecatedLFG (no AllowLoad flag, defaults to Game-only) should appear in \
         Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedLFG");
    assert!(
        !in_login,
        "Blizzard_DeprecatedLFG should NOT appear on the Login / glue screens — LFG / \
         Group Finder is an in-game concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_lfg_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("DeprecatedLFG") || message.contains("Deprecated_LFG"))
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedLFG emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_lfg_installs_is_premade_group_enabled_on_c_lfg_info(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(C_LFGInfo.IsPremadeGroupEnabled)")
        .expect("type(C_LFGInfo.IsPremadeGroupEnabled) query should succeed");
    assert_eq!(
        kind, "function",
        "Deprecated_LFG.lua line 9 reads `C_LFGInfo.IsPremadeGroupEnabled = C_LFGList.\
         IsPremadeGroupFinderEnabled;` — installing the legacy `IsPremadeGroupEnabled` \
         method on the C_LFGInfo namespace, aliased to C_LFGList's stubbed-with-`return \
         false` IsPremadeGroupFinderEnabled (runtime_surface_bootstrap.lua:2307). Cross-\
         namespace identity assignment — the same function value appears under two distinct \
         API homes"
    );

    let aliased: bool = env
        .eval("return C_LFGInfo.IsPremadeGroupEnabled == C_LFGList.IsPremadeGroupFinderEnabled")
        .expect("identity-equality query should succeed");
    assert!(
        aliased,
        "C_LFGInfo.IsPremadeGroupEnabled must be IDENTITY-EQUAL to \
         C_LFGList.IsPremadeGroupFinderEnabled — both sides reference the same function \
         value (the explicit `return false` closure from runtime_surface_bootstrap.lua:2307)"
    );

    let returned_false: bool = env
        .eval("return C_LFGInfo.IsPremadeGroupEnabled() == false")
        .expect("invocation query should succeed");
    assert!(
        returned_false,
        "Calling C_LFGInfo.IsPremadeGroupEnabled() must return false (delegated to the \
         IsPremadeGroupFinderEnabled stub at runtime_surface_bootstrap.lua:2307 which is \
         literally `function() return false end`)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_lfg_installs_get_search_result_member_info_wrapper_on_c_lfg_list(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(C_LFGList.GetSearchResultMemberInfo)")
        .expect("type(C_LFGList.GetSearchResultMemberInfo) query should succeed");
    assert_eq!(
        kind, "function",
        "Deprecated_LFG.lua line 10-15 installs a wrapper closure on \
         C_LFGList.GetSearchResultMemberInfo that calls C_LFGList.GetSearchResultPlayerInfo \
         (the new-API table-returning method) and destructures its `info` table into the \
         legacy 5-multireturn-value shape (assignedRole, classFilename, className, \
         specName, isLeader)"
    );

    let returned_nothing: bool = env
        .eval(
            "do \
                local a, b, c, d, e = C_LFGList.GetSearchResultMemberInfo(0, 1) \
                return a == nil and b == nil and c == nil and d == nil and e == nil \
            end",
        )
        .expect("invocation query should succeed");
    assert!(
        returned_nothing,
        "Calling C_LFGList.GetSearchResultMemberInfo with arbitrary args must return all \
         nils because the underlying C_LFGList.GetSearchResultPlayerInfo is NOT explicitly \
         stubbed — it resolves via the C_LFGList namespace `__index` metamethod \
         (runtime_surface_bootstrap.lua:2019-2028) to a no-op closure that returns nil. With \
         info=nil, the `if (info) then` branch at line 12 is skipped, so the wrapper returns \
         nothing (all destructured locals are nil)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_lfg_does_not_install_legacy_globals(env: &WowLuaEnv) {

    let no_legacy_globals: bool = env
        .eval(
            "return _G.IsPremadeGroupEnabled == nil \
                and _G.GetSearchResultMemberInfo == nil",
        )
        .expect("global-namespace probe query should succeed");
    assert!(
        no_legacy_globals,
        "Blizzard_DeprecatedLFG does NOT install any legacy globals — unlike most \
         Blizzard_Deprecated* shims which publish `_G.<LegacyName> = C_*.NewMethod`, this \
         shim performs in-place mutations on the C_LFGInfo and C_LFGList namespaces only. \
         `_G.IsPremadeGroupEnabled` and `_G.GetSearchResultMemberInfo` should remain nil"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_lfg_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_LFG.lua:4 doesn't bail before the \
         two namespace mutations run. If this CVar flips to false, neither \
         C_LFGInfo.IsPremadeGroupEnabled nor C_LFGList.GetSearchResultMemberInfo is \
         installed, and any legacy group-finder addon calling them blows up — \
         GetSearchResultMemberInfo via __index would actually still return a no-op closure \
         (because the namespace __index materializes one), but its multireturn shape would \
         silently differ from the legacy 5-tuple"
    );
}
}

#[test]
fn blizzard_deprecated_lfg_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedLFG");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedLFG dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedLFG has NO XML files — pure Lua namespace-mutation definitions \
         only. Got entries: {entries:?}"
    );

    let has_runtime_shims = entries.iter().any(|n| n == "Deprecated_LFG.lua");
    assert!(
        has_runtime_shims,
        "Blizzard_DeprecatedLFG should ship `Deprecated_LFG.lua` (the runtime in-place \
         namespace mutations for the 2 deprecated LFG methods)"
    );
}
