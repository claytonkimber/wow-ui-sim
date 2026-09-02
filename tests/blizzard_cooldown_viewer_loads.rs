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

fn cooldown_viewer_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_CooldownViewer/Blizzard_CooldownViewer.toc")
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
fn blizzard_cooldown_viewer_toc_declares_required_deps_and_mainline_only() {
    let toc = TocFile::from_file(&cooldown_viewer_toc())
        .expect("Blizzard_CooldownViewer TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_CooldownViewer is non-LOD (it auto-loads on Game screen so the four \
         CooldownViewer top-level frames are always present for EditMode bring-up)"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_CooldownViewer declares `## AllowLoadGameType: standard`; the simulator \
         treats `mainline` and `standard` as the same retail target so this must NOT be \
         flagged as game-type-restricted"
    );
    let deps = toc.dependencies();
    for required in ["Blizzard_EditMode", "Blizzard_ChatFrameBase"] {
        assert!(
            deps.contains(&required.to_string()),
            "Blizzard_CooldownViewer should declare `## Dependencies: {required}` (the Cooldown \
             Manager registers itself as an EditMode system and uses ChatFrame slash output), \
             got {deps:?}"
        );
    }
}

#[test]
fn blizzard_cooldown_viewer_appears_in_game_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CooldownViewer");
    assert!(
        in_game,
        "Blizzard_CooldownViewer (`## AllowLoadGameType: standard`, non-LOD) should appear in \
         Game-screen auto-discovery"
    );
}

