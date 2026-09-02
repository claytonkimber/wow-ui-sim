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

fn damage_meter_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DamageMeter/Blizzard_DamageMeter.toc")
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
fn blizzard_damage_meter_toc_declares_edit_mode_dependency() {
    let toc =
        TocFile::from_file(&damage_meter_toc()).expect("Blizzard_DamageMeter TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DamageMeter is non-LOD — must auto-load on Game-screen bring-up so the \
         DamageMeter toplevel frame is created and the `damageMeterEnabled` CVar callback is \
         registered before VARIABLES_LOADED fires"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DamageMeter does not declare UseSecureEnvironment"
    );
    let deps = toc.dependencies();
    assert!(
        deps.contains(&"Blizzard_EditMode".to_string()),
        "Blizzard_DamageMeter should declare `## Dependencies: Blizzard_EditMode` (the \
         DamageMeterTemplate inherits EditModeDamageMeterSystemTemplate from \
         Blizzard_EditMode/Shared/EditModeSystemTemplates.xml; DamageMeterMixin:OnLoad calls \
         EditModeDamageMeterSystemMixin.OnSystemLoad), got {deps:?}"
    );
    let toc_text = std::fs::read_to_string(damage_meter_toc())
        .expect("Blizzard_DamageMeter TOC should be readable");
    assert!(
        toc_text.contains("## SavedVariablesPerCharacter: DamageMeterPerCharacterSettings"),
        "Blizzard_DamageMeter.toc should declare SavedVariablesPerCharacter for \
         `DamageMeterPerCharacterSettings` (storing windowDataList — which damage meter \
         windows were open and what damageMeterType they tracked)"
    );
    assert!(
        toc_text.contains("## LoadSavedVariablesFirst: 1"),
        "Blizzard_DamageMeter.toc should declare LoadSavedVariablesFirst: 1 so the saved \
         windowDataList is available before DamageMeterMixin:OnLoad runs \
         InitializeWindowDataList"
    );
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Blizzard_DamageMeter.toc should declare AllowLoadGameType: standard (retail-only)"
    );
}

#[test]
fn blizzard_damage_meter_appears_in_game_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DamageMeter");
    assert!(
        in_game,
        "Blizzard_DamageMeter (non-LOD with `## Dependencies: Blizzard_EditMode`) should \
         appear in Game-screen auto-discovery so the DamageMeter frame is built before any \
         combat events fire"
    );
}

prefork_full_ui_case! {
fn blizzard_damage_meter_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_DamageMeter")
                || message.contains("DamageMeterMixin")
                || message.contains("DamageMeterEntry")
                || message.contains("DamageMeterSourceWindow")
                || message.contains("DamageMeterSessionWindow")
                || message.contains("DamageMeterSettingsDropdownButton")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DamageMeter emitted Lua errors during load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_damage_meter_toplevel_frame_is_created(env: &WowLuaEnv) {

    let frame_present: bool = env
        .eval(
            "return type(DamageMeter) == 'table' \
                and DamageMeter:GetParent() == UIParent \
                and type(DamageMeter.GetSessionWindow) == 'function' \
                and type(DamageMeter.GetPrimarySessionWindow) == 'function'",
        )
        .expect("DamageMeter toplevel-frame query should succeed");
    assert!(
        frame_present,
        "Blizzard_DamageMeter.xml line 11 should create the toplevel `DamageMeter` frame as a \
         child of UIParent (inheriting DamageMeterTemplate which inherits \
         EditModeDamageMeterSystemTemplate). The frame owns the up-to-3 session windows via \
         GetSessionWindow(index) and GetPrimarySessionWindow()"
    );
}
}

