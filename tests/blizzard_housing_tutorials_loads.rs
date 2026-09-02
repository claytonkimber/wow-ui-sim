#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn tutorials_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_HousingTutorials")
}

fn tutorials_toc() -> PathBuf {
    tutorials_dir().join("Blizzard_HousingTutorials.toc")
}

const ALL_MIXINS: &[&str] = &[
    "HouseDecorQuestWatcherMixin",
    "HouseDecorQuestTutorialMixin",
    "HouseDecorWatcherMixin",
    "HouseClippingAndGridTutorialMixin",
    "HouseModesUnlockedTutorialMixin",
    "HouseExpertModeTutorialMixin",
    "HouseCleanupModeTutorialMixin",
    "HouseMarketTabTutorialMixin",
    "HouseDecorCustomizationsTutorialMixin",
    "HouseLayoutTutorialMixin",
    "HousingTutorialsNewPipMixin",
    "HousingTutorialsHouseTeleportWatcherMixin",
    "HousingTutorialsHouseTeleportMixin",
    "HouseFinderWatcherMixin",
    "HouseFinderMapTutorialMixin",
];

const ALL_DATA_GLOBALS: &[&str] = &[
    "HousingTutorialsQuestManager",
    "HousingTutorialQuestIDs",
    "HousingTutorialStates",
    "HousingTutorialHelpTipSystems",
    "HousingTutorialData",
    "HOUSING_TUTORIALS_HOUSE_TELEPORT_EVENTS",
    "HousingTutorialUtil",
];

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
fn blizzard_housing_tutorials_find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&tutorials_dir()).expect("Blizzard_HousingTutorials TOC should resolve");
    assert_eq!(
        resolved,
        tutorials_toc(),
        "Blizzard_HousingTutorials ships exactly one bare TOC — retail-only addon resolves via \
         `find_toc_file` fallthrough"
    );
}

#[test]
fn blizzard_housing_tutorials_toc_declares_auto_loaded_with_one_dependency() {
    let toc = TocFile::from_file(&tutorials_toc()).expect("HousingTutorials TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_HousingTutorials omits `## LoadOnDemand:` so it auto-loads via the Game-screen \
         discovery sweep — it must be present from the moment the player enters the game world \
         so its event watchers (HouseFinderWatcherMixin / HouseDecorWatcherMixin / \
         HousingTutorialsHouseTeleportWatcherMixin) can install themselves before the corresponding \
         tutorial trigger events fire"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_Tutorials".to_string()],
        "Single `## Dependencies:` entry: Blizzard_Tutorials (provides the \
         HelpTipStateMachineBasedTutorialMixin parent class that 5 of this addon's mixins extend \
         via CreateFromMixins, plus the TutorialQuestManager class that HousingTutorialsQuestManager \
         extends, plus the BagTutorialBaseMixin used by the local HousingTutorialsItemAcquisitionMixin \
         in Misc.lua)"
    );
}

#[test]
fn blizzard_housing_tutorials_toc_is_retail_only_and_game_only() {
    let toc = TocFile::from_file(&tutorials_toc()).expect("HousingTutorials TOC should parse");
    let toc_text = std::fs::read_to_string(tutorials_toc()).expect("TOC should read");
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Declares `## AllowLoadGameType: standard` — retail-only Midnight feature"
    );
    assert!(!toc.is_game_type_restricted());
    assert!(
        toc_text.contains("## AllowLoad: Game"),
        "Declares `## AllowLoad: Game` — tutorials only run inside the actual game session, \
         never on Login / CharacterSelect / CharacterCreate (no housing UI exists outside the \
         game world)"
    );
    assert!(!toc_text.contains("## DefaultState:"));
    assert!(
        toc.saved_variables().is_empty(),
        "No `## SavedVariables*` — all tutorial completion state lives in CVar bitfields \
         (HOUSING_TUTORIAL_CVAR_BITFIELD with Enum.FrameTutorialAccount.Housing* bits) plus \
         C_QuestLog completion flags; no client-side persistence of its own"
    );
}

