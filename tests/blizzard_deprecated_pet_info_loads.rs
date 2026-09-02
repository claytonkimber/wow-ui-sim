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

fn deprecated_pet_info_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeprecatedPetInfo/Blizzard_DeprecatedPetInfo.toc")
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
fn blizzard_deprecated_pet_info_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_pet_info_toc())
        .expect("Blizzard_DeprecatedPetInfo TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedPetInfo declares `## LoadOnDemand: 0` — the PetAssistMode / \
         GetPetTalentTree globals must install before any pet-frame addon Lua executes \
         that calls them"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedPetInfo does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedPetInfo declares NO dependencies — both shims simply forward \
         to C_PetInfo, which is bootstrapped at env init by \
         src/lua_api/globals/missing_surface/tooltip_info/mod.rs:23 (registers 3 explicit \
         methods: GetPetTamersForMap, GetSpellForPetAction, IsPetActionPassive). Neither \
         PetAssistMode nor GetPetTalentTree is explicitly stubbed — both resolve via the \
         namespace `__index` metamethod to cached no-op closures"
    );

    let toc_text = std::fs::read_to_string(deprecated_pet_info_toc())
        .expect("Blizzard_DeprecatedPetInfo TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedPetInfo omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311), matching the legacy pet API in-game-only surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedPetInfo omits `## AllowLoadGameType:` so the shims install on \
         every game type without restriction"
    );
}

#[test]
fn blizzard_deprecated_pet_info_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedPetInfo");
    assert!(
        in_game,
        "Blizzard_DeprecatedPetInfo (no AllowLoad flag, defaults to Game-only) should \
         appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedPetInfo");
    assert!(
        !in_login,
        "Blizzard_DeprecatedPetInfo should NOT appear on the Login / glue screens — pet \
         management is an in-game concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_pet_info_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedPetInfo") || message.contains("Deprecated_PetInfo")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedPetInfo emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_pet_info_installs_two_function_shims(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return type(PetAssistMode) == 'function' \
                and type(GetPetTalentTree) == 'function'",
        )
        .expect("2-shim function-installation query should succeed");
    assert!(
        installed,
        "Deprecated_PetInfo.lua line 9-10 should publish 2 forwarding global functions: \
         PetAssistMode → C_PetInfo.PetAssistMode; GetPetTalentTree → \
         C_PetInfo.GetPetTalentTree. Neither backing method is explicitly stubbed; both \
         resolve via `__wow_namespace_mt.__index` (runtime_surface_bootstrap.lua:2019-2028) \
         which lazily materializes a no-op `function() return nil end` and caches via \
         `rawset(t, key, fn)` so reads are stable. Result: each global is a callable \
         no-op, not nil"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_pet_info_globals_alias_c_pet_info_methods_by_identity(env: &WowLuaEnv) {

    let aliases_match: bool = env
        .eval(
            "return PetAssistMode == C_PetInfo.PetAssistMode \
                and GetPetTalentTree == C_PetInfo.GetPetTalentTree",
        )
        .expect("identity-equality query should succeed");
    assert!(
        aliases_match,
        "Both deprecated globals must reference identity-equal values to their backing \
         C_PetInfo methods. Since both methods are unstubbed, the namespace `__index` \
         caches the no-op closure via `rawset(t, key, fn)` so subsequent reads return the \
         SAME function — making the global alias and the namespace lookup point at the \
         exact same function value"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_pet_info_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_PetInfo.lua:4 doesn't bail before \
         PetAssistMode and GetPetTalentTree are defined. If this CVar flips to false, both \
         globals are skipped and any legacy pet-frame addon calling them blows up with \
         `attempt to call a nil value`"
    );
}
}

#[test]
fn blizzard_deprecated_pet_info_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedPetInfo");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedPetInfo dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedPetInfo has NO XML files — pure Lua function-shim definitions \
         only. Got entries: {entries:?}"
    );

    let has_runtime_shims = entries.iter().any(|n| n == "Deprecated_PetInfo.lua");
    assert!(
        has_runtime_shims,
        "Blizzard_DeprecatedPetInfo should ship `Deprecated_PetInfo.lua` (the runtime shim \
         definitions for the 2 deprecated pet-info globals)"
    );
}
