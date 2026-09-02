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

fn deprecated_arena_ui_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_Deprecated_ArenaUI/Blizzard_Deprecated_ArenaUI.toc")
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
fn blizzard_deprecated_arena_ui_toc_is_non_lod_with_unitframe_and_durabilityframe_deps() {
    let toc = TocFile::from_file(&deprecated_arena_ui_toc())
        .expect("Blizzard_Deprecated_ArenaUI TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_Deprecated_ArenaUI is non-LOD — the legacy ArenaEnemyFramesContainer must \
         exist on Game-screen bring-up so the battleground flag-carrier UI can attach to \
         UIParent before any combat events fire"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_Deprecated_ArenaUI does not declare UseSecureEnvironment"
    );

    let deps = toc.dependencies();
    assert!(
        deps.iter().any(|d| d == "Blizzard_UnitFrame"),
        "Blizzard_Deprecated_ArenaUI declares Blizzard_UnitFrame as a dependency — the arena \
         enemy frames call UnitFrame_Initialize / UnitFrame_Update / UnitFrame_OnEnter / \
         UnitFrame_OnEvent / UnitFrameHealPredictionBars_Update from that addon. Got {deps:?}"
    );
    assert!(
        deps.iter().any(|d| d == "Blizzard_DurabilityFrame"),
        "Blizzard_Deprecated_ArenaUI declares Blizzard_DurabilityFrame as a dependency — \
         ArenaEnemyMatchFramesContainerMixin.OnShow / OnHide and \
         ArenaEnemyPrepFramesContainerMixin.OnShow / OnHide call \
         DurabilityFrame:SetAlerts() (Deprecated_ArenaUI.lua:97, 102, 462, 467). Got {deps:?}"
    );

    let toc_text = std::fs::read_to_string(deprecated_arena_ui_toc())
        .expect("Blizzard_Deprecated_ArenaUI TOC should read");
    assert!(
        toc_text.contains("## AllowLoad: Game"),
        "Blizzard_Deprecated_ArenaUI declares `## AllowLoad: Game` — the legacy arena UI is \
         in-game-only (battleground flag-carrier display has no glue-screen counterpart)"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_Deprecated_ArenaUI omits `## AllowLoadGameType:` so the legacy frames load \
         on every game type without restriction (battlegrounds exist on both mainline and \
         classic flavors)"
    );
}

