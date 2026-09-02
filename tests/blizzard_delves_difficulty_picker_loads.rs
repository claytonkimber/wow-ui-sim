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

fn delves_difficulty_picker_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DelvesDifficultyPicker/Blizzard_DelvesDifficultyPicker.toc")
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
fn blizzard_delves_difficulty_picker_toc_is_load_on_demand_with_colors_dep() {
    let toc = TocFile::from_file(&delves_difficulty_picker_toc())
        .expect("Blizzard_DelvesDifficultyPicker TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_DelvesDifficultyPicker is `## LoadOnDemand: 1` — UIParentLoadAddOn or the \
         CustomGossipFrameBase opener triggers loading when a delve gossip pops, NOT at \
         Game-screen bring-up"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DelvesDifficultyPicker does not declare UseSecureEnvironment"
    );
    let deps = toc.dependencies();
    assert_eq!(
        deps,
        vec!["Blizzard_Colors".to_string()],
        "Blizzard_DelvesDifficultyPicker should declare exactly one dependency: Blizzard_Colors \
         (the difficulty-picker pulls semantic color constants for tier highlights / disabled \
         entrance error text), got {deps:?}"
    );
}

#[test]
fn blizzard_delves_difficulty_picker_is_absent_from_game_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DelvesDifficultyPicker");
    assert!(
        !in_game,
        "Blizzard_DelvesDifficultyPicker is LOD so it must NOT appear in Game-screen \
         auto-discovery — it loads on demand when the player triggers a delve-entrance gossip"
    );
}