#[test]
fn blizzard_housing_tutorials_toc_lists_six_files_in_order() {
    let toc = TocFile::from_file(&tutorials_toc()).expect("HousingTutorials TOC should parse");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        files,
        vec![
            "Blizzard_HousingTutorialsData.lua".to_string(),
            "Blizzard_HousingTutorialsUtil.lua".to_string(),
            "Blizzard_HousingTutorialsDecor.lua".to_string(),
            "Blizzard_HousingTutorialsHouseFinder.lua".to_string(),
            "Blizzard_HousingTutorialsMisc.lua".to_string(),
            "Blizzard_HousingTutorialsInit.lua".to_string(),
        ],
        "TOC body lists exactly 6 source files in this exact order — Data loads FIRST so its 5 \
         lookup tables (HousingTutorialQuestIDs / HousingTutorialStates / \
         HousingTutorialHelpTipSystems / HousingTutorialData / HousingTutorialsQuestManager) \
         publish before any consuming mixin runs, Util SECOND for the HousingTutorialUtil helper \
         namespace, then Decor / HouseFinder / Misc declare the mixin tables consumed by Init, \
         and Init runs LAST as the entry-point — its file-scope `HousingTutorialManager:Init()` \
         tail call calls UpdateHousingTutorials() and registers the SETTINGS_LOADED EventRegistry \
         callback so the tutorial chain bootstraps on addon load"
    );
}

#[test]
fn blizzard_housing_tutorials_directory_holds_seven_entries() {
    let dir = tutorials_dir();
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_HousingTutorials directory should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        7,
        "Directory ships exactly 7 entries (6 source + 1 TOC, no flavor subdirectory, no XML \
         files — this addon is pure Lua: it instruments existing housing UI frames via mixin \
         watcher observers and HelpTip popovers, never declares its own widgets). Got: {entries:?}"
    );
    assert!(entries.contains(&"Blizzard_HousingTutorials.toc".to_string()));
}

