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

fn deprecated_instance_encounter_toc() -> PathBuf {
    blizzard_ui_dir()
        .join("Blizzard_DeprecatedInstanceEncounter/Blizzard_DeprecatedInstanceEncounter.toc")
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
fn blizzard_deprecated_instance_encounter_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_instance_encounter_toc())
        .expect("Blizzard_DeprecatedInstanceEncounter TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedInstanceEncounter declares `## LoadOnDemand: 0` — the \
         IsEncounterInProgress / IsEncounterSuppressingRelease / \
         IsEncounterLimitingResurrections globals must install before any encounter / \
         release / resurrection-gating addon Lua executes that calls them"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedInstanceEncounter does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedInstanceEncounter declares NO dependencies — every shim simply \
         forwards to the C_InstanceEncounter namespace (the shim defines closures that \
         dereference C_InstanceEncounter only at call time, not at module-eval time)"
    );

    let toc_text = std::fs::read_to_string(deprecated_instance_encounter_toc())
        .expect("Blizzard_DeprecatedInstanceEncounter TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedInstanceEncounter omits `## AllowLoad:` — defaults to \
         Game-screen-only (src/toc.rs:311), matching the legacy encounter in-game-only API \
         surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedInstanceEncounter omits `## AllowLoadGameType:` so the shims \
         install on every game type without restriction"
    );
}

#[test]
fn blizzard_deprecated_instance_encounter_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedInstanceEncounter");
    assert!(
        in_game,
        "Blizzard_DeprecatedInstanceEncounter (no AllowLoad flag, defaults to Game-only) \
         should appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedInstanceEncounter");
    assert!(
        !in_login,
        "Blizzard_DeprecatedInstanceEncounter should NOT appear on the Login / glue \
         screens — encounter / release / resurrection gating is an in-game concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_instance_encounter_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedInstanceEncounter")
                || message.contains("Deprecated_InstanceEncounter")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedInstanceEncounter emitted Lua errors during Game-screen load:\n  \
         {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_instance_encounter_installs_function_shims(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return type(IsEncounterInProgress) == 'function' \
                and type(IsEncounterSuppressingRelease) == 'function' \
                and type(IsEncounterLimitingResurrections) == 'function'",
        )
        .expect("encounter-shim function-installation query should succeed");
    assert!(
        installed,
        "Deprecated_InstanceEncounter.lua line 8-18 should publish 3 forwarding global \
         functions: IsEncounterInProgress (overwrites the Rust-registered global at \
         src/lua_api/globals/combat_probes.rs:62 with a Lua closure that delegates to \
         C_InstanceEncounter.IsEncounterInProgress); IsEncounterSuppressingRelease (new \
         global, delegates to C_InstanceEncounter.IsEncounterSuppressingRelease); \
         IsEncounterLimitingResurrections (new global, delegates to \
         C_InstanceEncounter.IsEncounterLimitingResurrections). Note: the closures are \
         defined at module-eval time but reference C_InstanceEncounter only at call time \
         — the namespace itself is currently unstubbed in this simulator, so calling these \
         shims would error with `attempt to index a nil value`. Load-time correctness only \
         requires the closures to install"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_instance_encounter_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_InstanceEncounter.lua:4 doesn't bail \
         before the 3 closures install. If this CVar flips to false, the Lua-side closures \
         are skipped and IsEncounterInProgress remains the original Rust-registered \
         function (which is actually preferable — the Rust impl reads SimState directly), \
         while IsEncounterSuppressingRelease / IsEncounterLimitingResurrections never \
         install at all"
    );
}
}

#[test]
fn blizzard_deprecated_instance_encounter_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedInstanceEncounter");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedInstanceEncounter dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedInstanceEncounter has NO XML files — pure Lua function-shim \
         definitions only. Got entries: {entries:?}"
    );

    let has_runtime_shims = entries
        .iter()
        .any(|n| n == "Deprecated_InstanceEncounter.lua");
    assert!(
        has_runtime_shims,
        "Blizzard_DeprecatedInstanceEncounter should ship `Deprecated_InstanceEncounter.lua` \
         (the runtime shim definitions for the 3 deprecated encounter-state globals)"
    );
}
