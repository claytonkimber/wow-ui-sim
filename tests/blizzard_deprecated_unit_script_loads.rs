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

fn deprecated_unit_script_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeprecatedUnitScript/Blizzard_DeprecatedUnitScript.toc")
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
fn blizzard_deprecated_unit_script_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_unit_script_toc())
        .expect("Blizzard_DeprecatedUnitScript TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedUnitScript declares `## LoadOnDemand: 0` — the \
         ShowBossFrameWhenUninteractable global must install before any boss-frame addon \
         Lua executes that calls it"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedUnitScript does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedUnitScript declares NO dependencies — the single shim forwards \
         to UnitIsVisible (registered as a global at \
         src/lua_api/globals/group_queries.rs:100), bootstrapped at env init"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_DeprecatedUnitScript declares no `## AllowLoadGameType:` filter at the \
         addon level — `is_game_type_restricted()` returns false. Note that the second \
         file entry has a PER-FILE `[AllowLoadGameType classic]` annotation, but that \
         filters individual file inclusion, not whole-addon load"
    );

    let toc_text = std::fs::read_to_string(deprecated_unit_script_toc())
        .expect("Blizzard_DeprecatedUnitScript TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedUnitScript omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311)"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedUnitScript omits the addon-level `## AllowLoadGameType:` \
         metadata — only a PER-FILE annotation appears on line 8"
    );
}

#[test]
fn blizzard_deprecated_unit_script_toc_filters_classic_overrides_file_on_mainline() {
    let toc = TocFile::from_file(&deprecated_unit_script_toc())
        .expect("Blizzard_DeprecatedUnitScript TOC should parse");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    assert_eq!(
        files.len(),
        1,
        "On mainline retail, the TOC should yield exactly 1 file entry — \
         `Deprecated_UnitScript.lua`. The 2nd line `[Family]\\Deprecated_UnitScriptOverrides.lua \
         [AllowLoadGameType classic]` is FILTERED OUT by `is_allowed_game_type` \
         (src/toc.rs:43-57): the per-file `[AllowLoadGameType classic]` annotation \
         specifies only `classic`, which is neither `mainline` nor `standard`, so \
         push_file_entry skips this entry at toc.rs:141-143. Got: {files:?}"
    );
    assert_eq!(
        files[0], "Deprecated_UnitScript.lua",
        "The single non-filtered file should be the top-level Deprecated_UnitScript.lua. \
         Got: {files:?}"
    );

    let toc_text = std::fs::read_to_string(deprecated_unit_script_toc())
        .expect("Blizzard_DeprecatedUnitScript TOC should read");
    assert!(
        toc_text
            .contains("[Family]\\Deprecated_UnitScriptOverrides.lua [AllowLoadGameType classic]"),
        "TOC text MUST contain the per-file `[Family]\\Deprecated_UnitScriptOverrides.lua \
         [AllowLoadGameType classic]` line — proving that the filter is applied by parser, \
         not absent from the TOC entirely"
    );
}

#[test]
fn blizzard_deprecated_unit_script_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedUnitScript");
    assert!(
        in_game,
        "Blizzard_DeprecatedUnitScript (no AllowLoad flag, defaults to Game-only) should \
         appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedUnitScript");
    assert!(
        !in_login,
        "Blizzard_DeprecatedUnitScript should NOT appear on the Login / glue screens — \
         unit / boss-frame management is an in-game concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_unit_script_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedUnitScript") || message.contains("Deprecated_UnitScript")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedUnitScript emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_unit_script_installs_show_boss_frame_when_uninteractable(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(ShowBossFrameWhenUninteractable)")
        .expect("type(ShowBossFrameWhenUninteractable) query should succeed");
    assert_eq!(
        kind, "function",
        "Deprecated_UnitScript.lua line 9 declares `function \
         ShowBossFrameWhenUninteractable(unit) return UnitIsVisible(unit); end` — a \
         locally-defined wrapper closure inside a `do ... end` block. The shim returns \
         the result of UnitIsVisible (registered at \
         src/lua_api/globals/group_queries.rs:100) — repurposing the visibility query as \
         the boss-frame-uninteractable predicate (legacy behavior was hard-coded on; the \
         shim now defers to visibility)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_unit_script_show_boss_frame_returns_true_for_player(env: &WowLuaEnv) {

    let visible: bool = env
        .eval("return ShowBossFrameWhenUninteractable('player')")
        .expect("ShowBossFrameWhenUninteractable('player') query should succeed");
    assert!(
        visible,
        "Calling ShowBossFrameWhenUninteractable('player') must return true. The wrapper \
         delegates to UnitIsVisible (group_queries.rs:881) which hard-codes 'player' / \
         'pet' / 'vehicle' as always visible — no fog-of-war / range modeling in the sim"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_unit_script_show_boss_frame_returns_false_for_empty_string(env: &WowLuaEnv) {

    let visible: bool = env
        .eval("return ShowBossFrameWhenUninteractable('')")
        .expect("ShowBossFrameWhenUninteractable('') query should succeed");
    assert!(
        !visible,
        "Calling ShowBossFrameWhenUninteractable('') must return false because \
         UnitIsVisible (group_queries.rs:880) explicitly returns false for the empty unit \
         string. This documents the wrapper's straight passthrough semantics"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_unit_script_classic_overrides_globals_do_not_leak_on_mainline(env: &WowLuaEnv) {

    let no_classic_globals: bool = env
        .eval("return _G.CastingInfo == nil and _G.ChannelInfo == nil")
        .expect("Classic-only-globals probe should succeed");
    assert!(
        no_classic_globals,
        "On mainline retail, neither CastingInfo nor ChannelInfo should exist as globals. \
         Both are defined only in Classic/Deprecated_UnitScriptOverrides.lua, which is \
         FILTERED OUT of toc.files by `is_allowed_game_type` because its per-file \
         annotation `[AllowLoadGameType classic]` doesn't include `mainline`/`standard`. \
         If these globals leak, the file filter is broken"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_unit_script_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_UnitScript.lua:4 doesn't bail before \
         ShowBossFrameWhenUninteractable is defined"
    );
}
}

#[test]
fn blizzard_deprecated_unit_script_ships_classic_overrides_subdir_and_top_level_lua() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedUnitScript");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedUnitScript dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedUnitScript has NO XML files — pure Lua function-shim \
         definitions only. Got entries: {entries:?}"
    );

    assert!(
        entries.iter().any(|n| n == "Deprecated_UnitScript.lua"),
        "Blizzard_DeprecatedUnitScript should ship `Deprecated_UnitScript.lua` (the \
         mainline shim with ShowBossFrameWhenUninteractable). Got entries: {entries:?}"
    );
    assert!(
        entries.iter().any(|n| n == "Classic"),
        "Blizzard_DeprecatedUnitScript should ship a `Classic/` subdirectory containing \
         the classic-only Deprecated_UnitScriptOverrides.lua (CastingInfo / ChannelInfo). \
         The top-level TOC's `[Family]` template resolves to `Mainline` on retail \
         (src/toc.rs:145), so the Classic file path would resolve to \
         `Classic/Deprecated_UnitScriptOverrides.lua` only if `[Family]` were `Classic`. \
         Got entries: {entries:?}"
    );

    let classic_overrides = dir.join("Classic/Deprecated_UnitScriptOverrides.lua");
    assert!(
        classic_overrides.exists(),
        "Classic/Deprecated_UnitScriptOverrides.lua should exist on disk even though it \
         is filtered out of mainline loads — confirms the addon ships both variants"
    );
}
