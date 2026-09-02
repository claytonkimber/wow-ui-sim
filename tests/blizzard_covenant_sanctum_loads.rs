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

fn covenant_sanctum_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_CovenantSanctum/Blizzard_CovenantSanctum.toc")
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
fn blizzard_covenant_sanctum_toc_is_load_on_demand() {
    let toc = TocFile::from_file(&covenant_sanctum_toc())
        .expect("Blizzard_CovenantSanctum TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_CovenantSanctum declares `## LoadOnDemand: 1` (the Sanctum upgrade panel is \
         opened by the Sanctum NPC interaction, so it must NOT auto-load on Game-screen \
         bring-up)"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_CovenantSanctum does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_CovenantSanctum has no `## Dependencies` line — its TOC is just \
         Title/Author/`## LoadOnDemand: 1` plus Blizzard_CovenantSanctum.xml + \
         Blizzard_CovenantSanctumUpgrades.xml"
    );
}

#[test]
fn blizzard_covenant_sanctum_is_absent_from_game_auto_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CovenantSanctum");
    assert!(
        !in_game,
        "Blizzard_CovenantSanctum is `## LoadOnDemand: 1`, so it must NOT appear in \
         Game-screen auto-discovery — the Sanctum NPC interaction loads it via \
         UIParentLoadAddOn"
    );
}

