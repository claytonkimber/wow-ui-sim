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

fn covenant_preview_ui_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_CovenantPreviewUI/Blizzard_CovenantPreviewUI.toc")
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
fn blizzard_covenant_preview_ui_toc_is_load_on_demand() {
    let toc = TocFile::from_file(&covenant_preview_ui_toc())
        .expect("Blizzard_CovenantPreviewUI TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_CovenantPreviewUI declares `## LoadOnDemand: 1` (the Shadowlands covenant \
         preview/selection panel is shown only when the player triggers the covenant choice \
         flow at level 60, so it must NOT auto-load on Game-screen bring-up)"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_CovenantPreviewUI does not declare UseSecureEnvironment"
    );
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_GameMenuEsc".to_string()],
        "Blizzard_CovenantPreviewUI declares Blizzard_GameMenuEsc so its Escape handler is \
         available before the preview frame loads"
    );
}

#[test]
fn blizzard_covenant_preview_ui_is_absent_from_game_auto_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CovenantPreviewUI");
    assert!(
        !in_game,
        "Blizzard_CovenantPreviewUI is `## LoadOnDemand: 1`, so it must NOT appear in \
         Game-screen auto-discovery — it is loaded by the COVENANT_PREVIEW_OPEN_FROM_UI / \
         OPEN_JOURNAL flow via UIParentLoadAddOn"
    );
}