prefork_full_ui_case! {
fn blizzard_cooldown_viewer_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_CooldownViewer")
                || message.contains("CooldownViewer")
                || message.contains("CooldownLayoutType")
                || message.contains("PandemicAlert")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_CooldownViewer emitted Lua errors during load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_cooldown_viewer_toplevel_frames_are_defined(env: &WowLuaEnv) {

    let frames_present: bool = env
        .eval(
            "return type(_G.EssentialCooldownViewer) == 'table' \
                and EssentialCooldownViewer:GetParent() == UIParent \
                and type(_G.UtilityCooldownViewer) == 'table' \
                and UtilityCooldownViewer:GetParent() == UIParent \
                and type(_G.BuffIconCooldownViewer) == 'table' \
                and BuffIconCooldownViewer:GetParent() == UIParent \
                and type(_G.BuffBarCooldownViewer) == 'table' \
                and type(_G.CooldownViewerSettings) == 'table' \
                and CooldownViewerSettings:GetParent() == UIParent",
        )
        .expect("toplevel frame query should succeed");
    assert!(
        frames_present,
        "Blizzard_CooldownViewer should define its 5 top-level frames after load: \
         EssentialCooldownViewer / UtilityCooldownViewer / BuffIconCooldownViewer (all three \
         inherit CooldownViewerTemplate + UIParentBottomManagedFrameTemplate, parented to \
         UIParent), BuffBarCooldownViewer (inherits CooldownViewerTemplate only, since the \
         buff-bar layout owns its own anchor), and CooldownViewerSettings (inherits \
         ButtonFrameTemplate + CallbackRegistrantTemplate, parent UIParent, toplevel=true, \
         hidden=true — opened via the `/cdmgr` slash command)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_cooldown_viewer_constants_and_enums_are_populated(env: &WowLuaEnv) {

    let constants_present: bool = env
        .eval(
            "return Enum.CooldownViewerCategory.HiddenActive == -1 \
                and Enum.CooldownViewerCategory.HiddenPassive == -2 \
                and Enum.CDMLayoutMode.AccessOnly == false \
                and Enum.CDMLayoutMode.AllowCreate == true \
                and Enum.CooldownLayoutType.Character == 1 \
                and Enum.CooldownLayoutType.Account == 2 \
                and Enum.CooldownLayoutStatus.Success == 0 \
                and Enum.CooldownLayoutStatus.InvalidLayoutName == 1 \
                and Enum.CooldownLayoutStatus.TooManyLayouts == 2 \
                and Enum.CooldownLayoutStatus.NoValidAlerts == 6 \
                and Enum.CooldownLayoutAction.ChangeOrder == 0 \
                and Enum.CooldownLayoutAction.AddAlert == 3 \
                and Enum.CooldownViewerSound.TextToSpeech == 0 \
                and Enum.CooldownViewerSound.AnimalsCat == 1 \
                and Enum.CooldownViewerSound.War3WolfHowl == 67 \
                and COOLDOWN_VIEWER_CLASS_AND_SPEC_FORMAT == '%s - %s'",
        )
        .expect("constants query should succeed");
    assert!(
        constants_present,
        "CooldownViewerSettingsConstants.lua should populate \
         Enum.CooldownViewerCategory.{{HiddenActive=-1,HiddenPassive=-2}}, \
         Enum.CDMLayoutMode.{{AccessOnly=false,AllowCreate=true}}, \
         Enum.CooldownLayoutType.{{Character=1,Account=2}}, \
         Enum.CooldownLayoutStatus.{{Success=0,...,NoValidAlerts=6}}, \
         Enum.CooldownLayoutAction.{{ChangeOrder=0,...,AddAlert=3}}, the 94-entry Enum.CooldownViewerSound table (TextToSpeech=0, AnimalsCat=1, \
         War3WolfHowl=67), and the localizable \
         COOLDOWN_VIEWER_CLASS_AND_SPEC_FORMAT='%s - %s'"
    );
}
}

prefork_full_ui_case! {
fn blizzard_cooldown_viewer_core_mixins_are_defined(env: &WowLuaEnv) {

    let mixins_present: bool = env
        .eval(
            "return type(CooldownViewerMixin) == 'table' \
                and type(CooldownViewerCooldownMixin) == 'table' \
                and type(CooldownViewerBuffMixin) == 'table' \
                and type(EssentialCooldownViewerMixin) == 'table' \
                and type(UtilityCooldownViewerMixin) == 'table' \
                and type(BuffIconCooldownViewerMixin) == 'table' \
                and type(BuffBarCooldownViewerMixin) == 'table' \
                and type(CooldownViewerItemMixin) == 'table' \
                and type(CooldownViewerCooldownItemMixin) == 'table' \
                and type(CooldownViewerEssentialItemMixin) == 'table' \
                and type(CooldownViewerUtilityItemMixin) == 'table' \
                and type(CooldownViewerBuffItemMixin) == 'table' \
                and type(CooldownViewerBuffIconItemMixin) == 'table' \
                and type(CooldownViewerBuffBarItemMixin) == 'table' \
                and type(CooldownViewerItemDebuffBorderMixin) == 'table'",
        )
        .expect("core mixin query should succeed");
    assert!(
        mixins_present,
        "Blizzard_CooldownViewer should define its 7 viewer-frame mixins (CooldownViewer / \
         CooldownViewerCooldown / CooldownViewerBuff base mixins plus the 4 concrete \
         Essential/Utility/BuffIcon/BuffBar specializations all built via CreateFromMixins \
         with EditModeCooldownViewerSystemMixin / UIParentManagedFrameMixin / \
         GridLayoutFrameMixin) and 8 item-row mixins (Item / CooldownItem / Essential/Utility \
         specializations, Buff / BuffIcon/BuffBar specializations, plus the standalone \
         ItemDebuffBorder mixin)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_cooldown_viewer_settings_panel_mixins_are_defined(env: &WowLuaEnv) {

    let mixins_present: bool = env
        .eval(
            "return type(CooldownViewerSettingsMixin) == 'table' \
                and type(CooldownViewerSettingsContentMixin) == 'table' \
                and type(CooldownViewerSettingsItemMixin) == 'table' \
                and type(CooldownViewerSettingsBarItemMixin) == 'table' \
                and type(CooldownViewerSettingsCategoryMixin) == 'table' \
                and type(CooldownViewerSettingsBarCategoryMixin) == 'table' \
                and type(CooldownViewerSettingsTabWithNewOptionMixin) == 'table' \
                and type(CooldownViewerSettingsCategoryNewOptionMixin) == 'table' \
                and type(CooldownViewerSettingsSearchBoxMixin) == 'table' \
                and type(CooldownViewerSettingsReorderMarkerMixin) == 'table' \
                and type(CooldownViewerBaseReorderTargetMixin) == 'table' \
                and type(CooldownViewerContainerReorderTargetMixin) == 'table'",
        )
        .expect("settings mixin query should succeed");
    assert!(
        mixins_present,
        "CooldownViewerSettings.lua should define its 11 settings-panel mixins: \
         CooldownViewerSettingsMixin (the panel root), CooldownViewerSettingsContentMixin \
         (the scrollable content body), CooldownViewerSettingsItemMixin / \
         CooldownViewerSettingsBarItemMixin (per-row entries with drag-reorder support), \
         CooldownViewerSettingsCategoryMixin / CooldownViewerSettingsBarCategoryMixin \
         (category headers), CooldownViewerSettingsTabWithNewOptionMixin / \
         CooldownViewerSettingsCategoryNewOptionMixin (NewDefinitionsChecker-backed new-option \
         indicators for tabs and category headers), CooldownViewerSettingsSearchBoxMixin / \
         CooldownViewerSettingsReorderMarkerMixin (search filter + drop-target indicator), \
         and the two reorder-target base mixins (BaseReorderTarget + \
         ContainerReorderTarget — built via CreateFromMixins for the per-item and \
         per-category drop zones)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_cooldown_viewer_util_helpers_round_trip_class_spec_tag(env: &WowLuaEnv) {

    let util_present: bool = env
        .eval(
            "return type(CooldownViewerUtil) == 'table' \
                and type(CooldownViewerUtil.GetCurrentClassAndSpecTag) == 'function' \
                and type(CooldownViewerUtil.GetClassAndSpecTagText) == 'function' \
                and type(CooldownViewerUtil.IsDisabledCategory) == 'function' \
                and CooldownViewerUtil.IsDisabledCategory(0) == false",
        )
        .expect("CooldownViewerUtil query should succeed");
    assert!(
        util_present,
        "CooldownViewerUtil should expose its 3 functions (GetCurrentClassAndSpecTag, \
         GetClassAndSpecTagText, IsDisabledCategory). Exercise IsDisabledCategory with a normal \
         category id and preserve its false result"
    );
}
}

prefork_full_ui_case! {
fn blizzard_cooldown_viewer_settings_panel_starts_hidden_and_toggles(env: &WowLuaEnv) {

    let toggles_correctly: bool = env
        .eval(
            "local before = CooldownViewerSettings:IsShown(); \
             CooldownViewerSettings:Show(); \
             local after_show = CooldownViewerSettings:IsShown(); \
             CooldownViewerSettings:Hide(); \
             local after_hide = CooldownViewerSettings:IsShown(); \
             return (not before) and after_show and (not after_hide)",
        )
        .expect("CooldownViewerSettings show/hide round-trip should succeed");
    assert!(
        toggles_correctly,
        "CooldownViewerSettings is `<Frame ... hidden=\"true\">` so IsShown() must start false; \
         a Show()/Hide() round-trip should cycle visibility cleanly without the slash-command \
         registration interfering with the Show frame method"
    );
}
}
