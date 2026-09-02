#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::discover_blizzard_addons_for_screen;
use wow_ui_sim::loader::load_addon;
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn encounter_timeline_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_EncounterTimeline/Blizzard_EncounterTimeline.toc")
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
fn blizzard_encounter_timeline_toc_declares_standard_game_type_with_edit_mode_dep() {
    let toc = TocFile::from_file(&encounter_timeline_toc())
        .expect("Blizzard_EncounterTimeline TOC should parse");

    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_EncounterTimeline omits `## LoadOnDemand:` — must auto-load on Game-screen \
         bring-up so the EditMode encounter-events system is registered before any encounter \
         timeline event fires"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_EncounterTimeline does not declare `## UseSecureEnvironment` — runs in the \
         standard taint environment"
    );

    let deps = toc.dependencies();
    assert_eq!(
        deps,
        &["Blizzard_EditMode".to_string()],
        "Blizzard_EncounterTimeline declares exactly one dependency: Blizzard_EditMode \
         (timeline frame inherits EditModeEncounterEventsSystemTemplate, defined by EditMode)"
    );

    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_EncounterTimeline declares `## AllowLoadGameType: standard` — `standard` is \
         the mainline-retail token (src/toc.rs:298-299), so the addon is NOT considered \
         game-type-restricted on retail"
    );

    let toc_text = std::fs::read_to_string(encounter_timeline_toc())
        .expect("Blizzard_EncounterTimeline TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_EncounterTimeline omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311), matching the in-combat-only encounter-timeline surface"
    );
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Blizzard_EncounterTimeline must declare `## AllowLoadGameType: standard` so that \
         classic/wrath/cata clients skip loading this mainline-only encounter feature"
    );
}

#[test]
fn blizzard_encounter_timeline_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_EncounterTimeline");
    assert!(
        in_game,
        "Blizzard_EncounterTimeline (no LoadOnDemand, no AllowLoad → Game-only default) should \
         auto-discover on the Game screen"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_EncounterTimeline");
    assert!(
        !in_login,
        "Blizzard_EncounterTimeline must NOT appear on the Login / glue screens — encounter \
         timelines are an in-combat feature with no glue-screen surface"
    );
}

