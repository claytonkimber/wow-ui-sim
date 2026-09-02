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

fn delves_companion_configuration_toc() -> PathBuf {
    blizzard_ui_dir()
        .join("Blizzard_DelvesCompanionConfiguration/Blizzard_DelvesCompanionConfiguration.toc")
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
fn blizzard_delves_companion_configuration_toc_declares_three_deps_standard_only() {
    let toc = TocFile::from_file(&delves_companion_configuration_toc())
        .expect("Blizzard_DelvesCompanionConfiguration TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DelvesCompanionConfiguration is non-LOD — the companion configuration + \
         ability list panels auto-load on Game-screen bring-up so the GENERIC_TRAIT_FRAME_* \
         and DELVES_GREAT_VAULT_FRAME_* / TRAIT_TREE_CHANGED listeners are wired before the \
         Brann Bronzebeard Delves NPC interaction occurs"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DelvesCompanionConfiguration does not declare UseSecureEnvironment"
    );

    let deps = toc.dependencies();
    for required in [
        "Blizzard_SharedTalentUI",
        "Blizzard_PagedContent",
        "Blizzard_Colors",
    ] {
        assert!(
            deps.contains(&required.to_string()),
            "Blizzard_DelvesCompanionConfiguration should declare `## Dependencies: \
             Blizzard_SharedTalentUI, Blizzard_PagedContent, Blizzard_Colors` (the \
             configuration frame inherits TalentFrameBaseTemplate and uses the TalentDisplayMixin \
             from SharedTalentUI for the ability tree, the PagedContent ScrollBox templates for \
             the ability-list pagination, and the Blizzard_Colors palette for the role-icon \
             tinting). Missing: {required}, got {deps:?}"
        );
    }

    let toc_text = std::fs::read_to_string(delves_companion_configuration_toc())
        .expect("Blizzard_DelvesCompanionConfiguration TOC should read");
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Blizzard_DelvesCompanionConfiguration declares `## AllowLoadGameType: standard` (the \
         Brann companion + Delves system is mainline-retail-only — Classic flavors don't ship \
         this UI)"
    );
}

#[test]
fn blizzard_delves_companion_configuration_appears_in_game_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DelvesCompanionConfiguration");
    assert!(
        in_game,
        "Blizzard_DelvesCompanionConfiguration (non-LOD with `## AllowLoadGameType: standard` + \
         3 non-LOD deps) should appear in Game-screen auto-discovery so DelvesCompanionConfigurationFrame \
         is registered with UIParent before the player opens the Brann Bronzebeard companion \
         configuration via the Delves UI"
    );
}

