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

fn customization_ui_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_CustomizationUI/Blizzard_CustomizationUI.toc")
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
fn blizzard_customization_ui_toc_is_load_on_demand_for_both_screens() {
    let toc = TocFile::from_file(&customization_ui_toc())
        .expect("Blizzard_CustomizationUI TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_CustomizationUI declares `## LoadOnDemand: 1` (the customization template \
         library is brought in on-demand by CharacterCreate / BarberShop / etc. via \
         UIParentLoadAddOn — must NOT auto-load on Game-screen bring-up)"
    );
    assert!(
        toc.allows_screen(ScreenKind::Game),
        "Blizzard_CustomizationUI declares `## AllowLoad: Both` so it must be loadable from \
         the Game screen (BarberShop)"
    );
    assert!(
        toc.allows_screen(ScreenKind::Login) || toc.allows_screen(ScreenKind::CharacterSelect),
        "Blizzard_CustomizationUI declares `## AllowLoad: Both` so it must also be loadable \
         from the glue screens (CharacterCreate)"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_CustomizationUI does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_CustomizationUI has no `## Dependencies` line — it is a standalone template \
         library that consumers (Blizzard_CharacterCreate / Blizzard_BarbershopUI) declare a \
         dep on, not the other way around"
    );
}

#[test]
fn blizzard_customization_ui_is_absent_from_game_auto_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CustomizationUI");
    assert!(
        !in_game,
        "Blizzard_CustomizationUI is `## LoadOnDemand: 1`, so it must NOT appear in \
         Game-screen auto-discovery — it is loaded explicitly by Blizzard_CharacterCreate / \
         Blizzard_BarbershopUI via UIParentLoadAddOn"
    );
}

prefork_full_ui_case! {
fn blizzard_customization_ui_loads_via_load_addon_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &customization_ui_toc())
        .expect("Blizzard_CustomizationUI should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_CustomizationUI emitted Lua errors during explicit load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_customization_ui_util_helper_is_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &customization_ui_toc())
        .expect("Blizzard_CustomizationUI should load via Rust loader");

    let util_present: bool = env
        .eval(
            "return type(CustomizationUtil) == 'table' \
                and type(CustomizationUtil.UpdateShowDebugTooltipInfo) == 'function' \
                and type(CustomizationUtil.ShouldShowDebugTooltipInfo) == 'function'",
        )
        .expect("CustomizationUtil query should succeed");
    assert!(
        util_present,
        "Blizzard_CustomizationUtil.lua line 6 should publish the global namespace \
         `CustomizationUtil = {{}}` with two helpers (UpdateShowDebugTooltipInfo / \
         ShouldShowDebugTooltipInfo) backed by the file-local `showDebugTooltipInfo` cached \
         from `GetCVarBool('debugTargetInfo')`"
    );
}
}