prefork_full_ui_case! {
fn blizzard_damage_meter_main_mixin_methods_are_defined(env: &WowLuaEnv) {

    let mixin_present: bool = env
        .eval(
            "return type(DamageMeterMixin) == 'table' \
                and type(DamageMeterMixin.OnLoad) == 'function' \
                and type(DamageMeterMixin.OnEvent) == 'function' \
                and type(DamageMeterMixin.OnVariablesLoaded) == 'function' \
                and type(DamageMeterMixin.OnEnabledCVarChanged) == 'function' \
                and type(DamageMeterMixin.GetDefaultWindowData) == 'function' \
                and type(DamageMeterMixin.CreateWindowData) == 'function' \
                and type(DamageMeterMixin.InitializeWindowDataList) == 'function' \
                and type(DamageMeterMixin.LoadSavedWindowDataList) == 'function' \
                and type(DamageMeterMixin.SetupSessionWindow) == 'function' \
                and type(DamageMeterMixin.GetMaxSessionWindowCount) == 'function' \
                and type(DamageMeterMixin.GetCurrentSessionWindowCount) == 'function' \
                and type(DamageMeterMixin.CanShowNewSecondarySessionWindow) == 'function' \
                and type(DamageMeterMixin.ShowNewSecondarySessionWindow) == 'function' \
                and type(DamageMeterMixin.UpdateShownState) == 'function' \
                and type(DamageMeterMixin.IsPlayerInCombat) == 'function' \
                and type(DamageMeterMixin.ShouldBeShown) == 'function' \
                and type(DamageMeterMixin.IsEditing) == 'function' \
                and type(DamageMeterMixin.SetIsEditing) == 'function' \
                and type(DamageMeterMixin.GetNumberDisplayType) == 'function' \
                and type(DamageMeterMixin.SetNumberDisplayType) == 'function' \
                and type(DamageMeterMixin.GetBackgroundAlpha) == 'function' \
                and type(DamageMeterMixin.SetBackgroundAlpha) == 'function' \
                and type(DamageMeterMixin.GetBackgroundTransparency) == 'function' \
                and type(DamageMeterMixin.SetBackgroundTransparency) == 'function'",
        )
        .expect("DamageMeterMixin method query should succeed");
    assert!(
        mixin_present,
        "DamageMeter.lua should publish DamageMeterMixin (line 68) with the 72-method state \
         machine: lifecycle (OnLoad chaining EditModeDamageMeterSystemMixin.OnSystemLoad + \
         registering VARIABLES_LOADED via EventRegistry + the damageMeterEnabled CVar callback \
         via CVarCallbackRegistry; OnEvent dispatching PLAYER_IN_COMBAT_CHANGED / \
         PLAYER_LEVEL_CHANGED to UpdateShownState; OnVariablesLoaded; OnEnabledCVarChanged); \
         window-data CRUD (GetDefaultWindowData / CreateWindowData / InitializeWindowDataList \
         seeding the primary session window + restoring secondary windows from \
         DamageMeterPerCharacterSettings; LoadSavedWindowDataList validating saved entries via \
         IsSavedWindowDataValid; SetupSessionWindow); cap of MAX_DAMAGE_METER_SESSION_WINDOWS=3 \
         (GetMaxSessionWindowCount / GetCurrentSessionWindowCount / \
         CanShowNewSecondarySessionWindow / ShowNewSecondarySessionWindow); shown-state gating \
         (UpdateShownState / IsPlayerInCombat / ShouldBeShown / IsEditing / SetIsEditing); and \
         the EditMode-driven setting accessors (GetNumberDisplayType / SetNumberDisplayType / \
         GetBackgroundAlpha / SetBackgroundAlpha / GetBackgroundTransparency / \
         SetBackgroundTransparency)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_damage_meter_session_window_mixin_is_defined(env: &WowLuaEnv) {

    let mixin_present: bool = env
        .eval(
            "return type(DamageMeterSessionWindowMixin) == 'table' \
                and type(DamageMeterSessionWindowMixin.OnLoad) == 'function' \
                and type(DamageMeterSessionWindowMixin.OnShow) == 'function' \
                and type(DamageMeterSessionWindowMixin.OnHide) == 'function' \
                and type(DamageMeterSessionWindowMixin.OnEvent) == 'function' \
                and type(DamageMeterSessionWindowMixin.SetDamageMeterType) == 'function' \
                and type(DamageMeterSessionWindowMixin.GetDamageMeterType) == 'function' \
                and type(DamageMeterSessionWindowMixin.SetSession) == 'function' \
                and type(DamageMeterSessionWindowMixin.GetSessionType) == 'function' \
                and type(DamageMeterSessionWindowMixin.GetSessionID) == 'function' \
                and type(DamageMeterSessionWindowMixin.Refresh) == 'function' \
                and type(DamageMeterSessionWindowMixin.GetSourceWindow) == 'function' \
                and type(DamageMeterSessionWindowMixin.GetScrollBox) == 'function' \
                and type(DamageMeterSessionWindowMixin.GetSettingsDropdown) == 'function'",
        )
        .expect("DamageMeterSessionWindowMixin method query should succeed");
    assert!(
        mixin_present,
        "DamageMeterSessionWindow.lua should publish DamageMeterSessionWindowMixin (line 91) \
         with 107 methods covering the per-window damage-meter pane — lifecycle (OnLoad/OnShow/\
         OnHide/OnEvent), the damageMeterType / sessionType / sessionID accessors \
         (SetDamageMeterType / GetDamageMeterType / SetSession at line 820 / GetSessionType / \
         GetSessionID), the Refresh pipeline at line 754, and the child-frame accessors \
         (GetSourceWindow / GetScrollBox / GetSettingsDropdown / GetHeader / GetMinimizeButton \
         / GetResizeButton)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_damage_meter_source_window_mixin_is_defined(env: &WowLuaEnv) {

    let mixin_present: bool = env
        .eval(
            "return type(DamageMeterSourceWindowMixin) == 'table' \
                and type(DamageMeterSourceWindowMixin.OnLoad) == 'function' \
                and type(DamageMeterSourceWindowMixin.OnShow) == 'function' \
                and type(DamageMeterSourceWindowMixin.OnHide) == 'function' \
                and type(DamageMeterSourceWindowMixin.OnEvent) == 'function' \
                and type(DamageMeterSourceWindowMixin.OnEnter) == 'function' \
                and type(DamageMeterSourceWindowMixin.InitializeScrollBox) == 'function' \
                and type(DamageMeterSourceWindowMixin.InitializeResizeButton) == 'function' \
                and type(DamageMeterSourceWindowMixin.GetCombatSessionSource) == 'function' \
                and type(DamageMeterSourceWindowMixin.BuildDataProvider) == 'function' \
                and type(DamageMeterSourceWindowMixin.Refresh) == 'function' \
                and type(DamageMeterSourceWindowMixin.SetSource) == 'function' \
                and type(DamageMeterSourceWindowMixin.ClearSource) == 'function' \
                and type(DamageMeterSourceWindowMixin.IsShowingSource) == 'function' \
                and type(DamageMeterSourceWindowMixin.AnchorToSessionWindow) == 'function'",
        )
        .expect("DamageMeterSourceWindowMixin method query should succeed");
    assert!(
        mixin_present,
        "DamageMeterSourceWindow.lua should publish DamageMeterSourceWindowMixin (line 1) — the \
         right-side spell-drilldown pane that opens when a player bar is clicked. 53 methods: \
         lifecycle, scrollbox + resize-button setup, source identity (SetSource / ClearSource / \
         GetCombatSessionSource / IsShowingSource), the BuildDataProvider that pulls per-spell \
         entries from the active combat session, and AnchorToSessionWindow that side-anchors \
         this window LEFT or RIGHT of its parent DamageMeterSessionWindow"
    );
}
}

prefork_full_ui_case! {
fn blizzard_damage_meter_entry_mixins_are_defined(env: &WowLuaEnv) {

    let mixins_present: bool = env
        .eval(
            "return type(DamageMeterEntryMixin) == 'table' \
                and type(DamageMeterSourceEntryMixin) == 'table' \
                and type(DamageMeterSpellEntryMixin) == 'table' \
                and type(DamageMeterEntryMixin.Init) == 'function' \
                and type(DamageMeterEntryMixin.UpdateIcon) == 'function' \
                and type(DamageMeterEntryMixin.UpdateName) == 'function' \
                and type(DamageMeterEntryMixin.UpdateValue) == 'function' \
                and type(DamageMeterEntryMixin.UpdateStatusBar) == 'function' \
                and type(DamageMeterSourceEntryMixin.Init) == 'function' \
                and type(DamageMeterSourceEntryMixin.IsCreature) == 'function' \
                and type(DamageMeterSourceEntryMixin.GetIconAtlasElement) == 'function' \
                and type(DamageMeterSpellEntryMixin.Init) == 'function' \
                and type(DamageMeterSpellEntryMixin.GetSpellID) == 'function'",
        )
        .expect("DamageMeter entry mixin query should succeed");
    assert!(
        mixins_present,
        "DamageMeterEntry.lua should publish three entry-bar mixins: DamageMeterEntryMixin \
         (line 1, the shared Button base — 57 methods exposing the per-bar Init(source) entry \
         + the UpdateIcon / UpdateName / UpdateValue / UpdateStatusBar refresh helpers + child \
         accessors GetIcon / GetStatusBar / GetName / GetValue + value formatting via the \
         local numberDisplayTypeFormatters table); DamageMeterSourceEntryMixin (line 459, 9 \
         methods including Init(combatSource) / IsCreature / GetIconAtlasElement, used by the \
         session window to render per-source rows that drill-down to a SourceWindow on click); \
         and DamageMeterSpellEntryMixin (line 575, 9 methods including Init(combatSpell) / \
         GetSpellID, used by the source window to render per-spell rows). The two specialized \
         entry mixins inherit DamageMeterEntryTemplate via XML \
         (`<Button inherits=\"DamageMeterEntryTemplate\">`)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_damage_meter_settings_dropdown_button_is_defined(env: &WowLuaEnv) {

    let dropdown_present: bool = env
        .eval(
            "return type(DamageMeterSettingsDropdownButton) == 'table' \
                and type(ButtonStateBehaviorMixin) == 'table' \
                and type(DamageMeterSettingsDropdownButton.IsDown) == 'function' \
                and type(DamageMeterSettingsDropdownButton.IsOver) == 'function' \
                and type(DamageMeterSettingsDropdownButton.OnButtonStateChanged) == 'function' \
                and type(DamageMeterSettingsDropdownButton.GetIconAtlas) == 'function'",
        )
        .expect("DamageMeterSettingsDropdownButton query should succeed");
    assert!(
        dropdown_present,
        "DamageMeterSettingsDropdownButton.lua line 1 should define \
         `DamageMeterSettingsDropdownButton = CreateFromMixins(ButtonStateBehaviorMixin)` and \
         expose the inherited ButtonStateBehaviorMixin surface (IsDown / IsOver / IsDownOver) \
         plus the local OnButtonStateChanged hook that calls GetIconAtlas() and applies the \
         atlas to self:GetIcon(). The XML template is a `<DropdownButton virtual=\"true\">` \
         reused by each session window's settings menu (atlas keys disabled / normal / pressed \
         / hover / hoverPressed / open hold the `common-dropdown-a-button-settings-*` art)"
    );
}
}

#[test]
fn blizzard_damage_meter_xml_templates_are_registered() {
    let env = load_full_game_ui();
    drop(env);

    for template_name in [
        "DamageMeterTemplate",
        "DamageMeterEntryTemplate",
        "DamageMeterSourceEntryTemplate",
        "DamageMeterSpellEntryTemplate",
        "DamageMeterSettingsDropdownButtonTemplate",
        "DamageMeterSourceWindowTemplate",
        "DamageMeterSessionWindowTemplate",
    ] {
        assert!(
            wow_ui_sim::xml::get_template(template_name).is_some(),
            "{template_name} (`<Frame|Button|DropdownButton virtual=\"true\">` from one of the \
             6 Blizzard_DamageMeter XML files) should be registered in the Frame template \
             registry — the toplevel DamageMeter frame inherits DamageMeterTemplate, the \
             session window pool acquires DamageMeterSessionWindowTemplate, and the entry \
             scrollbox views inherit the entry/source/spell templates"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_damage_meter_constants_and_enums_are_defined(env: &WowLuaEnv) {

    let constants_present: bool = env
        .eval(
            "return DAMAGE_METER_DEFAULT_BAR_HEIGHT == 25 \
                and DAMAGE_METER_DEFAULT_BAR_SPACING == 4 \
                and DAMAGE_METER_TEXT_SIZE_TO_SCALE_MULTIPLIER == 0.01 \
                and DAMAGE_METER_TRANSPARENCY_TO_ALPHA_MULTIPLIER == 0.01 \
                and type(Enum.DamageMeterSessionType) == 'table' \
                and Enum.DamageMeterSessionType.Overall == 0 \
                and Enum.DamageMeterSessionType.Current == 1 \
                and Enum.DamageMeterSessionType.Expired == 2 \
                and type(Enum.DamageMeterType) == 'table' \
                and Enum.DamageMeterType.DamageDone == 0 \
                and Enum.DamageMeterType.HealingDone == 2 \
                and Enum.DamageMeterType.Deaths == 9",
        )
        .expect("DamageMeter constants/enum query should succeed");
    assert!(
        constants_present,
        "DamageMeterConstants.lua should publish DAMAGE_METER_DEFAULT_BAR_HEIGHT (25), \
         DAMAGE_METER_DEFAULT_BAR_SPACING (4), DAMAGE_METER_TEXT_SIZE_TO_SCALE_MULTIPLIER \
         (0.01), DAMAGE_METER_TRANSPARENCY_TO_ALPHA_MULTIPLIER (0.01); plus \
         Enum.DamageMeterSessionType={{Overall=0, Current=1, Expired=2}} and \
         Enum.DamageMeterType={{DamageDone=0, ..., Deaths=9, EnemyDamageTaken=10}} should be \
         registered (combat_system.rs SeqEnumDef DAMAGE_METER_SESSION_TYPE / DAMAGE_METER_TYPE)"
    );
}
}
