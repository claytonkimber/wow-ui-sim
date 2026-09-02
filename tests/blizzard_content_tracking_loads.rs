#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::discover_blizzard_addons_for_screen;
use wow_ui_sim::loader::load_addon;
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path()
        .expect("Blizzard UI cache should be available")
}

fn content_tracking_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_ContentTracking/Blizzard_ContentTracking.toc")
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
fn blizzard_content_tracking_toc_declares_required_deps_and_game_only_load() {
    let toc = TocFile::from_file(&content_tracking_toc())
        .expect("Blizzard_ContentTracking TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_ContentTracking is a non-LOD addon (it should auto-load on Game screen)"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_ContentTracking does not declare UseSecureEnvironment"
    );
    assert!(
        toc.allows_screen(ScreenKind::Game),
        "Blizzard_ContentTracking has `## AllowLoad: Game`, so it should load on the Game screen"
    );
    assert!(
        !toc.allows_screen(ScreenKind::Login),
        "Blizzard_ContentTracking is `## AllowLoad: Game` only, so it must not load on the Login \
         screen (Glue contexts don't have ObjectiveTracker)"
    );
    let deps = toc.dependencies();
    for required in ["Blizzard_FrameXMLBase", "Blizzard_ObjectiveTracker"] {
        assert!(
            deps.contains(&required.to_string()),
            "Blizzard_ContentTracking should declare `## Dependencies: {required}`, got {deps:?}"
        );
    }
}

#[test]
fn blizzard_content_tracking_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_ContentTracking");
    assert!(
        in_game,
        "Blizzard_ContentTracking (`## AllowLoad: Game`, non-LOD) should appear in Game-screen \
         auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_ContentTracking");
    assert!(
        !in_login,
        "Blizzard_ContentTracking should NOT appear in Login-screen discovery (`## AllowLoad: \
         Game`)"
    );
}

prefork_full_ui_case! {
fn blizzard_content_tracking_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_ContentTracking")
                || message.contains("ContentTrackingManager")
                || message.contains("ContentTrackingElement")
                || message.contains("ContentTrackingUtil")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_ContentTracking emitted Lua errors during load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_content_tracking_util_exposes_full_function_surface(env: &WowLuaEnv) {

    let util_present: bool = env
        .eval(
            "return type(ContentTrackingUtil) == 'table' \
                and type(ContentTrackingUtil.IsTrackingModifierDown) == 'function' \
                and type(ContentTrackingUtil.IsContentTrackingEnabled) == 'function' \
                and type(ContentTrackingUtil.RegisterTrackableElement) == 'function' \
                and type(ContentTrackingUtil.UnregisterTrackableElement) == 'function' \
                and type(ContentTrackingUtil.ProcessChatLink) == 'function' \
                and type(ContentTrackingUtil.GetTrackingMapInfoByEncounterID) == 'function' \
                and type(ContentTrackingUtil.IsContentTrackedInEncounter) == 'function' \
                and type(ContentTrackingUtil.OpenMapToTrackable) == 'function' \
                and type(ContentTrackingUtil.DisplayTrackingError) == 'function' \
                and type(ContentTrackingUtil.MakeCombinedID) == 'function' \
                and type(ContentTrackingUtil.SplitCombinedID) == 'function'",
        )
        .expect("ContentTrackingUtil query should succeed");
    assert!(
        util_present,
        "ContentTrackingUtil should expose its 11 functions (IsTrackingModifierDown, \
         IsContentTrackingEnabled, RegisterTrackableElement, UnregisterTrackableElement, \
         ProcessChatLink, GetTrackingMapInfoByEncounterID, IsContentTrackedInEncounter, \
         OpenMapToTrackable, DisplayTrackingError, MakeCombinedID, SplitCombinedID) after \
         ContentTrackingManager.lua loads"
    );
}
}

prefork_full_ui_case! {
fn blizzard_content_tracking_combined_id_pack_and_split_round_trip(env: &WowLuaEnv) {

    let round_trip_ok: bool = env
        .eval(
            "local trackableType = 7; \
             local trackableID = 12345; \
             local combined = ContentTrackingUtil.MakeCombinedID(trackableType, trackableID); \
             local outType, outID = ContentTrackingUtil.SplitCombinedID(combined); \
             return combined == 12345 * 1000 + 7 \
                and outType == trackableType \
                and outID == trackableID",
        )
        .expect("MakeCombinedID/SplitCombinedID round-trip should succeed");
    assert!(
        round_trip_ok,
        "ContentTrackingUtil.MakeCombinedID(type, id) should encode as `id * 1000 + type` \
         (CombinedIDOffset=1000), and SplitCombinedID should recover (type, id) — round-trip \
         must be lossless"
    );
}
}

prefork_full_ui_case! {
fn blizzard_content_tracking_element_mixin_methods_are_defined(env: &WowLuaEnv) {

    let mixin_present: bool = env
        .eval(
            "return type(ContentTrackingElementMixin) == 'table' \
                and type(ContentTrackingElementMixin.OnHide) == 'function' \
                and type(ContentTrackingElementMixin.SetTrackable) == 'function' \
                and type(ContentTrackingElementMixin.AddTrackable) == 'function' \
                and type(ContentTrackingElementMixin.ClearTrackables) == 'function' \
                and type(ContentTrackingElementMixin.CheckTrackableClick) == 'function' \
                and type(ContentTrackingElementMixin.UpdateTrackingCheckmark) == 'function' \
                and type(ContentTrackingElementMixin.HasTrackableSource) == 'function' \
                and type(ContentTrackingElementMixin.SetTrackingCheckmarkShown) == 'function'",
        )
        .expect("ContentTrackingElementMixin query should succeed");
    assert!(
        mixin_present,
        "ContentTrackingElementMixin should expose its 8 methods (OnHide, SetTrackable, \
         AddTrackable, ClearTrackables, CheckTrackableClick, UpdateTrackingCheckmark, \
         HasTrackableSource, SetTrackingCheckmarkShown) after ContentTrackingElement.lua loads"
    );
}
}

prefork_full_ui_case! {
fn blizzard_content_tracking_checkmark_mixin_methods_are_defined(env: &WowLuaEnv) {

    let mixin_present: bool = env
        .eval(
            "return type(ContentTrackingCheckmarkMixin) == 'table' \
                and type(ContentTrackingCheckmarkMixin.OnEnter) == 'function' \
                and type(ContentTrackingCheckmarkMixin.OnLeave) == 'function'",
        )
        .expect("ContentTrackingCheckmarkMixin query should succeed");
    assert!(
        mixin_present,
        "ContentTrackingCheckmarkMixin should expose its 2 methods (OnEnter, OnLeave) wired to \
         GameTooltip via the `checkmark-minimal` atlas template"
    );
}
}

prefork_full_ui_case! {
fn blizzard_content_tracking_xml_templates_are_registered(env: &WowLuaEnv) {
    let _env = env;

    assert!(
        wow_ui_sim::xml::get_template("ContentTrackingElementTemplate").is_some(),
        "ContentTrackingElementTemplate (`<Frame virtual=\"true\" mixin=\"\
         ContentTrackingElementMixin\">` from ContentTrackingElement.xml) should be registered \
         in the template registry after Blizzard_ContentTracking loads"
    );
    assert!(
        wow_ui_sim::xml::get_texture_template_size("ContentTrackingCheckmarkTemplate").is_some(),
        "ContentTrackingCheckmarkTemplate (`<Texture virtual=\"true\" atlas=\"checkmark-minimal\" \
         mixin=\"ContentTrackingCheckmarkMixin\">` 20x19) should be registered in the texture \
         template registry"
    );
}
}
