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

fn encounter_warnings_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_EncounterWarnings/Blizzard_EncounterWarnings.toc")
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
fn blizzard_encounter_warnings_toc_declares_standard_game_type_with_edit_mode_dep() {
    let toc = TocFile::from_file(&encounter_warnings_toc())
        .expect("Blizzard_EncounterWarnings TOC should parse");

    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_EncounterWarnings omits `## LoadOnDemand:` — must auto-load on Game-screen \
         bring-up so the three EncounterWarnings system frames are registered with EditMode \
         before any ENCOUNTER_WARNING event fires"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_EncounterWarnings does not declare `## UseSecureEnvironment` — runs in the \
         standard taint environment"
    );

    let deps = toc.dependencies();
    assert_eq!(
        deps,
        &["Blizzard_EditMode".to_string()],
        "Blizzard_EncounterWarnings declares exactly one dependency: Blizzard_EditMode (the \
         three system frames inherit EditModeEncounterEventsSystemTemplate, defined by EditMode)"
    );

    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_EncounterWarnings declares `## AllowLoadGameType: standard` — `standard` is \
         the mainline-retail token (src/toc.rs:298-299), so the addon is NOT considered \
         game-type-restricted on retail"
    );

    let toc_text = std::fs::read_to_string(encounter_warnings_toc())
        .expect("Blizzard_EncounterWarnings TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_EncounterWarnings omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311), matching the in-combat-only encounter-warning surface"
    );
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Blizzard_EncounterWarnings must declare `## AllowLoadGameType: standard` so \
         classic/wrath/cata clients skip loading this mainline-only encounter feature"
    );
}

#[test]
fn blizzard_encounter_warnings_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_EncounterWarnings");
    assert!(
        in_game,
        "Blizzard_EncounterWarnings (no LoadOnDemand, no AllowLoad → Game-only default) should \
         auto-discover on the Game screen"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_EncounterWarnings");
    assert!(
        !in_login,
        "Blizzard_EncounterWarnings must NOT appear on Login / glue screens — encounter \
         warnings are an in-combat boss-emote feature with no glue-screen surface"
    );
}