prefork_full_ui_case! {
fn blizzard_delves_companion_configuration_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DelvesCompanionConfiguration")
                || message.contains("DelvesCompanionAbilityList")
                || message.contains("CompanionConfigSlot")
                || message.contains("CompanionPortrait")
                || message.contains("CompanionExperienceRing")
                || message.contains("CompanionLevel")
                || message.contains("CompanionInfo")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DelvesCompanionConfiguration emitted Lua errors during load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_companion_configuration_toplevel_frames_are_created(env: &WowLuaEnv) {

    let frames_present: bool = env
        .eval(
            "return type(DelvesCompanionConfigurationFrame) == 'table' \
                and DelvesCompanionConfigurationFrame:GetParent() == UIParent \
                and DelvesCompanionConfigurationFrame:IsShown() == false \
                and type(DelvesCompanionAbilityListFrame) == 'table' \
                and DelvesCompanionAbilityListFrame:GetParent() == UIParent \
                and DelvesCompanionAbilityListFrame:IsShown() == false",
        )
        .expect("Delves Companion toplevel frame query should succeed");
    assert!(
        frames_present,
        "Blizzard_DelvesCompanionConfiguration.xml line 199 should create the toplevel \
         `DelvesCompanionConfigurationFrame` (parent=UIParent, toplevel=true, hidden=true, \
         inherits=InsetFrameTemplate, mixin=DelvesCompanionConfigurationFrameMixin) — the main \
         Brann customization panel — and Blizzard_DelvesCompanionAbilityList.xml line 47 should \
         create `DelvesCompanionAbilityListFrame` (parent=UIParent, toplevel=true, hidden=true, \
         inherits=PortraitFrameTemplate + TalentFrameBaseTemplate, mixin=\
         DelvesCompanionAbilityListFrameMixin) — the ability-tree panel that shows when the \
         player clicks `View Abilities`"
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_companion_configuration_main_mixin_is_published(env: &WowLuaEnv) {

    let mixin_present: bool = env
        .eval(
            "return type(DelvesCompanionConfigurationFrameMixin) == 'table' \
                and type(DelvesCompanionConfigurationFrameMixin.OnLoad) == 'function' \
                and type(DelvesCompanionConfigurationFrameMixin.OnShow) == 'function' \
                and type(DelvesCompanionConfigurationFrameMixin.OnHide) == 'function' \
                and type(DelvesCompanionConfigurationFrameMixin.OnEvent) == 'function' \
                and type(DelvesCompanionConfigurationFrameMixin.Refresh) == 'function' \
                and type(DelvesCompanionConfigurationFrameMixin.TryShowSeasonHelptip) == 'function'",
        )
        .expect("DelvesCompanionConfigurationFrameMixin query should succeed");
    assert!(
        mixin_present,
        "Blizzard_DelvesCompanionConfiguration.lua line 81 should publish \
         DelvesCompanionConfigurationFrameMixin with 7 lifecycle/refresh methods: OnLoad / \
         OnShow (registers GENERIC_TRAIT_FRAME_* / DELVES_GREAT_VAULT_FRAME_OPEN / \
         TRAIT_TREE_CHANGED) / OnHide / OnEvent / Refresh (drives the per-slot widget \
         reconfiguration) / TryShowSeasonHelptip (gates the LE_FRAME_TUTORIAL_DELVES_COMPANION \
         HelpTip)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_companion_configuration_companion_widget_mixins_are_published(env: &WowLuaEnv) {

    let mixins_present: bool = env
        .eval(
            "return type(CompanionPortraitFrameMixin) == 'table' \
                and type(CompanionPortraitFrameMixin.Refresh) == 'function' \
                and type(CompanionPortraitFrameMixin.OnEnter) == 'function' \
                and type(CompanionPortraitFrameMixin.OnLeave) == 'function' \
                and type(CompanionExperienceRingFrameMixin) == 'table' \
                and type(CompanionExperienceRingFrameMixin.Refresh) == 'function' \
                and type(CompanionLevelFrameMixin) == 'table' \
                and type(CompanionLevelFrameMixin.Refresh) == 'function' \
                and type(CompanionInfoFrameMixin) == 'table' \
                and type(CompanionInfoFrameMixin.Refresh) == 'function'",
        )
        .expect("Companion widget mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_DelvesCompanionConfiguration.lua should publish 4 widget mixins for the \
         Brann portrait composition: CompanionPortraitFrameMixin (line 188 — Refresh / OnEnter \
         / OnLeave with GameTooltip), CompanionExperienceRingFrameMixin (line 205 — Refresh \
         pulling C_DelvesUI experience), CompanionLevelFrameMixin (line 216 — Refresh stamping \
         the level number text), CompanionInfoFrameMixin (line 225 — Refresh aggregating the \
         portrait + ring + level + role icon)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_companion_configuration_slot_mixin_is_published(env: &WowLuaEnv) {

    let mixin_present: bool = env
        .eval(
            "return type(CompanionConfigSlotTemplateMixin) == 'table' \
                and type(CompanionConfigSlotTemplateMixin.OnLoad) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.OnEvent) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.OnShow) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.OnHide) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.OnEnter) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.OnLeave) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.OnMouseDown) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.SetSeenCurios) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.HasActiveEntry) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.HasSelectionAndInfo) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.CheckToggleAllowed) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.Refresh) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.PopulateOptionsList) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.GetSlotLabelText) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.GetSelectionNodeID) == 'function' \
                and type(CompanionConfigSlotTemplateMixin.BuildSelectionNodeOptions) == 'function' \
                and type(CompanionConfigSlotOptionsListMixin) == 'table' \
                and type(CompanionConfigSlotOptionsListMixin.OnShow) == 'function' \
                and type(CompanionConfigSlotOptionsListMixin.OnHide) == 'function' \
                and type(CompanionConfigListButtonMixin) == 'table' \
                and type(CompanionConfigListButtonMixin.OnClick) == 'function' \
                and type(CompanionConfigListButtonMixin.OnEnter) == 'function' \
                and type(CompanionConfigShowAbilitiesButtonMixin) == 'table'",
        )
        .expect("CompanionConfigSlot mixin query should succeed");
    assert!(
        mixin_present,
        "Blizzard_DelvesCompanionConfiguration.lua line 248 should publish \
         CompanionConfigSlotTemplateMixin with 16 methods (OnLoad / OnEvent / OnShow / OnHide \
         lifecycle, OnEnter / OnLeave / OnMouseDown input, SetSeenCurios for the new-curio \
         indicator, HasActiveEntry / HasSelectionAndInfo / CheckToggleAllowed for state \
         queries, Refresh + PopulateOptionsList + GetSlotLabelText + GetSelectionNodeID + \
         BuildSelectionNodeOptions for the SharedTalentUI tree integration) — backed by the \
         CompanionConfigSlotOptionsListMixin (lines 634-651, OnShow/OnHide), \
         CompanionConfigListButtonMixin (lines 651-735, OnClick/OnEnter for the dropdown rows), \
         and CompanionConfigShowAbilitiesButtonMixin (line 736 — the `View Abilities` button \
         that opens DelvesCompanionAbilityListFrame)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_companion_configuration_ability_list_mixins_are_published(env: &WowLuaEnv) {

    let mixins_present: bool = env
        .eval(
            "return type(DelvesCompanionAbilityListFrameMixin) == 'table' \
                and type(DelvesCompanionAbilityMixin) == 'table' \
                and DelvesCompanionAbilityMixin.Init ~= nil \
                and DelvesCompanionAbilityMixin.OnEnter ~= nil \
                and type(DelvesCompanionRoleDropdownMixin) == 'table' \
                and type(DelvesCompanionAbilityListPagingControlsMixin) == 'table'",
        )
        .expect("DelvesCompanionAbilityList mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_DelvesCompanionAbilityList.lua should publish 4 mixins: \
         DelvesCompanionAbilityListFrameMixin (line 44 — the ability-tree page lifecycle), \
         DelvesCompanionAbilityMixin (line 275 — `CreateFromMixins(TalentDisplayMixin)` so it \
         inherits TalentDisplayMixin.Init / OnEnter / SetTooltipInternal from \
         Blizzard_SharedTalentUI/Blizzard_TalentDisplay.lua), \
         DelvesCompanionRoleDropdownMixin (line 336 — the role-filter dropdown), and \
         DelvesCompanionAbilityListPagingControlsMixin (line 417 — the per-page navigation \
         strip backed by Blizzard_PagedContent)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_companion_configuration_xml_templates_are_registered(env: &WowLuaEnv) {
    let _env = env;

    let expected_templates = [
        "CompanionConfigSlotTemplate",
        "CompanionConfigListTemplate",
        "CompanionConfigListButtonTemplate",
        "DelvesCompanionAbilityTemplate",
    ];
    for template_name in expected_templates {
        assert!(
            wow_ui_sim::xml::get_template(template_name).is_some(),
            "Blizzard_DelvesCompanionConfiguration should register `{template_name}` in the \
             Frame template registry — used by DelvesCompanionConfigurationFrame for the per-\
             slot widgets, the SharedTalentUI tree's option list, the dropdown rows, and the \
             per-ability TalentDisplayMixin wrapper"
        );
    }
}
}