#[test]
fn blizzard_encounter_timeline_loads_after_edit_mode_dependency() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let edit_mode_index = game_addons
        .iter()
        .position(|(name, _)| name == "Blizzard_EditMode")
        .expect("Blizzard_EditMode must be present in Game-screen discovery");
    let timeline_index = game_addons
        .iter()
        .position(|(name, _)| name == "Blizzard_EncounterTimeline")
        .expect("Blizzard_EncounterTimeline must be present in Game-screen discovery");

    assert!(
        edit_mode_index < timeline_index,
        "topological_sort_addons must place Blizzard_EditMode before Blizzard_EncounterTimeline \
         (timeline frame inherits EditModeEncounterEventsSystemTemplate). Got EditMode at \
         {edit_mode_index}, EncounterTimeline at {timeline_index}"
    );
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("EncounterTimeline"))
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_EncounterTimeline emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_is_addon_loaded_returns_true(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_EncounterTimeline') and true or false")
        .expect("C_AddOns.IsAddOnLoaded probe should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_EncounterTimeline') must return true after the addon \
         auto-loads on the Game screen"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_creates_named_singleton_frame(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(EncounterTimeline)")
        .expect("type(EncounterTimeline) probe should succeed");
    assert_eq!(
        kind, "table",
        "EncounterTimeline (declared in EncounterTimeline.xml:3 as `<Frame name=\"EncounterTimeline\" ... parent=\"UIParent\">`) \
         must be the named singleton frame published as a global after load"
    );

    let name: String = env
        .eval("return EncounterTimeline:GetName()")
        .expect("EncounterTimeline:GetName() probe should succeed");
    assert_eq!(
        name, "EncounterTimeline",
        "EncounterTimeline:GetName() must echo the XML `name` attribute"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_starts_hidden(env: &WowLuaEnv) {

    let hidden: bool = env
        .eval("return EncounterTimeline:IsShown() == false")
        .expect("EncounterTimeline:IsShown() probe should succeed");
    assert!(
        hidden,
        "EncounterTimeline declares `hidden=\"true\"` in EncounterTimeline.xml:3 — must be \
         hidden by default. EditMode's system framework brings it visible only while the \
         player is editing the HUD layout or a live encounter triggers it"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_publishes_core_mixin(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(EncounterTimelineMixin)")
        .expect("type(EncounterTimelineMixin) probe should succeed");
    assert_eq!(
        kind, "table",
        "EncounterTimelineMixin (EncounterTimeline.lua:12) must publish as a global table — it \
         is the mixin attached to the EncounterTimeline singleton via the XML `mixin=` attribute"
    );

    let has_on_load: bool = env
        .eval("return type(EncounterTimelineMixin.OnLoad) == 'function'")
        .expect("EncounterTimelineMixin.OnLoad probe should succeed");
    assert!(
        has_on_load,
        "EncounterTimelineMixin:OnLoad (EncounterTimeline.lua:14) registers \
         ENCOUNTER_STATE_CHANGED, ENCOUNTER_TIMELINE_STATE_UPDATED, SETTINGS_LOADED events plus \
         five EventRegistry callbacks — must be a function on the mixin"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_publishes_settings_mixin_hierarchy(env: &WowLuaEnv) {

    let kinds: (String, String, String, String, String, String) = env
        .eval(
            "return type(EncounterTimelineSettingsMixin), \
                    type(EncounterTimelineViewSettingsMixin), \
                    type(EncounterTimelineTrackSettingsMixin), \
                    type(EncounterTimelineTrackViewSettingsMixin), \
                    type(EncounterTimelineTimerSettingsMixin), \
                    type(EncounterTimelineTimerViewSettingsMixin)",
        )
        .expect("settings mixin hierarchy probe should succeed");
    let table = "table".to_string();
    assert_eq!(
        kinds,
        (
            table.clone(),
            table.clone(),
            table.clone(),
            table.clone(),
            table.clone(),
            table.clone(),
        ),
        "Blizzard_EncounterTimeline must publish six settings mixins from \
         EncounterTimelineSettings.lua: Settings, ViewSettings (extends Settings), TrackSettings, \
         TrackViewSettings (extends TrackSettings), TimerSettings, TimerViewSettings (extends \
         TimerSettings) — Edit Mode wires them up to control sizing, transparency, and layout"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_publishes_view_and_frame_manager_mixins(env: &WowLuaEnv) {

    let kinds: (String, String, String) = env
        .eval(
            "return type(EncounterTimelineFrameManagerMixin), \
                    type(EncounterTimelineDataProviderMixin), \
                    type(EncounterTimelineViewMixin)",
        )
        .expect("view/frame-manager mixin probe should succeed");
    assert_eq!(
        kinds,
        (
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
        ),
        "EncounterTimelineViewMixin (EncounterTimelineView.lua) is built from \
         CreateFromMixins(FrameManagerMixin, DataProviderMixin, ViewSettingsMixin) — all three \
         underlying mixins must publish as global tables"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_publishes_track_and_timer_mixins(env: &WowLuaEnv) {

    let kinds: (String, String, String, String, String, String) = env
        .eval(
            "return type(EncounterTimelineTrackFrameMixin), \
                    type(EncounterTimelineTrackEventMixin), \
                    type(EncounterTimelineTrackViewMixin), \
                    type(EncounterTimelineTimerEventMixin), \
                    type(EncounterTimelineTimerViewMixin), \
                    type(EncounterTimelineTimerViewTrackDividerMixin)",
        )
        .expect("track/timer mixin probe should succeed");
    let table = "table".to_string();
    assert_eq!(
        kinds,
        (
            table.clone(),
            table.clone(),
            table.clone(),
            table.clone(),
            table.clone(),
            table.clone(),
        ),
        "Track and timer event/view mixins must all publish: TrackFrameMixin (combines \
         EventFrame + TrackSettings), TrackEventMixin (extends TrackFrame), TrackViewMixin \
         (extends View + TrackViewSettings + TrackLayout), TimerEventMixin (combines \
         EventFrame + ScriptedAnimatable + TimerSettings), TimerViewMixin (extends View + \
         TimerViewSettings), TimerViewTrackDividerMixin"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_publishes_orientation_and_animation_mixins(env: &WowLuaEnv) {

    let kinds: (String, String, String, String, String, String) = env
        .eval(
            "return type(EncounterTimelineTrackOrientationMixin), \
                    type(EncounterTimelineOrientedScriptRegionMixin), \
                    type(EncounterTimelineOrientedFrameMixin), \
                    type(EncounterTimelineOrientedTextureMixin), \
                    type(EncounterTimelineTrackInterpolatorMixin), \
                    type(EncounterTimelineScriptedAnimatableMixin)",
        )
        .expect("orientation/animation mixin probe should succeed");
    let table = "table".to_string();
    assert_eq!(
        kinds,
        (
            table.clone(),
            table.clone(),
            table.clone(),
            table.clone(),
            table.clone(),
            table.clone(),
        ),
        "Util mixins (EncounterTimelineUtil.lua) and the scripted-animatable mixin \
         (EncounterTimelineScriptedAnimationUtil.lua) must all publish as global tables — they \
         underpin orientation flipping (horizontal/vertical timelines) and the alpha-fade \
         animations applied to encounter event icons"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_publishes_constants_table(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(EncounterTimelineConstants)")
        .expect("type(EncounterTimelineConstants) probe should succeed");
    assert_eq!(
        kind, "table",
        "EncounterTimelineConstants (EncounterTimelineConstants.lua:1) must publish as a global \
         table holding the timeline's tunable numeric constants (size/scale ratio, animation \
         durations, divider offsets, trail-fade rate)"
    );

    let size_ratio: f64 = env
        .eval("return EncounterTimelineConstants.SizeToScaleMultiplier")
        .expect("EncounterTimelineConstants.SizeToScaleMultiplier probe should succeed");
    assert!(
        (size_ratio - 0.01).abs() < f64::EPSILON,
        "EncounterTimelineConstants.SizeToScaleMultiplier must equal 0.01 \
         (EncounterTimelineConstants.lua:3) — Edit Mode size sliders multiply by this to \
         convert percentage-style settings into native frame scale"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_publishes_dirty_flag_and_animation_enums(env: &WowLuaEnv) {

    let animation_kinds: (i64, i64) = env
        .eval(
            "return EncounterTimelineScriptedAnimation.FadeIn, \
                    EncounterTimelineScriptedAnimation.FadeOut",
        )
        .expect("EncounterTimelineScriptedAnimation enum probe should succeed");
    assert_eq!(
        animation_kinds,
        (1, 2),
        "EncounterTimelineScriptedAnimation (EncounterTimeline.lua:1-4) must publish FadeIn=1 \
         and FadeOut=2 — these select the timeline's two scripted-animation routines"
    );

    let visibility_bit: i64 = env
        .eval("return EncounterTimelineDirtyFlag.Visibility")
        .expect("EncounterTimelineDirtyFlag.Visibility probe should succeed");
    assert_eq!(
        visibility_bit, 1,
        "EncounterTimelineDirtyFlag.Visibility (EncounterTimeline.lua:7) must equal \
         bit.lshift(1, 0) == 1 — the only dirty-bit currently used by the timeline"
    );

    let all_mask_kind: String = env
        .eval("return type(EncounterTimelineDirtyFlag.All)")
        .expect("EncounterTimelineDirtyFlag.All probe should succeed");
    assert_eq!(
        all_mask_kind, "number",
        "EncounterTimelineDirtyFlag.All (EncounterTimeline.lua:10) must be a numeric mask \
         produced by Flags_CreateMaskFromTable — used to seed CreateFlags() in OnLoad"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_publishes_visibility_cvars_array(env: &WowLuaEnv) {

    let cvars: (String, String, i64) = env
        .eval(
            "return EncounterTimelineVisibilityCVars[1], \
                    EncounterTimelineVisibilityCVars[2], \
                    #EncounterTimelineVisibilityCVars",
        )
        .expect("EncounterTimelineVisibilityCVars probe should succeed");
    assert_eq!(
        cvars,
        (
            "combatWarningsEnabled".to_string(),
            "encounterTimelineEnabled".to_string(),
            2,
        ),
        "EncounterTimelineVisibilityCVars (EncounterTimelineConstants.lua:159-162) must list \
         exactly the two CVars the timeline observes for visibility — `combatWarningsEnabled` \
         (master combat-warning toggle) and `encounterTimelineEnabled` (timeline-specific \
         toggle). OnLoad subscribes to both via CVarCallbackRegistry"
    );

    let icon_cvars: (String, String) = env
        .eval(
            "return EncounterTimelineIndicatorIconCVars.Enabled, \
                    EncounterTimelineIndicatorIconCVars.HiddenIconMask",
        )
        .expect("EncounterTimelineIndicatorIconCVars probe should succeed");
    assert_eq!(
        icon_cvars,
        (
            "encounterTimelineIconographyEnabled".to_string(),
            "encounterTimelineIconographyHiddenMask".to_string(),
        ),
        "EncounterTimelineIndicatorIconCVars (EncounterTimelineConstants.lua:164-167) must map \
         Enabled and HiddenIconMask to their respective CVar names — these drive icon-set \
         filtering on the timeline"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_timeline_timer_layout_direction_enum_values(env: &WowLuaEnv) {

    let directions: (i64, i64) = env
        .eval(
            "return EncounterTimelineTimerLayoutDirection.TopToBottom, \
                    EncounterTimelineTimerLayoutDirection.BottomToTop",
        )
        .expect("EncounterTimelineTimerLayoutDirection probe should succeed");
    assert_eq!(
        directions,
        (1, 2),
        "EncounterTimelineTimerLayoutDirection (EncounterTimelineConstants.lua:42-47) must \
         publish TopToBottom=1, BottomToTop=2 — the timer view uses these to anchor newly \
         spawned timer frames either at the top of the list (growing down) or the bottom \
         (growing up)"
    );
}
}