prefork_full_ui_case! {
fn blizzard_covenant_sanctum_loads_via_load_addon_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &covenant_sanctum_toc())
        .expect("Blizzard_CovenantSanctum should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_CovenantSanctum emitted Lua errors during explicit load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_sanctum_frame_is_defined_and_parented_to_uiparent(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_sanctum_toc())
        .expect("Blizzard_CovenantSanctum should load via Rust loader");

    let frame_present: bool = env
        .eval(
            "return type(_G.CovenantSanctumFrame) == 'table' \
                and CovenantSanctumFrame:GetParent() == UIParent \
                and CovenantSanctumFrame:IsShown() == false \
                and type(CovenantSanctumFrame.UpgradesTab) == 'table'",
        )
        .expect("CovenantSanctumFrame query should succeed");
    assert!(
        frame_present,
        "Blizzard_CovenantSanctum.xml line 5 should define `CovenantSanctumFrame` (parent \
         UIParent, hidden=true, enableMouse=true, toplevel=true, mixin=CovenantSanctumMixin) \
         with a child Frame parentKey=UpgradesTab (line 447 of Blizzard_CovenantSanctumUpgrades.xml, \
         setAllPoints=true, mixin=CovenantSanctumUpgradesTabMixin) — the panel must start \
         hidden and the UpgradesTab subframe must be reachable"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_sanctum_main_mixin_methods_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_sanctum_toc())
        .expect("Blizzard_CovenantSanctum should load via Rust loader");

    let methods_present: bool = env
        .eval(
            "return type(CovenantSanctumMixin) == 'table' \
                and type(CovenantSanctumMixin.OnLoad) == 'function' \
                and type(CovenantSanctumMixin.OnShow) == 'function' \
                and type(CovenantSanctumMixin.OnHide) == 'function' \
                and type(CovenantSanctumMixin.InteractionStarted) == 'function' \
                and type(CovenantSanctumMixin.SetCovenantInfo) == 'function' \
                and type(CovenantSanctumMixin.GetCovenantID) == 'function' \
                and type(CovenantSanctumMixin.GetCovenantData) == 'function'",
        )
        .expect("CovenantSanctumMixin query should succeed");
    assert!(
        methods_present,
        "CovenantSanctumMixin should expose its 7 methods (OnLoad calls \
         RegisterUIPanel(CovenantSanctumFrame, {{area='center', pushable=0, \
         allowOtherPanels=0}}); OnShow plays UI_COVENANT_SANCTUM_OPEN_WINDOW; OnHide calls \
         C_CovenantSanctumUI.EndInteraction + plays close sound; InteractionStarted seeds \
         covenant data + ShowUIPanel; SetCovenantInfo applies NineSliceUtil + atlas + close \
         button border for the active covenant textureKit; GetCovenantID and GetCovenantData \
         expose the cached values used by the upgrades panel)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_sanctum_upgrades_tab_mixin_methods_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_sanctum_toc())
        .expect("Blizzard_CovenantSanctum should load via Rust loader");

    let methods_present: bool = env
        .eval(
            "return type(CovenantSanctumUpgradesTabMixin) == 'table' \
                and type(CovenantSanctumUpgradesTabMixin.OnLoad) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.OnShow) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.OnHide) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.OnEvent) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.OnResearchStarted) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.OnAnimaGained) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.OnAnimaGainEffectMissileFinished) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.OnAnimaGainEffectImpactFinished) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.OnCurrencyUpdate) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.Refresh) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.HasAnyTalents) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.UpdateDepositButton) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.SetSelectedTree) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.GetSelectedTree) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.GetSelectedTreeDescriptionText) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.DepositAnima) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.SetUpCurrencies) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.SetUpUpgrades) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.SetUpTextureKits) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.UpdateCurrencies) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.GetSortedResearchCurrencyCosts) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.CheckTutorials) == 'function' \
                and type(CovenantSanctumUpgradesTabMixin.HasAnySoulCurrencies) == 'function'",
        )
        .expect("CovenantSanctumUpgradesTabMixin query should succeed");
    assert!(
        methods_present,
        "CovenantSanctumUpgradesTabMixin should expose its 23 methods (OnLoad/OnShow/OnHide/\
         OnEvent + the 4 anima FX dispatchers OnResearchStarted/OnAnimaGained/\
         OnAnimaGainEffectMissileFinished/OnAnimaGainEffectImpactFinished/OnCurrencyUpdate; \
         Refresh / HasAnyTalents / UpdateDepositButton / SetSelectedTree / GetSelectedTree / \
         GetSelectedTreeDescriptionText / DepositAnima driving the talent-tree selection \
         flow; SetUpCurrencies / SetUpUpgrades / SetUpTextureKits / UpdateCurrencies / \
         GetSortedResearchCurrencyCosts / HasAnySoulCurrencies for the per-covenant \
         currency-bar and upgrade-tier textures; CheckTutorials gating the upgrade-tutorial \
         HelpTip)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_sanctum_upgrade_node_mixins_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_sanctum_toc())
        .expect("Blizzard_CovenantSanctum should load via Rust loader");

    let mixins_present: bool = env
        .eval(
            "return type(CovenantSanctumUpgradeBaseMixin) == 'table' \
                and type(CovenantSanctumUpgradeBaseMixin.Refresh) == 'function' \
                and type(CovenantSanctumUpgradeBaseMixin.GetTier) == 'function' \
                and type(CovenantSanctumUpgradeBaseMixin.GetDescriptionText) == 'function' \
                and type(CovenantSanctumUpgradeBaseMixin.OnMouseDown) == 'function' \
                and type(CovenantSanctumUpgradeBaseMixin.OnEnter) == 'function' \
                and type(CovenantSanctumUpgradeBaseMixin.RefreshTooltip) == 'function' \
                and type(CovenantSanctumUpgradeBaseMixin.OnLeave) == 'function' \
                and type(CovenantSanctumUpgradeBaseMixin.IsSelected) == 'function' \
                and type(CovenantSanctumUpgradeBaseMixin.SetUpTextureKit) == 'function' \
                and type(CovenantSanctumUpgradeTreeMixin) == 'table' \
                and CovenantSanctumUpgradeTreeMixin.Refresh == CovenantSanctumUpgradeBaseMixin.Refresh \
                and type(CovenantSanctumUpgradeReservoirMixin) == 'table' \
                and type(CovenantSanctumUpgradeReservoirMixin.OnHide) == 'function' \
                and type(CovenantSanctumUpgradeReservoirMixin.OnEnter) == 'function' \
                and type(CovenantSanctumUpgradeReservoirMixin.OnLeave) == 'function' \
                and type(CovenantSanctumUpgradeReservoirMixin.OnUpdate) == 'function' \
                and type(CovenantSanctumUpgradeReservoirMixin.SetUpTextureKit) == 'function' \
                and type(CovenantSanctumUpgradeReservoirMixin.SetUpAnimations) == 'function' \
                and type(CovenantSanctumUpgradeReservoirMixin.Refresh) == 'function' \
                and type(CovenantSanctumUpgradeReservoirMixin.UpdateAnima) == 'function' \
                and type(CovenantSanctumUpgradeReservoirMixin.UpdateFullSound) == 'function' \
                and type(CovenantSanctumUpgradeReservoirMixin.GetAnimaAmount) == 'function' \
                and type(CovenantSanctumUpgradeReservoirMixin.StartAnimaGainEffect) == 'function' \
                and type(CovenantSanctumUpgradeReservoirMixin.CancelAnimaGainEffect) == 'function'",
        )
        .expect("upgrade-node mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_CovenantSanctumUpgrades.lua should define a 3-level upgrade-node mixin \
         hierarchy: CovenantSanctumUpgradeBaseMixin (9 methods — Refresh/GetTier/\
         GetDescriptionText/OnMouseDown/OnEnter/RefreshTooltip/OnLeave/IsSelected/\
         SetUpTextureKit), CovenantSanctumUpgradeTreeMixin (CreateFromMixins(BaseMixin) — \
         used by the talent-tree subnodes), and CovenantSanctumUpgradeReservoirMixin \
         (CreateFromMixins(BaseMixin) + 12 reservoir-specific methods OnHide/OnEnter/OnLeave/\
         OnUpdate/SetUpTextureKit/SetUpAnimations/Refresh/UpdateAnima/UpdateFullSound/\
         GetAnimaAmount/StartAnimaGainEffect/CancelAnimaGainEffect for the per-covenant \
         anima reservoir)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_sanctum_per_node_helper_mixins_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_sanctum_toc())
        .expect("Blizzard_CovenantSanctum should load via Rust loader");

    let mixins_present: bool = env
        .eval(
            "return type(CovenantSanctumUpgradeTalentListMixin) == 'table' \
                and type(CovenantSanctumUpgradeTalentListMixin.OnLoad) == 'function' \
                and type(CovenantSanctumUpgradeTalentListMixin.Refresh) == 'function' \
                and type(CovenantSanctumUpgradeTalentListMixin.Upgrade) == 'function' \
                and type(CovenantSanctumUpgradeTalentListMixin.FindTalentButton) == 'function' \
                and type(CovenantSanctumIntroBoxMixin) == 'table' \
                and type(CovenantSanctumIntroBoxMixin.SetTalent) == 'function' \
                and type(CovenantSanctumIntroBoxMixin.SetStatusText) == 'function' \
                and type(CovenantSanctumIntroBoxMixin.UpdateResearchTime) == 'function' \
                and type(CovenantSanctumUpgradeTalentMixin) == 'table' \
                and type(CovenantSanctumUpgradeTalentMixin.Set) == 'function' \
                and type(CovenantSanctumUpgradeTalentMixin.OnEnter) == 'function' \
                and type(CovenantSanctumUpgradeTalentMixin.RefreshTooltip) == 'function' \
                and type(CovenantSanctumUpgradeButtonMixin) == 'table' \
                and type(CovenantSanctumUpgradeButtonMixin.OnClick) == 'function'",
        )
        .expect("per-node helper mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_CovenantSanctumUpgrades.lua should define 4 per-node helper mixins: \
         CovenantSanctumUpgradeTalentListMixin (4 methods OnLoad/Refresh/Upgrade/\
         FindTalentButton driving the talent button pool), CovenantSanctumIntroBoxMixin (3 \
         methods SetTalent/SetStatusText/UpdateResearchTime for the per-tier intro panel), \
         CovenantSanctumUpgradeTalentMixin (3 methods Set/OnEnter/RefreshTooltip for each \
         talent button), and CovenantSanctumUpgradeButtonMixin (single OnClick triggering \
         C_Garrison.ResearchTalent — used by the inherited UIPanelButtonTemplate \
         UpgradeButton)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_sanctum_xml_templates_are_registered(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_sanctum_toc())
        .expect("Blizzard_CovenantSanctum should load via Rust loader");

    for template_name in [
        "CovenantSanctumUpgradeTreeTemplate",
        "CovenantSanctumUpgradeReservoirTemplate",
        "CovenantSanctumUpgradeTalentTemplate",
    ] {
        assert!(
            wow_ui_sim::xml::get_template(template_name).is_some(),
            "{template_name} (`<Frame virtual=\"true\">` from \
             Blizzard_CovenantSanctumUpgrades.xml) should be registered in the template \
             registry after Blizzard_CovenantSanctum loads — the upgrade-node templates \
             populate the per-tier rows on the UpgradesTab"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_covenant_sanctum_uipanel_registration_uses_center_area(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_sanctum_toc())
        .expect("Blizzard_CovenantSanctum should load via Rust loader");

    let entry_present: bool = env
        .eval(
            "local entry = UIPanelWindows and UIPanelWindows['CovenantSanctumFrame']; \
             return type(entry) == 'table' \
                and entry.area == 'center' \
                and entry.pushable == 0 \
                and entry.allowOtherPanels == 0",
        )
        .expect("UIPanelWindows entry query should succeed");
    assert!(
        entry_present,
        "CovenantSanctumMixin:OnLoad calls `RegisterUIPanel(CovenantSanctumFrame, \
         {{area='center', pushable=0, allowOtherPanels=0}})` — UIPanelWindows must expose \
         this entry so the panel participates in the central UIParent panel-stacking system \
         (note: allowOtherPanels=0 means the Sanctum window is exclusive, unlike the renown \
         track which sits in 'left' with allowOtherPanels=1)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_sanctum_garrison_talent_availability_enum_matches_expected_values(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_sanctum_toc())
        .expect("Blizzard_CovenantSanctum should load via Rust loader");

    let enum_ok: bool = env
        .eval(
            "return type(Enum.GarrisonTalentAvailability) == 'table' \
                and type(Enum.GarrisonTalentAvailability.UnavailableAlreadyHave) == 'number'",
        )
        .expect("Enum.GarrisonTalentAvailability query should succeed");
    assert!(
        enum_ok,
        "Blizzard_CovenantSanctumUpgrades.lua line 4 keys the local GetCurrentTier helper \
         off Enum.GarrisonTalentAvailability.UnavailableAlreadyHave to count completed tiers \
         — the simulator's missing_enums.lua must populate Enum.GarrisonTalentAvailability \
         with the UnavailableAlreadyHave variant before this addon can determine the next \
         upgrade tier"
    );
}
}
