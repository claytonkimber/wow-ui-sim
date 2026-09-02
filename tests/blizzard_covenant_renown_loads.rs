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

fn covenant_renown_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_CovenantRenown/Blizzard_CovenantRenown.toc")
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
fn blizzard_covenant_renown_toc_is_load_on_demand() {
    let toc = TocFile::from_file(&covenant_renown_toc())
        .expect("Blizzard_CovenantRenown TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_CovenantRenown declares `## LoadOnDemand: 1` (the Shadowlands renown track \
         panel is opened via the Sanctum NPC interaction or the toggle command, so it must \
         NOT auto-load on Game-screen bring-up)"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_CovenantRenown does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_CovenantRenown has no `## Dependencies` line — its TOC is just \
         Title/Author/`## LoadOnDemand: 1` plus Blizzard_CovenantRenown.xml"
    );
}

#[test]
fn blizzard_covenant_renown_is_absent_from_game_auto_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CovenantRenown");
    assert!(
        !in_game,
        "Blizzard_CovenantRenown is `## LoadOnDemand: 1`, so it must NOT appear in \
         Game-screen auto-discovery — the Sanctum NPC interaction loads it via \
         UIParentLoadAddOn"
    );
}

prefork_full_ui_case! {
fn blizzard_covenant_renown_loads_via_load_addon_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &covenant_renown_toc())
        .expect("Blizzard_CovenantRenown should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_CovenantRenown emitted Lua errors during explicit load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_renown_frame_is_defined_and_parented_to_uiparent(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_renown_toc())
        .expect("Blizzard_CovenantRenown should load via Rust loader");

    let frame_present: bool = env
        .eval(
            "return type(_G.CovenantRenownFrame) == 'table' \
                and CovenantRenownFrame:GetParent() == UIParent \
                and CovenantRenownFrame:IsShown() == false \
                and type(CovenantRenownFrame.HeaderFrame) == 'table'",
        )
        .expect("CovenantRenownFrame query should succeed");
    assert!(
        frame_present,
        "Blizzard_CovenantRenown.xml line 66 should define `CovenantRenownFrame` (parent \
         UIParent, toplevel=true, hidden=true, enableMouse=true, mixin=CovenantRenownMixin) \
         with a child Frame parentKey=HeaderFrame (line 137, mixin=\
         CovenantRenownHeaderFrameMixin) — the renown panel must start hidden and the \
         HeaderFrame must be reachable for the OnEvent dispatch hover-tooltip refresh"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_renown_main_mixin_methods_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_renown_toc())
        .expect("Blizzard_CovenantRenown should load via Rust loader");

    let methods_present: bool = env
        .eval(
            "return type(CovenantRenownMixin) == 'table' \
                and type(CovenantRenownMixin.OnLoad) == 'function' \
                and type(CovenantRenownMixin.OnShow) == 'function' \
                and type(CovenantRenownMixin.OnHide) == 'function' \
                and type(CovenantRenownMixin.OnEvent) == 'function' \
                and type(CovenantRenownMixin.OnMouseWheel) == 'function' \
                and type(CovenantRenownMixin.SetUpCovenantData) == 'function' \
                and type(CovenantRenownMixin.GetLevels) == 'function' \
                and type(CovenantRenownMixin.Refresh) == 'function' \
                and type(CovenantRenownMixin.SelectLevel) == 'function' \
                and type(CovenantRenownMixin.OnTrackUpdate) == 'function' \
                and type(CovenantRenownMixin.OnLevelEffectFinished) == 'function' \
                and type(CovenantRenownMixin.PlayLevelEffect) == 'function' \
                and type(CovenantRenownMixin.CancelLevelEffect) == 'function' \
                and type(CovenantRenownMixin.SetCelebrationSwirlEffects) == 'function' \
                and type(CovenantRenownMixin.SetRewards) == 'function' \
                and type(CovenantRenownMixin.CheckTutorials) == 'function'",
        )
        .expect("CovenantRenownMixin query should succeed");
    assert!(
        methods_present,
        "CovenantRenownMixin should expose its 15 methods (OnLoad calls \
         RegisterUIPanel(CovenantRenownFrame, {{area='left', pushable=0, allowOtherPanels=1, \
         width=755, height=540}}) and seeds rewardsPool=CreateFramePool('FRAME', self, \
         'CovenantRenownRewardTemplate'); OnShow registers \
         COVENANT_SANCTUM_RENOWN_LEVEL_CHANGED + COVENANT_RENOWN_CATCH_UP_STATE_UPDATE + \
         plays UI_COVENANT_RENOWN_OPEN_WINDOW; OnHide saves the displayed level into \
         CVar `lastRenownForCovenant<id>`; OnMouseWheel scrolls the TrackFrame; \
         SetUpCovenantData / GetLevels / Refresh / SelectLevel / OnTrackUpdate / \
         OnLevelEffectFinished / PlayLevelEffect / CancelLevelEffect drive the renown-track \
         UI; SetCelebrationSwirlEffects / SetRewards manage the model-scene fanfare + per-\
         level reward tiles; CheckTutorials gates the LE_FRAME_TUTORIAL_COVENANT_RENOWN_REWARDS \
         and PROGRESS HelpTip flow)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_renown_reward_and_header_mixins_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_renown_toc())
        .expect("Blizzard_CovenantRenown should load via Rust loader");

    let mixins_present: bool = env
        .eval(
            "return type(CovenantRenownRewardMixin) == 'table' \
                and type(CovenantRenownRewardMixin.SetReward) == 'function' \
                and type(CovenantRenownRewardMixin.RefreshReward) == 'function' \
                and type(CovenantRenownRewardMixin.OnEnter) == 'function' \
                and type(CovenantRenownHeaderFrameMixin) == 'table' \
                and type(CovenantRenownHeaderFrameMixin.OnEnter) == 'function' \
                and type(CovenantRenownHeaderFrameMixin.OnLeave) == 'function' \
                and type(CovenantRenownHeaderFrameMixin.SetupRenownAvailableIcon) == 'function'",
        )
        .expect("reward / header mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_CovenantRenown.lua should define 2 secondary mixins: \
         CovenantRenownRewardMixin (3 methods — SetReward applies the per-covenant \
         rewardTextureKitRegions + Check/icon visibility, RefreshReward pulls icon/name/\
         description via RenownRewardUtil.GetRenownRewardInfo, OnEnter shows GameTooltip) \
         and CovenantRenownHeaderFrameMixin (3 methods — OnEnter shows the level-state \
         GameTooltip switching between RENOWN_LEVEL_MAXIMUM / CAUGHT_UP / CATCH_UP_MODE / \
         CURRENT based on C_CovenantSanctumUI.HasMaximumRenown / IsWeeklyRenownCapped / \
         IsPlayerInRenownCatchUpMode, OnLeave hides the tooltip, SetupRenownAvailableIcon \
         toggles the renown-available chevron based on weekly-cap + maximum)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_renown_xml_template_is_registered(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_renown_toc())
        .expect("Blizzard_CovenantRenown should load via Rust loader");

    assert!(
        wow_ui_sim::xml::get_template("CovenantRenownRewardTemplate").is_some(),
        "CovenantRenownRewardTemplate (`<Frame virtual=\"true\" mixin=\"CovenantRenownRewardMixin\" \
         frameLevel=\"10\">` from Blizzard_CovenantRenown.xml line 5) should be registered in \
         the template registry after Blizzard_CovenantRenown loads — the rewardsPool created \
         in OnLoad depends on this template"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_renown_rewards_pool_is_created_on_load(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_renown_toc())
        .expect("Blizzard_CovenantRenown should load via Rust loader");

    let pool_ready: bool = env
        .eval(
            "return type(CovenantRenownFrame.rewardsPool) == 'table' \
                and type(CovenantRenownFrame.rewardsPool.Acquire) == 'function' \
                and type(CovenantRenownFrame.rewardsPool.ReleaseAll) == 'function' \
                and CovenantRenownFrame.FinalToastSlabTexture == CovenantRenownFrame.FinalToast.SlabTexture",
        )
        .expect("frame pool / FinalToastSlabTexture query should succeed");
    assert!(
        pool_ready,
        "CovenantRenownMixin:OnLoad (line 54-67) should populate \
         self.rewardsPool=CreateFramePool('FRAME', self, 'CovenantRenownRewardTemplate') and \
         alias self.FinalToastSlabTexture=self.FinalToast.SlabTexture — the rewards pool must \
         expose Acquire / ReleaseAll for SetRewards, and the FinalToast slab texture must be \
         reachable via the alias (used by mainTextureKitRegions in SetupTextureKit)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_renown_uipanel_registration_uses_left_area(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_renown_toc())
        .expect("Blizzard_CovenantRenown should load via Rust loader");

    let entry_present: bool = env
        .eval(
            "local entry = UIPanelWindows and UIPanelWindows['CovenantRenownFrame']; \
             return type(entry) == 'table' \
                and entry.area == 'left' \
                and entry.pushable == 0 \
                and entry.allowOtherPanels == 1 \
                and entry.width == 755 \
                and entry.height == 540",
        )
        .expect("UIPanelWindows entry query should succeed");
    assert!(
        entry_present,
        "CovenantRenownMixin:OnLoad calls `RegisterUIPanel(CovenantRenownFrame, {{area='left', \
         pushable=0, allowOtherPanels=1, width=755, height=540}})` — UIPanelWindows must \
         expose this entry so the panel participates in the central UIParent panel-stacking \
         system"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_renown_covenant_type_enum_matches_expected_values(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_renown_toc())
        .expect("Blizzard_CovenantRenown should load via Rust loader");

    let enum_ok: bool = env
        .eval(
            "return type(Enum.CovenantType) == 'table' \
                and Enum.CovenantType.None == 0 \
                and Enum.CovenantType.Kyrian == 1 \
                and Enum.CovenantType.Venthyr == 2 \
                and Enum.CovenantType.NightFae == 3 \
                and Enum.CovenantType.Necrolord == 4",
        )
        .expect("Enum.CovenantType query should succeed");
    assert!(
        enum_ok,
        "Blizzard_CovenantRenown.lua keys two internal tables (`finalToastSwirlEffects` \
         lines 24-29 and `levelEffects` lines 31-36) off Enum.CovenantType.Kyrian / Venthyr / \
         NightFae / Necrolord — the simulator's missing_enums.lua must populate \
         {{None=0, Kyrian=1, Venthyr=2, NightFae=3, Necrolord=4}} before this addon can pick \
         the per-covenant ModelScene swirl/level FX"
    );
}
}