#[test]
fn blizzard_housing_tutorials_appears_in_game_screen_auto_discovery_only() {
    let ui = blizzard_ui_dir();
    let game_addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_HousingTutorials");
    assert!(
        in_game,
        "Blizzard_HousingTutorials MUST appear in the Game ScreenKind auto-discovery pass — \
         omitting `## LoadOnDemand:` plus `## AllowLoad: Game` makes it eligible exclusively for \
         the Game-screen sweep"
    );
    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&ui, screen);
        let discovered = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_HousingTutorials");
        assert!(
            !discovered,
            "Blizzard_HousingTutorials must NOT appear in the {screen:?} ScreenKind \
             auto-discovery pass — `## AllowLoad: Game` excludes it from every non-Game screen"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_housing_tutorials_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let lua_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let related: Vec<&String> = lua_errors
        .iter()
        .filter(|e| {
            e.contains("Blizzard_HousingTutorials/")
                || e.contains("Blizzard_HousingTutorials\\")
                || ALL_MIXINS.iter().any(|m| e.contains(m))
                || e.contains("HousingTutorialUtil")
                || e.contains("HousingTutorialManager")
        })
        .collect();
    assert!(
        related.is_empty(),
        "Loading Blizzard_HousingTutorials must not emit any addon-specific Lua errors. Got {} \
         errors: {:?}",
        related.len(),
        related
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_tutorials_is_addon_loaded_via_game_screen_pass(env: &WowLuaEnv) {
    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingTutorials')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_HousingTutorials') must return true after the \
         Game-screen auto-discovery pass — proves the loader registered the addon name without \
         any explicit LoD call"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_tutorials_dependency_loads_via_game_screen_pass(env: &WowLuaEnv) {
    let dep_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_Tutorials')")
        .expect("Blizzard_Tutorials IsAddOnLoaded query should succeed");
    assert!(
        dep_loaded,
        "Blizzard_Tutorials (the only declared dep) must auto-load via the Game-screen \
         discovery pass before HousingTutorials runs — provides the \
         HelpTipStateMachineBasedTutorialMixin / TutorialQuestManager / BagTutorialBaseMixin \
         classes the housing tutorials inherit"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_tutorials_publishes_all_fifteen_mixins(env: &WowLuaEnv) {
    for mixin in ALL_MIXINS {
        let mixin_type: String = env
            .eval(&format!("return type(_G['{mixin}'])"))
            .expect("mixin _G lookup should succeed");
        assert_eq!(
            mixin_type, "table",
            "{mixin} must publish as a `_G` table — every mixin is declared at file scope across \
             the 3 mixin-bearing Lua files (Decor 10 mixins / Misc 3 publicly-named mixins / \
             HouseFinder 2 mixins) so Init.lua's UpdateHousingTutorials can CreateFromMixins each \
             one into the activeTutorials registry when its CVar gate opens"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_tutorials_decor_quest_tutorial_inherits_help_tip_state_machine(env: &WowLuaEnv) {
    let parent_type: String = env
        .eval("return type(_G['HelpTipStateMachineBasedTutorialMixin'])")
        .expect("parent mixin lookup should succeed");
    assert_eq!(
        parent_type, "table",
        "HelpTipStateMachineBasedTutorialMixin must publish as a `_G` table from the \
         Blizzard_Tutorials dep — required as the parent class for HouseDecorQuestTutorialMixin / \
         HouseDecorCustomizationsTutorialMixin / HouseLayoutTutorialMixin / \
         HousingTutorialsHouseTeleportMixin / HouseFinderMapTutorialMixin's CreateFromMixins calls"
    );
    for derived in [
        "HouseDecorQuestTutorialMixin",
        "HouseDecorCustomizationsTutorialMixin",
        "HouseLayoutTutorialMixin",
        "HousingTutorialsHouseTeleportMixin",
        "HouseFinderMapTutorialMixin",
    ] {
        let derived_type: String = env
            .eval(&format!("return type(_G['{derived}'])"))
            .expect("derived tutorial mixin lookup should succeed");
        assert_eq!(
            derived_type, "table",
            "{derived} = CreateFromMixins(HelpTipStateMachineBasedTutorialMixin) must publish as \
             a `_G` table — proves the CreateFromMixins copy-on-init runs at file load time and \
             the parent class is resolved from the Blizzard_Tutorials dep"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_tutorials_publishes_all_seven_data_globals(env: &WowLuaEnv) {
    for global in ALL_DATA_GLOBALS {
        let value_type: String = env
            .eval(&format!("return type(_G['{global}'])"))
            .expect("data global lookup should succeed");
        assert_eq!(
            value_type, "table",
            "{global} must publish as a `_G` table — declared at file scope in either Data.lua \
             (loads FIRST: HousingTutorialsQuestManager / HousingTutorialQuestIDs / \
             HousingTutorialStates / HousingTutorialHelpTipSystems / HousingTutorialData), \
             Util.lua (HousingTutorialUtil), or Misc.lua \
             (HOUSING_TUTORIALS_HOUSE_TELEPORT_EVENTS); the data globals are queried by every \
             mixin to drive CVar bitfield lookups, quest ID matching, and HelpTip system \
             registration"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_tutorials_publishes_can_show_house_finder_tutorial_function(env: &WowLuaEnv) {
    let value_type: String = env
        .eval("return type(_G['CanShowHouseFinderTutorial'])")
        .expect("CanShowHouseFinderTutorial lookup should succeed");
    assert_eq!(
        value_type, "function",
        "CanShowHouseFinderTutorial must publish as a `_G` function — Init.lua line 12 declares \
         it WITHOUT `local` keyword (unlike the file-local CanShowHouseDecorQuestTutorial / \
         CanShowHouseDecorTutorials helpers), so it publishes globally so external housing UI \
         code can gate visual hints on it; checks both Enum.FrameTutorialAccount.HousingHouseFinderMap \
         and Enum.FrameTutorialAccount.HousingHouseFinderVisitHouse CVar bitfield bits"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_tutorials_publishes_update_housing_tutorials_function(env: &WowLuaEnv) {
    let value_type: String = env
        .eval("return type(_G['UpdateHousingTutorials'])")
        .expect("UpdateHousingTutorials lookup should succeed");
    assert_eq!(
        value_type, "function",
        "UpdateHousingTutorials must publish as a `_G` function — Init.lua line 18 declares it \
         WITHOUT `local` keyword so external code (and the file-scope HousingTutorialManager:Init() \
         tail call) can re-evaluate which tutorial watchers should be active; gated on \
         `housingTutorialsEnabled` CVarBool and selectively CreateFromMixins-attaches \
         HouseFinderWatcherMixin / HouseDecorQuestWatcherMixin / HouseDecorWatcherMixin into the \
         file-local activeTutorials table"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_tutorials_quest_manager_inherits_tutorial_quest_manager(env: &WowLuaEnv) {
    let parent_type: String = env
        .eval("return type(_G['TutorialQuestManager'])")
        .expect("parent class lookup should succeed");
    assert_eq!(
        parent_type, "table",
        "TutorialQuestManager must publish as a `_G` table from the Blizzard_Tutorials dep — \
         required as the parent class for HousingTutorialsQuestManager's CreateFromMixins call \
         on Data.lua line 2; HousingTutorialsQuestManager:ReinitializeExistingQuests is invoked \
         on the SETTINGS_LOADED EventRegistry callback registered by HousingTutorialManager:Init"
    );
}
}
