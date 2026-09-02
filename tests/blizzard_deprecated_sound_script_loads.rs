#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path()
        .expect("Blizzard UI cache should be synced")

}

fn deprecated_sound_script_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeprecatedSoundScript/Blizzard_DeprecatedSoundScript.toc")
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
fn blizzard_deprecated_sound_script_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_sound_script_toc())
        .expect("Blizzard_DeprecatedSoundScript TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedSoundScript declares `## LoadOnDemand: 0` — the \
         PlayVocalErrorSoundID global must install before any vocal-error-sound addon Lua \
         executes that calls it"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedSoundScript does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedSoundScript declares NO dependencies — the single shim simply \
         forwards to C_Sound, which is bootstrapped at env init by the explicit \
         temporary sound workaround"
    );

    let toc_text = std::fs::read_to_string(deprecated_sound_script_toc())
        .expect("Blizzard_DeprecatedSoundScript TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedSoundScript omits `## AllowLoad:` — defaults to \
         Game-screen-only (src/toc.rs:311), matching the legacy vocal-error-sound API \
         in-game-only surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedSoundScript omits `## AllowLoadGameType:` so the shim installs \
         on every game type without restriction"
    );
}

#[test]
fn blizzard_deprecated_sound_script_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedSoundScript");
    assert!(
        in_game,
        "Blizzard_DeprecatedSoundScript (no AllowLoad flag, defaults to Game-only) should \
         appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedSoundScript");
    assert!(
        !in_login,
        "Blizzard_DeprecatedSoundScript should NOT appear on the Login / glue screens — \
         vocal error sounds are an in-game concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_sound_script_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedSoundScript") || message.contains("Deprecated_SoundScript")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedSoundScript emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_sound_script_play_vocal_error_sound_id_aliases_play_vocal_error_sound(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(PlayVocalErrorSoundID)")
        .expect("type(PlayVocalErrorSoundID) query should succeed");
    assert_eq!(
        kind, "function",
        "Deprecated_SoundScript.lua line 9 reads `PlayVocalErrorSoundID = \
         C_Sound.PlayVocalErrorSound;` — note the RENAMED right-hand side: legacy global \
         `PlayVocalErrorSoundID` is bound to the new API name `C_Sound.PlayVocalErrorSound` \
         (without the `ID` suffix). C_Sound.PlayVocalErrorSound is an explicit \
         temporary no-op sound workaround"
    );

    let aliased: bool = env
        .eval("return PlayVocalErrorSoundID == C_Sound.PlayVocalErrorSound")
        .expect("identity-equality query should succeed");
    assert!(
        aliased,
        "PlayVocalErrorSoundID must be IDENTITY-EQUAL to C_Sound.PlayVocalErrorSound — \
         both sides see the SAME explicit no-op closure"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_sound_script_play_vocal_error_sound_id_calls_without_error(env: &WowLuaEnv) {

    let no_error: bool = env
        .eval("return pcall(PlayVocalErrorSoundID, 0)")
        .expect("pcall probe should succeed");
    assert!(
        no_error,
        "Calling PlayVocalErrorSoundID(0) must NOT throw — it resolves through the \
         explicit temporary sound no-op. pcall returns ok=true"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_sound_script_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_SoundScript.lua:4 doesn't bail \
         before PlayVocalErrorSoundID is defined. If this CVar flips to false, the global \
         is skipped and any legacy addon calling it blows up with `attempt to call a nil \
         value`"
    );
}
}

#[test]
fn blizzard_deprecated_sound_script_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedSoundScript");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedSoundScript dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedSoundScript has NO XML files — pure Lua single-shim \
         definition only. Got entries: {entries:?}"
    );

    let has_runtime_shims = entries.iter().any(|n| n == "Deprecated_SoundScript.lua");
    assert!(
        has_runtime_shims,
        "Blizzard_DeprecatedSoundScript should ship `Deprecated_SoundScript.lua` (the \
         runtime shim definition for the single deprecated PlayVocalErrorSoundID global)"
    );
}