prefork_full_ui_case! {
fn blizzard_delves_difficulty_picker_loads_via_explicit_load_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &delves_difficulty_picker_toc())
        .expect("Blizzard_DelvesDifficultyPicker should load via explicit Rust loader call");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_DelvesDifficultyPicker emitted Lua errors during LOD load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_difficulty_picker_toplevel_frame_is_created(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &delves_difficulty_picker_toc())
        .expect("LOD load should succeed");

    let frame_present: bool = env
        .eval(
            "return DelvesDifficultyPickerFrame ~= nil \
                and DelvesDifficultyPickerFrame:IsObjectType('Frame') \
                and not DelvesDifficultyPickerFrame:IsShown()",
        )
        .expect("DelvesDifficultyPickerFrame query should succeed");
    assert!(
        frame_present,
        "DelvesDifficultyPickerFrame (Blizzard_DelvesDifficultyPicker.xml:63, parent=UIParent, \
         hidden, inherits CustomGossipFrameBaseTemplate + InsetFrameTemplate, mixin \
         DelvesDifficultyPickerFrameMixin) should be created and hidden by default"
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_difficulty_picker_main_mixin_is_published(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &delves_difficulty_picker_toc())
        .expect("LOD load should succeed");

    let mixin_present: bool = env
        .eval(
            "return type(DelvesDifficultyPickerFrameMixin) == 'table' \
                and type(DelvesDifficultyPickerFrameMixin.OnLoad) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.OnEvent) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.OnShow) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.OnHide) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.SetupDropdown) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.UpdateWidgets) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.SetInitialTier) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.GetSelectedTierInfo) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.SetSelectedTierInfo) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.GetTierInfos) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.CanEnterDelve) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.UpdatePortalButtonState) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.CheckAndSetDisplayMode) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.CheckForNewTierUnlocks) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.TryShowHelpTip) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.HideHelpTip) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.TryShow) == 'function' \
                and type(DelvesDifficultyPickerFrameMixin.SetStartingPage) == 'function'",
        )
        .expect("DelvesDifficultyPickerFrameMixin query should succeed");
    assert!(
        mixin_present,
        "Blizzard_DelvesDifficultyPicker.lua line 51 should publish \
         DelvesDifficultyPickerFrameMixin with the full lifecycle (OnLoad / OnEvent / OnShow / \
         OnHide), dropdown setup (SetupDropdown), tier state (SetInitialTier / \
         GetSelectedTierInfo / SetSelectedTierInfo / GetTierInfos / CanEnterDelve / \
         UpdatePortalButtonState), display-mode checks (CheckAndSetDisplayMode), help-tip \
         lifecycle (CheckForNewTierUnlocks / TryShowHelpTip / HideHelpTip), the public TryShow \
         entry point used by gossip script handlers, and the required-but-empty SetStartingPage \
         hook for CustomGossipFrameBaseTemplate"
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_difficulty_picker_secondary_mixins_are_published(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &delves_difficulty_picker_toc())
        .expect("LOD load should succeed");

    let mixins_present: bool = env
        .eval(
            "return type(DelveChallengesContainerFrameMixin) == 'table' \
                and type(DelvesDifficultyPickerEnterDelveButtonMixin) == 'table' \
                and type(DelveRewardsContainerFrameMixin) == 'table' \
                and type(DelveRewardsButtonMixin) == 'table' \
                and type(DelvesDifficultyPickerDropdownMixin) == 'table' \
                and type(DelveChallengesContainerFrameMixin.OnLoad) == 'function' \
                and type(DelveChallengesContainerFrameMixin.CheckPartyLeader) == 'function' \
                and type(DelvesDifficultyPickerEnterDelveButtonMixin.OnClick) == 'function' \
                and type(DelveRewardsContainerFrameMixin.SetRewards) == 'function' \
                and type(DelveRewardsButtonMixin.OnEnter) == 'function' \
                and type(DelvesDifficultyPickerDropdownMixin.OnEnter) == 'function'",
        )
        .expect("Secondary mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_DelvesDifficultyPicker.lua should publish 5 secondary mixins: \
         DelveChallengesContainerFrameMixin (line 207 — wraps TalentFrameGridTemplate to host \
         the affix challenges grid; uses CheckPartyLeader for leader-only commit gating), \
         DelvesDifficultyPickerEnterDelveButtonMixin (line 606 — UIPanelButton wrapper on the \
         Enter Delve button with OnClick that calls C_PartyInfo / C_GossipInfo), \
         DelveRewardsContainerFrameMixin (line 668 — owns the right-rail scroll list of \
         drop-chance reward items, populated via SetRewards), DelveRewardsButtonMixin (line \
         776 — per-row LargeItemButton wrapper with hover tooltip), and \
         DelvesDifficultyPickerDropdownMixin (line 825 — WowStyle1Dropdown that surfaces tier \
         eligibility tooltips on hover via OnEnter)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_difficulty_picker_keeps_player_key_state_local(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &delves_difficulty_picker_toc())
        .expect("LOD load should succeed");

    let helper_is_local: bool = env
        .eval("return GetPlayerKeyState == nil")
        .expect("GetPlayerKeyState query should succeed");
    assert!(
        helper_is_local,
        "Retail 12.1 declares GetPlayerKeyState as a local helper in \
         Blizzard_DelvesDifficultyPicker.lua; loading the addon must not publish it globally"
    );
}
}

prefork_full_ui_case! {
fn blizzard_delves_difficulty_picker_xml_templates_are_registered(env: &WowLuaEnv) {
    let _env = env;
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_mode(ScreenKind::Game);
    {
        let mut state = env.state().borrow_mut();
        state.addon_base_paths = vec![blizzard_ui_dir()];
    }
    wow_ui_sim::xml::register_intrinsic_templates();
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    for (_, toc_path) in &addons {
        let _ = load_addon(&env.loader_env(), toc_path);
    }
    let _ = load_addon(&env.loader_env(), &delves_difficulty_picker_toc());

    let reward_button_template = wow_ui_sim::xml::get_template("DelveRewardItemButtonTemplate");
    let bountiful_animation_template =
        wow_ui_sim::xml::get_template("BountifulWidgetAnimationTemplate");

    assert!(
        reward_button_template.is_some(),
        "DelveRewardItemButtonTemplate (Blizzard_DelvesDifficultyPicker.xml:4 — virtual=true, \
         inherits LargeItemButtonTemplate, mixin DelveRewardsButtonMixin) should be registered \
         with the XML template registry so DelveRewardsContainerFrameMixin:SetRewards can spawn \
         per-row reward buttons"
    );
    assert!(
        bountiful_animation_template.is_some(),
        "BountifulWidgetAnimationTemplate (Blizzard_DelvesDifficultyPicker.xml:14 — virtual=true) \
         should be registered with the XML template registry — it owns the VFX overlay shown \
         on top of the bountiful UI widget when the player holds a delve key"
    );
}
}

#[test]
fn blizzard_delves_difficulty_picker_required_events_are_registerable() {
    assert!(
        wow_ui_sim::event::is_registerable_event("WALK_IN_DATA_UPDATE"),
        "WALK_IN_DATA_UPDATE should be a registerable event (src/event/valid_events_c.rs:542) — \
         drives the difficulty-picker refresh when delve walk-in data changes"
    );
    assert!(
        wow_ui_sim::event::is_registerable_event("ACTIVE_DELVE_DATA_UPDATE"),
        "ACTIVE_DELVE_DATA_UPDATE should be a registerable event (src/event/valid_events_a.rs:36)"
    );
    assert!(
        wow_ui_sim::event::is_registerable_event("PARTY_ELIGIBILITY_FOR_DELVE_TIERS_CHANGED"),
        "PARTY_ELIGIBILITY_FOR_DELVE_TIERS_CHANGED should be a registerable event \
         (src/event/valid_events_b.rs:334) — fires when a party member's tier eligibility \
         changes so the picker can re-disable / re-enable tiers"
    );
    assert!(
        wow_ui_sim::event::is_registerable_event("PARTY_LEADER_CHANGED"),
        "PARTY_LEADER_CHANGED should be a registerable event"
    );
    assert!(
        wow_ui_sim::event::is_registerable_event("GROUP_LEFT"),
        "GROUP_LEFT should be a registerable event"
    );
}