#[test]
fn blizzard_encounter_warnings_loads_after_edit_mode_dependency() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let edit_mode_index = game_addons
        .iter()
        .position(|(name, _)| name == "Blizzard_EditMode")
        .expect("Blizzard_EditMode must be present in Game-screen discovery");
    let warnings_index = game_addons
        .iter()
        .position(|(name, _)| name == "Blizzard_EncounterWarnings")
        .expect("Blizzard_EncounterWarnings must be present in Game-screen discovery");

    assert!(
        edit_mode_index < warnings_index,
        "topological_sort_addons must place Blizzard_EditMode before Blizzard_EncounterWarnings \
         (each system frame inherits EditModeEncounterEventsSystemTemplate). Got EditMode at \
         {edit_mode_index}, EncounterWarnings at {warnings_index}"
    );
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("EncounterWarnings"))
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_EncounterWarnings emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_is_addon_loaded_returns_true(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_EncounterWarnings') and true or false")
        .expect("C_AddOns.IsAddOnLoaded probe should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_EncounterWarnings') must return true after the addon \
         auto-loads on the Game screen"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_creates_three_severity_singletons(env: &WowLuaEnv) {

    let kinds: (String, String, String) = env
        .eval(
            "return type(CriticalEncounterWarnings), \
                    type(MediumEncounterWarnings), \
                    type(MinorEncounterWarnings)",
        )
        .expect("severity singleton type probe should succeed");
    assert_eq!(
        kinds,
        (
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
        ),
        "EncounterWarnings.xml lines 24-46 declare three named non-virtual frames \
         (CriticalEncounterWarnings/MediumEncounterWarnings/MinorEncounterWarnings) all \
         parent=UIParent, inheriting EncounterWarningsSystemFrameTemplate — must publish as \
         three global tables after load"
    );

    let names: (String, String, String) = env
        .eval(
            "return CriticalEncounterWarnings:GetName(), \
                    MediumEncounterWarnings:GetName(), \
                    MinorEncounterWarnings:GetName()",
        )
        .expect("severity singleton name probe should succeed");
    assert_eq!(
        names,
        (
            "CriticalEncounterWarnings".to_string(),
            "MediumEncounterWarnings".to_string(),
            "MinorEncounterWarnings".to_string(),
        ),
        ":GetName() must echo each XML `name` attribute"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_severity_singletons_inherit_view_child_frame(env: &WowLuaEnv) {

    let view_kinds: (String, String, String) = env
        .eval(
            "return type(CriticalEncounterWarnings.View), \
                    type(MediumEncounterWarnings.View), \
                    type(MinorEncounterWarnings.View)",
        )
        .expect("View child probe should succeed");
    let table = "table".to_string();
    assert_eq!(
        view_kinds,
        (table.clone(), table.clone(), table.clone()),
        "EncounterWarnings.xml:10 nests `<Frame parentKey=\"View\" \
         inherits=\"EncounterWarningsViewTemplate\">` inside the system-frame template — every \
         severity singleton inheriting the template must expose `.View` as a child Frame on \
         the Lua side. EncounterWarningsViewMixin (CreateFromMixins(SettingsMixin, \
         ResizeLayoutMixin)) is attached to that child"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_publishes_system_frame_mixin(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(EncounterWarningsSystemFrameMixin)")
        .expect("type(EncounterWarningsSystemFrameMixin) probe should succeed");
    assert_eq!(
        kind, "table",
        "EncounterWarningsSystemFrameMixin (EncounterWarnings.lua:5, \
         CreateFromMixins(EditModeEncounterEventsSystemMixin)) must publish as a global table \
         — it is the mixin attached to the EncounterWarningsSystemFrameTemplate via the XML \
         `mixin=` attribute"
    );

    let has_lifecycle: (bool, bool, bool, bool) = env
        .eval(
            "return type(EncounterWarningsSystemFrameMixin.OnLoad) == 'function', \
                    type(EncounterWarningsSystemFrameMixin.OnShow) == 'function', \
                    type(EncounterWarningsSystemFrameMixin.OnHide) == 'function', \
                    type(EncounterWarningsSystemFrameMixin.OnEvent) == 'function'",
        )
        .expect("system-frame lifecycle method probe should succeed");
    assert_eq!(
        has_lifecycle,
        (true, true, true, true),
        "EncounterWarningsSystemFrameMixin must define OnLoad / OnShow / OnHide / OnEvent — \
         the four script handlers wired up by EncounterWarnings.xml:17-20. OnLoad subscribes \
         to CLEAR_BOSS_EMOTES + PLAYER_IN_COMBAT_CHANGED and the three visibility CVars; \
         OnShow/OnHide gate ENCOUNTER_WARNING registration via FrameUtil"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_publishes_view_and_view_element_mixins(env: &WowLuaEnv) {

    let kinds: (String, String, String, String, String) = env
        .eval(
            "return type(EncounterWarningsViewMixin), \
                    type(EncounterWarningsViewElementMixin), \
                    type(EncounterWarningsSwingAnimationGroupMixin), \
                    type(EncounterWarningsIconElementMixin), \
                    type(EncounterWarningsTextElementMixin)",
        )
        .expect("view + view-element mixin probe should succeed");
    let table = "table".to_string();
    assert_eq!(
        kinds,
        (
            table.clone(),
            table.clone(),
            table.clone(),
            table.clone(),
            table.clone(),
        ),
        "EncounterWarningsView.lua publishes ViewMixin (CreateFromMixins(SettingsMixin, \
         ResizeLayoutMixin)); EncounterWarningsViewElements.lua publishes the base \
         ViewElementMixin plus three derived mixins (SwingAnimationGroup, IconElement, \
         TextElement). All five must publish as global tables"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_publishes_settings_and_util_namespaces(env: &WowLuaEnv) {

    let kinds: (String, String) = env
        .eval(
            "return type(EncounterWarningsSettingsMixin), \
                    type(EncounterWarningsUtil)",
        )
        .expect("settings/util namespace probe should succeed");
    assert_eq!(
        kinds,
        ("table".to_string(), "table".to_string()),
        "EncounterWarningsSettingsMixin (EncounterWarningsSettings.lua) and \
         EncounterWarningsUtil (EncounterWarningsUtil.lua) must publish as global tables — \
         the settings mixin is consumed by EncounterWarningsViewMixin's CreateFromMixins, \
         and the util namespace holds shared helpers"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_publishes_constants_table(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(EncounterWarningsConstants)")
        .expect("type(EncounterWarningsConstants) probe should succeed");
    assert_eq!(
        kind, "table",
        "EncounterWarningsConstants (EncounterWarningsConstants.lua:1) must publish as a \
         global table holding the addon's tunable numeric constants"
    );

    let scale: f64 = env
        .eval("return EncounterWarningsConstants.SizeToScaleMultiplier")
        .expect("SizeToScaleMultiplier probe should succeed");
    assert!(
        (scale - 0.01).abs() < f64::EPSILON,
        "EncounterWarningsConstants.SizeToScaleMultiplier must equal 0.01 \
         (EncounterWarningsConstants.lua:3) — Edit Mode size sliders multiply by this to \
         convert percentage-style settings into native frame scale"
    );

    let chat_icon_size: i64 = env
        .eval("return EncounterWarningsConstants.ChatMessageIconDisplaySize")
        .expect("ChatMessageIconDisplaySize probe should succeed");
    assert_eq!(
        chat_icon_size, 20,
        "EncounterWarningsConstants.ChatMessageIconDisplaySize must equal 20 \
         (EncounterWarningsConstants.lua:7) — used to scale icon textures inlined into chat \
         messages mirroring an encounter warning"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_publishes_severity_lookup_tables(env: &WowLuaEnv) {

    let severity_kinds: (String, String, String) = env
        .eval(
            "return type(EncounterWarningsSystemSeverity), \
                    type(EncounterWarningsSeverityTextSizeLimits), \
                    type(EncounterWarningsSeverityFonts)",
        )
        .expect("severity lookup table probe should succeed");
    assert_eq!(
        severity_kinds,
        (
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
        ),
        "EncounterWarningsConstants.lua publishes three severity-keyed dispatch tables: \
         SystemSeverity (system-index → EncounterEventSeverity), SeverityTextSizeLimits \
         (severity → text bounding box), SeverityFonts (severity → font-getter closure). \
         All three must publish as tables"
    );

    let high_limits: (i64, i64) = env
        .eval(
            "local entry = EncounterWarningsSeverityTextSizeLimits[Enum.EncounterEventSeverity.High]; \
             return entry.width, entry.height",
        )
        .expect("High-severity size-limits probe should succeed");
    assert_eq!(
        high_limits,
        (490, 48),
        "EncounterWarningsSeverityTextSizeLimits[High] must equal {{ width=490, height=48 }} \
         (EncounterWarningsConstants.lua:25) — chosen to be ~110px less than the 600x48 XML \
         size of CriticalEncounterWarnings so the text doesn't escape the EditMode preview \
         bounds"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_publishes_visibility_cvars_array(env: &WowLuaEnv) {

    let cvars: (String, String, String, i64) = env
        .eval(
            "return EncounterWarningsVisibilityCVars[1], \
                    EncounterWarningsVisibilityCVars[2], \
                    EncounterWarningsVisibilityCVars[3], \
                    #EncounterWarningsVisibilityCVars",
        )
        .expect("EncounterWarningsVisibilityCVars probe should succeed");
    assert_eq!(
        cvars,
        (
            "combatWarningsEnabled".to_string(),
            "encounterWarningsEnabled".to_string(),
            "encounterWarningsLevel".to_string(),
            3,
        ),
        "EncounterWarningsVisibilityCVars (EncounterWarningsConstants.lua:39-43) must list \
         exactly three CVars: master combatWarningsEnabled toggle, encounterWarnings-specific \
         enabled toggle, encounterWarningsLevel (severity threshold). OnLoad iterates this \
         array and registers a CVarCallbackRegistry callback per entry that invokes \
         self:UpdateVisibility() when any of them flips"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_publishes_dynamic_events_array(env: &WowLuaEnv) {

    let events: (String, i64) = env
        .eval(
            "return EncounterWarningsSystemDynamicEvents[1], \
                    #EncounterWarningsSystemDynamicEvents",
        )
        .expect("EncounterWarningsSystemDynamicEvents probe should succeed");
    assert_eq!(
        events,
        ("ENCOUNTER_WARNING".to_string(), 1),
        "EncounterWarningsSystemDynamicEvents (EncounterWarnings.lua:1-3) must list exactly \
         one event — `ENCOUNTER_WARNING`. OnShow registers it via \
         FrameUtil.RegisterFrameForEvents and OnHide unregisters it, so the system frame only \
         observes warnings while it is visible"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_warnings_setting_defaults_table_is_populated(env: &WowLuaEnv) {

    let icon_scale: f64 = env
        .eval("return EncounterWarningsSettingDefaults.IconScale")
        .expect("EncounterWarningsSettingDefaults.IconScale probe should succeed");
    assert!(
        (icon_scale - 1.0).abs() < f64::EPSILON,
        "EncounterWarningsSettingDefaults.IconScale must equal 1.0 \
         (EncounterWarningsConstants.lua:11) — the default Edit Mode icon-scale slider value"
    );

    let tooltip_anchor_kind: String = env
        .eval("return type(EncounterWarningsSettingDefaults.TooltipAnchor)")
        .expect("TooltipAnchor type probe should succeed");
    assert_eq!(
        tooltip_anchor_kind, "number",
        "EncounterWarningsSettingDefaults.TooltipAnchor must resolve to a numeric \
         Enum.EncounterEventsTooltipAnchor.Default value — confirms the enum is seeded at \
         env init before this constants file evaluates"
    );
}
}