#[test]
fn blizzard_deprecated_arena_ui_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_Deprecated_ArenaUI");
    assert!(
        in_game,
        "Blizzard_Deprecated_ArenaUI (`## AllowLoad: Game`) should appear in Game-screen \
         auto-discovery so ArenaEnemyFramesContainer is ready before the first \
         ARENA_OPPONENT_UPDATE event"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_Deprecated_ArenaUI");
    assert!(
        !in_login,
        "Blizzard_Deprecated_ArenaUI should NOT appear on the Login / glue screens — arena \
         enemy frames are an in-game concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_arena_ui_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Deprecated_ArenaUI")
                || message.contains("Blizzard_Deprecated_ArenaUI")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_Deprecated_ArenaUI emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_arena_ui_publishes_max_arena_enemies_global(env: &WowLuaEnv) {

    let value: i32 = env
        .eval("return MAX_ARENA_ENEMIES")
        .expect("MAX_ARENA_ENEMIES query should succeed");
    assert_eq!(
        value, 5,
        "Deprecated_ArenaUI.lua:6 sets `MAX_ARENA_ENEMIES = 5` — the legacy arena UI fixed \
         five-opponent cap that gates every `for i = 1, MAX_ARENA_ENEMIES` loop in the \
         match/prep containers and the `GetBestAnchorUnitFrameForOppponent` clamp"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_arena_ui_creates_top_level_container_frame(env: &WowLuaEnv) {

    let frame_present: bool = env
        .eval(
            "return ArenaEnemyFramesContainer ~= nil \
                and ArenaEnemyFramesContainer:IsObjectType('Frame') \
                and ArenaEnemyPrepFramesContainer ~= nil \
                and not ArenaEnemyPrepFramesContainer:IsShown() \
                and ArenaEnemyMatchFramesContainer ~= nil \
                and not ArenaEnemyMatchFramesContainer:IsShown()",
        )
        .expect("ArenaEnemyFramesContainer query should succeed");
    assert!(
        frame_present,
        "Deprecated_ArenaUI.xml:360 should create ArenaEnemyFramesContainer (toplevel Frame, \
         inherits ResizeLayoutFrame + UIParentRightManagedFrameTemplate, mixin \
         ArenaEnemyFramesContainerMixin) holding two hidden child Frames: \
         ArenaEnemyPrepFramesContainer (xml:366) and ArenaEnemyMatchFramesContainer (xml:408). \
         Both child containers must be hidden by default — they only show during arena prep \
         (ARENA_PREP_OPPONENT_SPECIALIZATIONS) or active arena (PLAYER_ENTERING_WORLD with \
         instanceType == 'pvp')"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_arena_ui_creates_five_match_and_prep_unit_frames(env: &WowLuaEnv) {

    let unit_frames_present: bool = env
        .eval(
            "return ArenaEnemyMatchFrame1 ~= nil and ArenaEnemyMatchFrame5 ~= nil \
                and ArenaEnemyPrepFrame1 ~= nil and ArenaEnemyPrepFrame5 ~= nil \
                and ArenaEnemyMatchFramesContainer.UnitFrames ~= nil \
                and #ArenaEnemyMatchFramesContainer.UnitFrames == 5 \
                and ArenaEnemyPrepFramesContainer.UnitFrames ~= nil \
                and #ArenaEnemyPrepFramesContainer.UnitFrames == 5",
        )
        .expect("ArenaEnemyMatchFrame / ArenaEnemyPrepFrame query should succeed");
    assert!(
        unit_frames_present,
        "Deprecated_ArenaUI.xml:416-440 / 374-398 should create 5 ArenaEnemyMatchFrame# \
         (inherits ArenaEnemyMatchFrameTemplate) and 5 ArenaEnemyPrepFrame# (inherits \
         ArenaEnemyPrepFrameTemplate) buttons under their respective containers, each with \
         `parentArray=\"UnitFrames\"` so the per-container UnitFrames table is populated for \
         the `for index, unitFrame in ipairs(self.UnitFrames)` iteration in \
         ResetCrowdControlCooldownData / UpdateFrames / GetBestAnchorUnitFrameForOppponent"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_arena_ui_registers_virtual_templates(env: &WowLuaEnv) {
    let _env = env;

    let templates = [
        "DeprecatedArenaBarSegmentTemplate",
        "ArenaCastingBarFrameTemplate",
        "ArenaEnemyPetFrameTemplate",
        "ArenaEnemyPrepFrameTemplate",
        "ArenaEnemyMatchFrameTemplate",
    ];
    for template in templates {
        assert!(
            wow_ui_sim::xml::get_template(template).is_some(),
            "{template} (Deprecated_ArenaUI.xml virtual=\"true\") should be registered with \
             the XML template registry — the 5 virtual templates back the 5 \
             ArenaEnemyPrepFrame# / ArenaEnemyMatchFrame# instances and the per-frame \
             casting-bar / pet-frame children"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_arena_ui_publishes_six_mixins_with_lifecycle_methods(env: &WowLuaEnv) {

    let mixins_present: bool = env
        .eval(
            "return type(ArenaEnemyFramesContainerMixin) == 'table' \
                and type(ArenaEnemyFramesContainerMixin.Update) == 'function' \
                and type(ArenaEnemyFramesContainerMixin.UpdateShownState) == 'function' \
                and type(ArenaEnemyMatchFramesContainerMixin) == 'table' \
                and type(ArenaEnemyMatchFramesContainerMixin.OnLoad) == 'function' \
                and type(ArenaEnemyMatchFramesContainerMixin.OnEvent) == 'function' \
                and type(ArenaEnemyMatchFramesContainerMixin.CheckEffectiveEnableState) == 'function' \
                and type(ArenaEnemyMatchFramesContainerMixin.ResetCrowdControlCooldownData) == 'function' \
                and type(ArenaEnemyMatchFrameMixin) == 'table' \
                and type(ArenaEnemyMatchFrameMixin.OnLoad) == 'function' \
                and type(ArenaEnemyMatchFrameMixin.UpdatePlayer) == 'function' \
                and type(ArenaEnemyMatchFrameMixin.SetMysteryPlayer) == 'function' \
                and type(ArenaEnemyMatchFrameMixin.UpdateCrowdControl) == 'function' \
                and type(ArenaEnemyMatchFrameMixin.UpdatePet) == 'function' \
                and type(ArenaEnemyPrepFrameMixin) == 'table' \
                and type(ArenaEnemyPrepFrameMixin.OnShow) == 'function' \
                and type(ArenaEnemyPrepFrameMixin.OnHide) == 'function' \
                and type(ArenaEnemyPetFrameMixin) == 'table' \
                and type(ArenaEnemyPetFrameMixin.OnLoad) == 'function' \
                and type(ArenaEnemyPetFrameMixin.OnEvent) == 'function' \
                and type(ArenaEnemyPetFrameMixin.Update) == 'function' \
                and type(ArenaEnemyPrepFramesContainerMixin) == 'table' \
                and type(ArenaEnemyPrepFramesContainerMixin.OnLoad) == 'function' \
                and type(ArenaEnemyPrepFramesContainerMixin.OnEvent) == 'function' \
                and type(ArenaEnemyPrepFramesContainerMixin.UpdateFrames) == 'function' \
                and type(ArenaEnemyPrepFramesContainerMixin.GetBestAnchorUnitFrameForOppponent) == 'function'",
        )
        .expect("Mixin presence query should succeed");
    assert!(
        mixins_present,
        "Deprecated_ArenaUI.lua should publish 6 mixins with lifecycle + behavior methods: \
         ArenaEnemyFramesContainerMixin (Update / UpdateShownState — line 26), \
         ArenaEnemyMatchFramesContainerMixin (OnLoad / OnEvent / CheckEffectiveEnableState / \
         ResetCrowdControlCooldownData — line 52), ArenaEnemyMatchFrameMixin (OnLoad / \
         UpdatePlayer / SetMysteryPlayer / UpdateCrowdControl / UpdatePet — line 141), \
         ArenaEnemyPrepFrameMixin (OnShow / OnHide — line 352), ArenaEnemyPetFrameMixin \
         (OnLoad / OnEvent / Update — line 362), ArenaEnemyPrepFramesContainerMixin (OnLoad / \
         OnEvent / UpdateFrames / GetBestAnchorUnitFrameForOppponent — line 444)"
    );
}
}

#[test]
fn blizzard_deprecated_arena_ui_required_events_are_registerable() {
    for event in [
        "ARENA_OPPONENT_UPDATE",
        "ARENA_PREP_OPPONENT_SPECIALIZATIONS",
        "ARENA_COOLDOWNS_UPDATE",
        "ARENA_CROWD_CONTROL_SPELL_UPDATE",
        "UNIT_PET",
        "UNIT_NAME_UPDATE",
        "UNIT_CLASSIFICATION_CHANGED",
        "PLAYER_ENTERING_WORLD",
        "VARIABLES_LOADED",
        "CVAR_UPDATE",
    ] {
        assert!(
            wow_ui_sim::event::is_registerable_event(event),
            "{event} should be a registerable event — Deprecated_ArenaUI.lua's \
             ArenaEnemyMatchFramesContainerMixin / ArenaEnemyMatchFrameMixin / \
             ArenaEnemyPetFrameMixin / ArenaEnemyPrepFramesContainerMixin OnLoad handlers \
             register it via self:RegisterEvent(...)"
        );
    }
}
