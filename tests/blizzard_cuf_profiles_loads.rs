#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::discover_blizzard_addons_for_screen;
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn cuf_profiles_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_CUFProfiles/Blizzard_CUFProfiles.toc")
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
        wow_ui_sim::loader::load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("[load {name}] FAILED: {err}"));
    }

    env.apply_post_load_workarounds();
    fire_startup_events_for_screen(&env, ScreenKind::Game);
    env
}

#[test]
fn blizzard_cuf_profiles_toc_declares_unit_frame_dep() {
    let toc =
        TocFile::from_file(&cuf_profiles_toc()).expect("Blizzard_CUFProfiles TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_CUFProfiles is non-LOD — it must auto-load on Game-screen bring-up so that \
         `CompactUnitFrameProfiles:Init({{...}})` runs and registers the CVar→option mapping \
         callbacks before VARIABLES_LOADED fires"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_CUFProfiles does not declare UseSecureEnvironment"
    );
    let deps = toc.dependencies();
    assert!(
        deps.contains(&"Blizzard_UnitFrame".to_string()),
        "Blizzard_CUFProfiles should declare `## Dependencies: Blizzard_UnitFrame` (the addon \
         mutates DefaultCompactUnitFrameOptions / DefaultCompactMiniFrameOptions / \
         DefaultCompactUnitFrameSetupOptions / DefaultCompactMiniFrameSetUpOptions and calls \
         DefaultCompactUnitFrameSetup / CompactUnitFrame_UpdateAll, all defined by \
         Blizzard_UnitFrame), got {deps:?}"
    );
}

#[test]
fn blizzard_cuf_profiles_loads_with_allow_load_environment_global_annotation() {
    let toc_text = std::fs::read_to_string(cuf_profiles_toc())
        .expect("Blizzard_CUFProfiles TOC should be readable");
    assert!(
        toc_text.contains("Blizzard_CompactUnitFrameProfiles.lua [AllowLoadEnvironment Global]"),
        "Blizzard_CUFProfiles.toc should annotate its single Lua file with \
         `[AllowLoadEnvironment Global]` (the file uses unprefixed globals like \
         `CompactUnitFrameProfiles:Init` + module-level locals; without the Global \
         environment opt-in the loader could try to sandbox it into a per-addon environment) \
         — got TOC contents:\n{toc_text}"
    );
}

#[test]
fn blizzard_cuf_profiles_appears_in_game_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CUFProfiles");
    assert!(
        in_game,
        "Blizzard_CUFProfiles (non-LOD with `## Dependencies: Blizzard_UnitFrame`) should \
         appear in Game-screen auto-discovery so the CompactUnitFrameProfiles:Init CVar \
         callbacks are wired during startup"
    );
}

prefork_full_ui_case! {
fn blizzard_cuf_profiles_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_CUFProfiles")
                || message.contains("CompactUnitFrameProfiles")
                || message.contains("Blizzard_CompactUnitFrameProfiles")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_CUFProfiles emitted Lua errors during load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_cuf_profiles_dependency_globals_are_populated(env: &WowLuaEnv) {

    let deps_present: bool = env
        .eval(
            "return type(DefaultCompactUnitFrameOptions) == 'table' \
                and type(DefaultCompactMiniFrameOptions) == 'table' \
                and type(DefaultCompactUnitFrameSetupOptions) == 'table' \
                and type(DefaultCompactMiniFrameSetUpOptions) == 'table' \
                and type(CVarCallbackRegistry) == 'table' \
                and type(CVarCallbackRegistry.RegisterCallback) == 'function' \
                and type(EventRegistry) == 'table' \
                and type(EventRegistry.RegisterFrameEventAndCallback) == 'function'",
        )
        .expect("CUF dependency globals query should succeed");
    assert!(
        deps_present,
        "Blizzard_CUFProfiles relies on the Blizzard_UnitFrame globals \
         DefaultCompactUnitFrameOptions / DefaultCompactMiniFrameOptions / \
         DefaultCompactUnitFrameSetupOptions / DefaultCompactMiniFrameSetUpOptions plus the \
         shared CVarCallbackRegistry:RegisterCallback (used to wire each \
         raidFrames*/pvpFrames*/raidOption* CVar to OnCVarChanged) and \
         EventRegistry:RegisterFrameEventAndCallback (used to wire VARIABLES_LOADED to \
         OnVariablesLoaded → ApplyCurrentSettings)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_cuf_profiles_module_local_does_not_leak_globally(env: &WowLuaEnv) {

    let local_only: bool = env
        .eval("return _G.CompactUnitFrameProfiles == nil")
        .expect("CompactUnitFrameProfiles global query should succeed");
    assert!(
        local_only,
        "Blizzard_CompactUnitFrameProfiles.lua line 39 declares `local CompactUnitFrameProfiles \
         = {{}}` so the dispatcher table is module-local — verifies the loader honors the \
         local declaration and does not promote it to a global. The CVar→option Init() call \
         on the next-to-last line (line 109-137) operates entirely through that local handle, \
         and the only externally-visible side effects are the CVar / EventRegistry callback \
         registrations"
    );
}
}