prefork_full_ui_case! {
fn blizzard_customization_ui_template_base_mixins_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &customization_ui_toc())
        .expect("Blizzard_CustomizationUI should load via Rust loader");

    let mixins_present: bool = env
        .eval(
            "return type(CustomizationContentFrameMixin) == 'table' \
                and type(CustomizationBaseButtonMixin) == 'table' \
                and type(CustomizationFrameWithTooltipMixin) == 'table' \
                and type(CustomizationMaskedButtonMixin) == 'table' \
                and type(CustomizationFrameWithExpandableTooltipMixin) == 'table' \
                and type(CustomizationSmallButtonMixin) == 'table' \
                and type(CustomizationClickOrHoldButtonMixin) == 'table' \
                and type(CustomizationNoHeaderTooltipMixin) == 'table'",
        )
        .expect("Customization template-base mixins query should succeed");
    assert!(
        mixins_present,
        "Blizzard_CustomizationTemplates.lua should publish the 8 template-base mixins: \
         CustomizationContentFrameMixin (line 3, the `<Frame>` content base — Set/Reset \
         offsets via SetCustomXOffset/SetCustomYOffset/Reset hooks); CustomizationBaseButton- \
         Mixin (line 15, CreateFromMixins(CustomizationContentFrameMixin) — basic button \
         that overrides `OnLoad` to register the parent ContentFrame state hooks); \
         CustomizationFrameWithTooltipMixin (line 25, CreateFromMixins(RingedFrameWith- \
         TooltipMixin) — adds the `OnLoad`/`OnEnter`/`OnLeave` shared between the option \
         widgets); CustomizationMaskedButtonMixin (line 33, CreateFromMixins(RingedMasked- \
         ButtonMixin)); CustomizationFrameWithExpandableTooltipMixin (line 40, full \
         standalone — manages the tooltip expand/collapse state with the `BAGSLOT_TOOLTIP` \
         hook); CustomizationSmallButtonMixin (line 96, CreateFromMixins(CustomizationFrame- \
         WithTooltipMixin, CustomizationContentFrameMixin)); CustomizationClickOrHoldButton- \
         Mixin (line 122, click-and-hold repeater used by zoom/rotate); and \
         CustomizationNoHeaderTooltipMixin (line 172)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_customization_ui_option_template_mixins_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &customization_ui_toc())
        .expect("Blizzard_CustomizationUI should load via Rust loader");

    let mixins_present: bool = env
        .eval(
            "return type(CustomizationOptionFrameBaseMixin) == 'table' \
                and type(CustomizationOptionFrameBaseMixin.SetupOption) == 'function' \
                and type(CustomizationOptionFrameBaseMixin.SetupAudio) == 'function' \
                and type(CustomizationOptionSliderMixin) == 'table' \
                and type(CustomizationOptionSliderMixin.OnSliderValueChanged) == 'function' \
                and type(CustomizationOptionCheckButtonMixin) == 'table' \
                and type(CustomizationOptionCheckButtonMixin.OnCheckButtonClick) == 'function' \
                and type(CustomizationDropdownWithSteppersAndLabelMixin) == 'table' \
                and type(CustomizationDropdownWithSteppersAndLabelMixin.SetMissingOptionWarningEnabled) == 'function' \
                and type(CustomizationElementDetailsMixin) == 'table' \
                and type(CustomizationElementDetailsMixin.GetTooltipText) == 'function' \
                and type(CustomizationElementDetailsMixin.UpdateFontColors) == 'function' \
                and type(CustomizationDropdownMixin) == 'table' \
                and type(CustomizationElementMixin) == 'table' \
                and type(CustomizationDropdownElementMixin) == 'table'",
        )
        .expect("Customization option-template mixins query should succeed");
    assert!(
        mixins_present,
        "Blizzard_CustomizationOptionTemplates.lua should publish the 8 option-template \
         mixins: CustomizationOptionFrameBaseMixin (line 3, CreateFromMixins(Customization- \
         ContentFrameMixin) with the SetupOption/SetOptionData/GetOptionData/RefreshOption/ \
         GetCurrentChoiceIndex/HasChoice/GetChoice/GetCurrentChoice/HasSound/GetSoundKit/ \
         SetupAudio/ShutdownAudio/GetAudioInterface/GetDebugName accessors shared by all \
         option widgets); CustomizationOptionSliderMixin (line 92, multi-mixin compose: \
         CustomizationOptionFrameBaseMixin + SliderWithButtonsAndLabelMixin + \
         CustomizationFrameWithTooltipMixin); CustomizationOptionCheckButtonMixin (line \
         152); CustomizationDropdownWithSteppersAndLabelMixin (line 196, with the \
         GetOrCreateWarningTexture/GetWarningTexture/SetMissingOptionWarningEnabled \
         missing-option red-arrow logic); CustomizationElementDetailsMixin (line 416, \
         tooltip + font-color helper that drives every dropdown element); \
         CustomizationDropdownMixin (line 657); CustomizationElementMixin (line 684); and \
         CustomizationDropdownElementMixin (line 774)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_customization_ui_audio_interface_mixins_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &customization_ui_toc())
        .expect("Blizzard_CustomizationUI should load via Rust loader");

    let mixins_present: bool = env
        .eval(
            "return type(CustomizationAudioInterfaceMixin) == 'table' \
                and type(CustomizationAudioInterfaceMixin.OnEvent) == 'function' \
                and type(CustomizationAudioInterfaceMixin.SetupAudio) == 'function' \
                and type(CustomizationAudioInterfaceMixin.IsPlaying) == 'function' \
                and type(CustomizationAudioInterfaceMixin.PlayAudioInternal) == 'function' \
                and type(CustomizationAudioInterfaceMixin.PlayAudio) == 'function' \
                and type(CustomizationAudioInterfaceMixin.StopAudio) == 'function' \
                and type(CustomizationAudioInterfaceMixin.OnPlaybackFinished) == 'function' \
                and type(CustomizationAudioInterfaceMixin.CheckResumePlayback) == 'function' \
                and type(CustomizationAudioInterfaceMixin.OnAudioPlayingTick) == 'function' \
                and type(CustomizationAudioInterfacePlayButtonMixin) == 'table' \
                and type(CustomizationAudioInterfacePlayButtonMixin.OnClick) == 'function' \
                and type(CustomizationAudioInterfacePlayButtonMixin.UpdateState) == 'function' \
                and type(CustomizationAudioInterfaceMuteButtonMixin) == 'table' \
                and type(CustomizationAudioInterfaceMuteButtonMixin.OnClick) == 'function' \
                and type(CustomizationAudioInterfaceMuteButtonMixin.UpdateState) == 'function'",
        )
        .expect("Customization audio-interface mixins query should succeed");
    assert!(
        mixins_present,
        "Blizzard_CustomizationAudioInterface.lua should publish 3 mixins: \
         CustomizationAudioInterfaceMixin (line 7, 9 methods — OnEvent dispatching \
         SOUND_KIT_FINISHED/SOUNDKIT_FINISHED to OnPlaybackFinished, SetupAudio caching \
         soundKit, IsPlaying, PlayAudioInternal calling PlaySound, PlayAudio handling \
         pause/resume, StopAudio calling StopSound, OnPlaybackFinished re-firing the \
         CHARACTER_CUSTOMIZATION_AUDIO_FINISHED hook, CheckResumePlayback and \
         OnAudioPlayingTick driving the OnUpdate poll); \
         CustomizationAudioInterfacePlayButtonMixin (line 108, CreateFromMixins(Customization- \
         FrameWithTooltipMixin) — OnLoad/OnClick/GetStateTextures/UpdateState toggling \
         play↔pause atlases); CustomizationAudioInterfaceMuteButtonMixin (line 144, \
         CreateFromMixins(CustomizationFrameWithTooltipMixin) — OnLoad/GetStateTextures/ \
         UpdateState/OnClick/OnPulseAnimPlay/OnPulseAnimLoop driving the mute pulse \
         animation)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_customization_ui_main_frame_mixins_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &customization_ui_toc())
        .expect("Blizzard_CustomizationUI should load via Rust loader");

    let mixins_present: bool = env
        .eval(
            "return type(CustomizationParentFrameBaseMixin) == 'table' \
                and type(CustomizationRandomizeAppearanceButtonMixin) == 'table' \
                and type(CustomizationResetCameraButtonMixin) == 'table' \
                and type(CustomizationZoomButtonMixin) == 'table' \
                and type(CustomizationRotateButtonMixin) == 'table' \
                and type(CustomizationCategoryButtonMixin) == 'table' \
                and type(CustomizationFrameBaseMixin) == 'table'",
        )
        .expect("Customization main-frame mixins query should succeed");
    assert!(
        mixins_present,
        "Blizzard_CustomizationUI.lua should publish 7 main-frame mixins: \
         CustomizationParentFrameBaseMixin (line 3, the abstract parent frame mixin \
         consumed by Blizzard_CharacterCreate's CharCustomizationFrame and the BarbershopUI \
         RootFrame); CustomizationRandomizeAppearanceButtonMixin (line 73); \
         CustomizationResetCameraButtonMixin (line 83); CustomizationZoomButtonMixin (line \
         94, CreateFromMixins(CustomizationClickOrHoldButtonMixin)); \
         CustomizationRotateButtonMixin (line 106, CreateFromMixins(CustomizationClick- \
         OrHoldButtonMixin)); CustomizationCategoryButtonMixin (line 118, CreateFromMixins(\
         CustomizationMaskedButtonMixin, CustomizationContentFrameMixin)); \
         CustomizationFrameBaseMixin (line 185, the per-customization-frame state machine \
         that builds the option/element pools)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_customization_ui_xml_templates_are_registered(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &customization_ui_toc())
        .expect("Blizzard_CustomizationUI should load via Rust loader");

    for template_name in [
        "CustomizationAudioInterfacePlayButtonTemplate",
        "CustomizationAudioInterfaceMuteButtonTemplate",
        "CustomizationAudioInterface",
        "CustomizationBaseButtonTemplate",
        "CustomizationFrameWithTooltipTemplate",
        "CustomizationMaskedButtonTemplate",
        "CustomizationSmallButtonTemplate",
        "CustomizationClickOrHoldButtonTemplate",
        "CustomizationOptionSliderTemplate",
        "CustomizationOptionCheckButtonTemplate",
        "CustomizationElementDetailsTemplate",
        "CustomizationElementTemplate",
        "CustomizationDropdownElementTemplate",
        "CustomizationDropdownWithSteppersAndLabelTemplate",
        "CustomizationCategoryButtonTemplate",
        "CustomizationResetCameraButtonTemplate",
        "CustomizationZoomOutButtonTemplate",
        "CustomizationZoomInButtonTemplate",
        "CustomizationRotateLeftButtonTemplate",
        "CustomizationRotateRightButtonTemplate",
        "CustomizationRandomizeAppearanceButtonTemplate",
        "CustomizationFrameBaseTemplate",
    ] {
        assert!(
            wow_ui_sim::xml::get_template(template_name).is_some(),
            "{template_name} (`<Frame|Button|CheckButton virtual=\"true\">` from one of the 4 \
             Customization XML files) should be registered in the Frame template registry \
             after Blizzard_CustomizationUI loads — there are 23 virtual templates in this \
             addon (22 Frame/Button/CheckButton + 1 `<Texture>` named \
             CustomizationMissingOptionWarningTemplate which lives in the separate texture \
             template registry) and all 22 Frame-style ones must be discoverable for \
             Blizzard_CharacterCreate / Blizzard_BarbershopUI to inherit them"
        );
    }
}
}
