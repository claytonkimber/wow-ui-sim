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

fn deprecated_glue_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeprecatedGlue/Blizzard_DeprecatedGlue.toc")
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
fn blizzard_deprecated_glue_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_glue_toc())
        .expect("Blizzard_DeprecatedGlue TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedGlue declares `## LoadOnDemand: 0` — the IsOnGlueScreen global \
         must install before any addon Lua executes that reads it"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedGlue does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedGlue declares NO dependencies — the single shim simply reads \
         from the always-loaded C_Glue namespace registered by src/c_api/c_glue.rs"
    );

    let toc_text = std::fs::read_to_string(deprecated_glue_toc())
        .expect("Blizzard_DeprecatedGlue TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedGlue omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311). NOTE: this means `IsOnGlueScreen` is materialized to `false` at \
         in-game load time and never refreshes — matching real WoW where the legacy global \
         was a snapshot, not a live query"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedGlue omits `## AllowLoadGameType:` so the shim installs on every \
         game type without restriction"
    );
}

#[test]
fn blizzard_deprecated_glue_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedGlue");
    assert!(
        in_game,
        "Blizzard_DeprecatedGlue (no AllowLoad flag, defaults to Game-only) should appear in \
         Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedGlue");
    assert!(
        !in_login,
        "Blizzard_DeprecatedGlue should NOT appear on the Login / glue screens — the legacy \
         IsOnGlueScreen global was an in-game snapshot only; glue-screen Lua never read it"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_glue_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("DeprecatedGlue") || message.contains("Deprecated_Glue"))
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedGlue emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_glue_publishes_is_on_glue_screen_as_boolean_not_function(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(IsOnGlueScreen)")
        .expect("type(IsOnGlueScreen) query should succeed");
    assert_eq!(
        kind, "boolean",
        "Deprecated_Glue.lua line 9 reads `IsOnGlueScreen = C_Glue.IsOnGlueScreen();` — note \
         the trailing `()` invocation. The legacy global is materialized as the BOOLEAN return \
         value (a snapshot of the screen mode at load time), NOT as a function reference. This \
         is intentional in Blizzard's source — legacy callers wrote `if IsOnGlueScreen then` \
         expecting a value-typed boolean, not `if IsOnGlueScreen() then`. type() should report \
         'boolean'"
    );

    let value: bool = env
        .eval("return IsOnGlueScreen")
        .expect("read of IsOnGlueScreen should succeed");
    assert!(
        !value,
        "When this addon loads on the Game screen (the only screen it auto-discovers on), \
         `C_Glue.IsOnGlueScreen()` returns false from the SimState-backed C_Glue surface. \
         Therefore the materialized `IsOnGlueScreen` boolean must be false in this load context"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_glue_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_Glue.lua:4 doesn't bail before \
         `IsOnGlueScreen` is defined. If this CVar flips to false, any legacy addon reading \
         `if IsOnGlueScreen then` gets `nil` instead of `false`, which is still falsy — but \
         any code doing explicit `IsOnGlueScreen == false` comparisons would break"
    );
}
}

#[test]
fn blizzard_deprecated_glue_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedGlue");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedGlue dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedGlue has NO XML files — pure Lua single-shim definition only. \
         Got entries: {entries:?}"
    );

    let has_runtime_shims = entries.iter().any(|n| n == "Deprecated_Glue.lua");
    assert!(
        has_runtime_shims,
        "Blizzard_DeprecatedGlue should ship `Deprecated_Glue.lua` (the runtime shim that \
         materializes IsOnGlueScreen as a boolean snapshot)"
    );
}