prefork_full_ui_case! {
fn blizzard_covenant_preview_ui_loads_via_load_addon_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &covenant_preview_ui_toc())
        .expect("Blizzard_CovenantPreviewUI should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_CovenantPreviewUI emitted Lua errors during explicit load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_preview_frame_is_defined_and_parented_to_uiparent(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_preview_ui_toc())
        .expect("Blizzard_CovenantPreviewUI should load via Rust loader");

    let frame_present: bool = env
        .eval(
            "return type(_G.CovenantPreviewFrame) == 'table' \
                and CovenantPreviewFrame:GetParent() == UIParent \
                and CovenantPreviewFrame:IsShown() == false",
        )
        .expect("CovenantPreviewFrame query should succeed");
    assert!(
        frame_present,
        "Blizzard_CovenantPreviewUI.xml line 57 should define the toplevel \
         `CovenantPreviewFrame` (parent UIParent, frameStrata=DIALOG, hidden=true, \
         enableMouse=true, mixin=CovenantPreviewFrameMixin) — it must start hidden until \
         CovenantPreviewFrame:TryShow(covenantInfo) is called"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_preview_frame_mixin_methods_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_preview_ui_toc())
        .expect("Blizzard_CovenantPreviewUI should load via Rust loader");

    let methods_present: bool = env
        .eval(
            "return type(CovenantPreviewFrameMixin) == 'table' \
                and type(CovenantPreviewFrameMixin.OnLoad) == 'function' \
                and type(CovenantPreviewFrameMixin.OnShow) == 'function' \
                and type(CovenantPreviewFrameMixin.OnHide) == 'function' \
                and type(CovenantPreviewFrameMixin.OnEvent) == 'function' \
                and type(CovenantPreviewFrameMixin.HandleEscape) == 'function' \
                and type(CovenantPreviewFrameMixin.Reset) == 'function' \
                and type(CovenantPreviewFrameMixin.SetupTextureKits) == 'function' \
                and type(CovenantPreviewFrameMixin.SetupFramesWithTextureKit) == 'function' \
                and type(CovenantPreviewFrameMixin.TryShow) == 'function' \
                and type(CovenantPreviewFrameMixin.SetupCovenantFeature) == 'function' \
                and type(CovenantPreviewFrameMixin.SetupAbilityButtons) == 'function' \
                and type(CovenantPreviewFrameMixin.SetupAndGetAbilityButton) == 'function' \
                and type(CovenantPreviewFrameMixin.SetupSoulbindButtons) == 'function' \
                and type(CovenantPreviewFrameMixin.SetupAndGetSoulbindButton) == 'function' \
                and type(CovenantPreviewFrameMixin.SetupModelSceneFrame) == 'function' \
                and type(CovenantPreviewFrameMixin.SetupCovenantInfoPanel) == 'function'",
        )
        .expect("CovenantPreviewFrameMixin query should succeed");
    assert!(
        methods_present,
        "CovenantPreviewFrameMixin should expose its 16 methods (OnLoad creates the \
         AbilityButtons + Soulbinds frame pools; OnShow registers \
         COVENANT_PREVIEW_CLOSE/PLAYER_CHOICE_CLOSE + UIPanelUpdateScaleForFit; OnHide \
         cleans up + calls C_CovenantPreview.CloseFromUI; OnEvent dispatch; HandleEscape; \
         Reset; SetupTextureKits + SetupFramesWithTextureKit applying per-covenant \
         NineSliceUtil layouts; TryShow as the public entry point; the Setup* helpers for \
         covenant feature/abilities/soulbinds/model scene/info panel)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_preview_ability_and_feature_and_soulbind_mixins_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_preview_ui_toc())
        .expect("Blizzard_CovenantPreviewUI should load via Rust loader");

    let mixins_present: bool = env
        .eval(
            "return type(CovenantAbilityButtonMixin) == 'table' \
                and type(CovenantAbilityButtonMixin.OnEnter) == 'function' \
                and type(CovenantAbilityButtonMixin.OnLeave) == 'function' \
                and type(CovenantAbilityButtonMixin.SetupButton) == 'function' \
                and type(CovenantFeatureButtonMixin) == 'table' \
                and type(CovenantFeatureButtonMixin.Setup) == 'function' \
                and type(CovenantFeatureButtonMixin.OnEnter) == 'function' \
                and type(CovenantFeatureButtonMixin.OnLeave) == 'function' \
                and type(CovenantSoulbindButtonMixin) == 'table' \
                and type(CovenantSoulbindButtonMixin.SetupButton) == 'function' \
                and type(CovenantSoulbindButtonMixin.OnEnter) == 'function' \
                and type(CovenantSoulbindButtonMixin.OnLeave) == 'function' \
                and type(CovenantPreviewModelSceneContainerMixin) == 'table' \
                and type(CovenantPreviewModelSceneContainerMixin.ShouldAcceptDressUp) == 'function'",
        )
        .expect("per-button mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_CovenantPreviewUI.lua should define its 4 secondary mixins: \
         CovenantAbilityButtonMixin (3 methods — OnEnter/OnLeave wired to GameTooltip + \
         SetupButton fetching C_Spell.GetSpellTexture for the ability icon), \
         CovenantFeatureButtonMixin (3 methods — Setup populating self.name/description, \
         OnEnter/OnLeave with GameTooltip_AddHighlightLine/AddNormalLine), \
         CovenantSoulbindButtonMixin (3 methods — SetupButton + OnEnter using \
         Spell:CreateFromSpellID with ContinueWithCancelOnSpellLoad to populate \
         EmbeddedItemTooltip + OnLeave cancelling the callback), and \
         CovenantPreviewModelSceneContainerMixin (single ShouldAcceptDressUp returning false)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_preview_xml_templates_are_registered(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_preview_ui_toc())
        .expect("Blizzard_CovenantPreviewUI should load via Rust loader");

    for template_name in [
        "CovenantAbilityButtonTemplate",
        "CovenantSoulbindButtonTemplate",
    ] {
        assert!(
            wow_ui_sim::xml::get_template(template_name).is_some(),
            "{template_name} (`<Button virtual=\"true\">` from Blizzard_CovenantPreviewUI.xml) \
             should be registered in the template registry after Blizzard_CovenantPreviewUI \
             loads — the AbilityButtonsPool/SoulbindButtonsPool both depend on these"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_covenant_preview_frame_pools_are_created_on_load(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_preview_ui_toc())
        .expect("Blizzard_CovenantPreviewUI should load via Rust loader");

    let pools_ready: bool = env
        .eval(
            "return type(CovenantPreviewFrame.AbilityButtonsPool) == 'table' \
                and type(CovenantPreviewFrame.AbilityButtonsPool.Acquire) == 'function' \
                and type(CovenantPreviewFrame.AbilityButtonsPool.ReleaseAll) == 'function' \
                and type(CovenantPreviewFrame.SoulbindButtonsPool) == 'table' \
                and type(CovenantPreviewFrame.SoulbindButtonsPool.Acquire) == 'function' \
                and type(CovenantPreviewFrame.SoulbindButtonsPool.ReleaseAll) == 'function'",
        )
        .expect("frame pool query should succeed");
    assert!(
        pools_ready,
        "CovenantPreviewFrameMixin:OnLoad (line 71-74) should populate \
         self.AbilityButtonsPool=CreateFramePool('BUTTON', self.InfoPanel, \
         'CovenantAbilityButtonTemplate') and self.SoulbindButtonsPool=CreateFramePool('BUTTON', \
         self.InfoPanel, 'CovenantSoulbindButtonTemplate') — both must expose Acquire and \
         ReleaseAll for the per-covenant ability/soulbind row population during TryShow"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_preview_covenant_ability_type_enum_matches_expected_values(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_preview_ui_toc())
        .expect("Blizzard_CovenantPreviewUI should load via Rust loader");

    let enum_ok: bool = env
        .eval(
            "return type(Enum.CovenantAbilityType) == 'table' \
                and Enum.CovenantAbilityType.Class == 0 \
                and Enum.CovenantAbilityType.Signature == 1 \
                and Enum.CovenantAbilityType.Soulbind == 2",
        )
        .expect("Enum.CovenantAbilityType query should succeed");
    assert!(
        enum_ok,
        "Blizzard_CovenantPreviewUI keys an internal abilityTypeText table off \
         Enum.CovenantAbilityType.Class / .Signature (line 65-68 in \
         Blizzard_CovenantPreviewUI.lua) — the simulator's missing_enums.lua must populate \
         {{Class=0, Signature=1, Soulbind=2}} before this addon can label its preview rows"
    );
}
}
