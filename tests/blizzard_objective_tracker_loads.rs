#![cfg(feature = "client-retail")]
use std::path::PathBuf;


use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::paths::default_blizzard_ui_addons_path;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> std::path::PathBuf {
    default_blizzard_ui_addons_path().expect("Blizzard UI cache should be synced")

}

fn objective_tracker_dir() -> std::path::PathBuf {
    blizzard_ui_dir().join("Blizzard_ObjectiveTracker")
}

fn objective_tracker_toc() -> std::path::PathBuf {
    objective_tracker_dir().join("Blizzard_ObjectiveTracker.toc")
}

const TOC_FILE_COUNT: usize = 40;

const REQUIRED_DEPS: &[&str] = &[
    "Blizzard_MawBuffs",
    "Blizzard_TieredEntranceTraits",
    "Blizzard_TransmogShared",
    "Blizzard_ManagedFrameSystem",
    "Blizzard_UIWidgets",
];

const ANNOTATED_MANAGER_FILE: &str = "Blizzard_ObjectiveTrackerManager.lua";

const PUBLIC_BASE_MIXINS: &[&str] = &[
    "ObjectiveTrackerFrameMixin",
    "ObjectiveTrackerContainerMixin",
    "ObjectiveTrackerModuleMixin",
    "ObjectiveTrackerBlockMixin",
    "ObjectiveTrackerLineMixin",
    "ObjectiveTrackerSlidingMixin",
    "ObjectiveTrackerAnimBlockMixin",
    "ObjectiveTrackerAnimLineMixin",
    "ObjectiveTrackerQuestPOIBlockMixin",
    "ObjectiveTrackerProgressBarMixin",
];

const PUBLIC_MODULE_MIXINS: &[&str] = &[
    "AchievementObjectiveTrackerMixin",
    "AdventureObjectiveTrackerMixin",
    "BonusObjectiveTrackerMixin",
    "CampaignQuestObjectiveTrackerMixin",
    "InitiativeTasksObjectiveTrackerMixin",
    "MonthlyActivitiesObjectiveTrackerMixin",
    "ProfessionsRecipeTrackerMixin",
    "QuestObjectiveTrackerMixin",
    "ScenarioObjectiveTrackerMixin",
    "UIWidgetObjectiveTrackerMixin",
    "WorldQuestObjectiveTrackerMixin",
];

const NAMED_NON_VIRTUAL_FRAMES: &[&str] = &[
    "ObjectiveTrackerFrame",
    "AchievementObjectiveTracker",
    "AdventureObjectiveTracker",
    "BonusObjectiveTracker",
    "CampaignQuestObjectiveTracker",
    "InitiativeTasksObjectiveTracker",
    "MonthlyActivitiesObjectiveTracker",
    "ProfessionsRecipeTracker",
    "QuestObjectiveTracker",
    "ScenarioObjectiveTracker",
    "UIWidgetObjectiveTracker",
    "WorldQuestObjectiveTracker",
    "ObjectiveTrackerUIWidgetContainer",
];

const VIRTUAL_TEMPLATES_NOT_IN_GLOBALS: &[&str] = &[
    "ObjectiveTrackerBlockTemplate",
    "ObjectiveTrackerAnimBlockTemplate",
    "ObjectiveTrackerContainerTemplate",
    "BonusObjectiveTrackerBlockTemplate",
    "AutoQuestPopUpBlockTemplate",
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
fn blizzard_objective_tracker_find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&objective_tracker_dir()).expect("Blizzard_ObjectiveTracker TOC resolves");
    assert_eq!(
        resolved,
        objective_tracker_toc(),
        "Blizzard_ObjectiveTracker ships exactly one bare TOC — no `_Mainline.toc` variant. \
         The tracker is one of the few large UI subsystems that retail kept under a single TOC \
         despite shipping flavor-restricted code paths (every Lua file uses Constants.* / \
         Enum.* / C_QuestLog.* lookups that resolve to retail-only values). The simulator's \
         `find_toc_file` at src/loader/mod.rs:67-92 prefers `_Mainline.toc` first, then this \
         bare `.toc` second, then a non-Classic fallback — the bare-TOC slot is the one that \
         resolves here"
    );

    let mainline = objective_tracker_dir().join("Blizzard_ObjectiveTracker_Mainline.toc");
    assert!(
        !mainline.exists(),
        "There must be NO `_Mainline.toc` at {} — the addon ships ONLY the bare TOC. The \
         _Mainline-suffixed slot is the first lookup at src/loader/mod.rs:67, so the absence \
         is load-bearing: it locks the addon into the bare-TOC code path",
        mainline.display()
    );
}

#[test]
fn blizzard_objective_tracker_toc_declares_eager_game_only_with_five_required_deps() {
    let toc =
        TocFile::from_file(&objective_tracker_toc()).expect("Blizzard_ObjectiveTracker TOC parses");

    assert!(
        !toc.is_load_on_demand(),
        "TOC OMITS `## LoadOnDemand:` so `is_load_on_demand()` returns false — the tracker is \
         the central quest/objective HUD; every player has it visible the moment they enter \
         the world, so eager loading is mandatory. A LoD route would force the engine to \
         delay the tracker until the first AddQuestWatch call, defeating the purpose of the \
         persistent on-screen quest list"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "TOC OMITS `## AllowLoad:` so `allows_screen` at src/toc.rs:311 returns true for the \
         Game screen by default — the omitted-key default is Game-only (NOT all screens), \
         which matches the tracker's purpose: in-world quest tracking has no meaning on glue \
         screens"
    );
    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            !toc.allows_screen(screen),
            "Default-Game-only `allows_screen({screen:?})` must return false — glue screens \
             have no QuestLog / WorldMap / CombatLog state, and the tracker's OnLoad \
             registers ZONE_CHANGED_NEW_AREA / ZONE_CHANGED / QUEST_ACCEPTED, none of which \
             fire on glue screens"
        );
    }

    assert_eq!(
        toc.dependencies(),
        REQUIRED_DEPS,
        "TOC must declare exactly 4 hard dependencies in source order: \
         Blizzard_MawBuffs, Blizzard_TieredEntranceTraits, \
         Blizzard_TransmogShared, and Blizzard_UIWidgets"
    );

    assert!(
        toc.optional_deps().is_empty(),
        "Current retail TOC declares no optional dependencies"
    );

    assert!(
        toc.saved_variables().is_empty(),
        "Zero saved variables — the tracker mirrors live server state (active quests, \
         scenario stage, world quest progress) on every Update. Persisting would only stale \
         the cache; the engine reissues all data via QUEST_LOG_UPDATE / WORLD_QUEST_UPDATE \
         on login"
    );
}

#[test]
fn blizzard_objective_tracker_toc_declares_dependencies_in_raw_bytes() {
    let raw = std::fs::read_to_string(objective_tracker_toc())
        .expect("Blizzard_ObjectiveTracker TOC reads utf-8");
    for dep in [
        "## Dep: Blizzard_MawBuffs",
        "## Dep: Blizzard_TieredEntranceTraits",
        "## Dep: Blizzard_TransmogShared",
        "## Dep: Blizzard_UIWidgets",
    ] {
        assert!(
            raw.contains(dep),
            "TOC must declare dependency line `{dep}`. Retail currently uses repeated `## Dep:` \
             lines here rather than comma-separated RequiredDep/OptionalDeps metadata"
        );
    }
    assert!(
        !raw.contains("## LoadOnDemand"),
        "TOC must NOT declare `## LoadOnDemand` in any form — the absence (rather than \
         `LoadOnDemand: 0`) is what flags eager loading. Distinct from Blizzard_Notification \
         which uses the explicit `LoadOnDemand: 0` form"
    );
    assert!(
        !raw.contains("## AllowLoad:"),
        "TOC must NOT declare `## AllowLoad:` — the absence is the canonical retail spelling \
         for Game-only addons (default), distinct from Blizzard_Notification's explicit \
         `## AllowLoad: Both`"
    );
    assert!(
        !raw.contains("## SavedVariables"),
        "TOC must NOT declare any `## SavedVariables*` keys — the tracker is a pure mirror of \
         live server state, no persistence"
    );
    assert!(
        raw.contains(&format!(
            "{ANNOTATED_MANAGER_FILE} [AllowLoadEnvironment Global]"
        )),
        "TOC must annotate the manager file with the per-file `[AllowLoadEnvironment Global]` \
         marker. The marker tells the retail engine to load the manager into the global \
         environment (rather than a sandboxed addon environment); the simulator's TOC parser \
         strips the annotation from the path and maps it to a `file_env_overrides` slot, the \
         same as `[LoadIntoEnvironment global]`"
    );
}

#[test]
fn blizzard_objective_tracker_toc_lists_forty_files_with_per_file_annotation_stripped() {
    let toc =
        TocFile::from_file(&objective_tracker_toc()).expect("Blizzard_ObjectiveTracker TOC parses");

    assert_eq!(
        toc.files.len(),
        TOC_FILE_COUNT,
        "TOC body must list exactly {TOC_FILE_COUNT} files (Lua + XML interleaved across 5 \
         logical groups separated by blank lines): foundation (5 Lua + 5 XML = 10 entries: \
         Fonts.xml, Manager.lua [annotated], Shared.lua/xml, Block.lua/xml, Module.lua/xml, \
         Container.lua/xml), anim templates (1 XML + 1 Lua), quest POI block (1 XML + 1 \
         Lua), tracker modules (12 Lua + 12 XML for 12 module variants), and entry point (1 \
         Lua + 1 XML). The blank-line grouping is a parser-irrelevant readability device — \
         the loader walks `files` linearly"
    );

    let manager = toc
        .files
        .iter()
        .find(|p| p.to_string_lossy() == ANNOTATED_MANAGER_FILE)
        .expect("Manager file present in TOC body");
    assert_eq!(
        manager.to_string_lossy(),
        ANNOTATED_MANAGER_FILE,
        "Manager file must appear with the `[AllowLoadEnvironment Global]` annotation \
         stripped — `push_file_entry` at src/toc.rs:147 strips ALL `[...]` annotations via \
         `strip_annotations` before constructing the PathBuf, so the file path stored in \
         `toc.files` is the bare filename. Pinning this guards against the parser \
         accidentally retaining the annotation as part of the path string (which would make \
         `load_addon` look for a file literally named `Blizzard_ObjectiveTrackerManager.lua \
         [AllowLoadEnvironment Global]` and fail with ENOENT)"
    );
}

#[test]
fn blizzard_objective_tracker_appears_in_game_screen_eager_discovery_only() {
    let ui = blizzard_ui_dir();

    let game_addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_ObjectiveTracker");
    assert!(
        in_game,
        "Blizzard_ObjectiveTracker must auto-discover on Game screen — eager (no \
         LoadOnDemand) AND default Game-only AllowLoad makes \
         `discover_blizzard_addons_for_screen(Game)` include the addon"
    );

    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&ui, screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_ObjectiveTracker");
        assert!(
            !found,
            "Blizzard_ObjectiveTracker must NOT auto-discover on screen {screen:?} — the \
             default Game-only AllowLoad restricts discovery to ScreenKind::Game. The \
             tracker references C_QuestLog / C_Map / C_Scenario from OnLoad / OnEvent, none \
             of which resolve on glue screens"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_objective_tracker_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_ObjectiveTracker")
                || message.contains("ObjectiveTrackerFrame")
                || message.contains("ObjectiveTrackerManager")
                || message.contains("ObjectiveTrackerContainer")
                || message.contains("ObjectiveTrackerModule")
                || message.contains("ObjectiveTrackerBlock")
                || message.contains("ObjectiveTrackerLine")
                || message.contains("QuestObjectiveTracker")
                || message.contains("ScenarioObjectiveTracker")
                || message.contains("BonusObjectiveTracker")
                || message.contains("WorldQuestObjectiveTracker")
                || message.contains("AchievementObjectiveTracker")
                || message.contains("CampaignQuestObjectiveTracker")
                || message.contains("ProfessionsRecipeTracker")
                || message.contains("AutoQuestPopup")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_ObjectiveTracker emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_objective_tracker_is_addon_loaded_after_eager_sweep(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_ObjectiveTracker')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_ObjectiveTracker') must return true after the eager \
         Game-screen sweep — no LoadOnDemand puts the addon in the eager set"
    );
}
}

prefork_full_ui_case! {
fn blizzard_objective_tracker_publishes_manager_singleton_with_initial_state(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G.ObjectiveTrackerManager)")
        .expect("ObjectiveTrackerManager probe succeeds");
    assert_eq!(
        kind, "table",
        "_G.ObjectiveTrackerManager must publish as a table — \
         Blizzard_ObjectiveTrackerManager.lua line 1 declares the singleton with initial \
         state `{{ containers = {{}}, moduleToContainerMap = {{}}, backgroundAlpha = 0, \
         canAddModules = true }}`. The manager is the registry that every container \
         (ObjectiveTrackerFrame, BonusObjectiveTracker, WorldQuestObjectiveTracker) calls \
         `:AddContainer` on during OnLoad. Without the singleton, every container's OnLoad \
         would error on the first nil-indexed `self.containers` mutation"
    );

    let can_add: bool = env
        .eval("return _G.ObjectiveTrackerManager.canAddModules")
        .expect("canAddModules probe succeeds");
    assert!(
        can_add,
        "ObjectiveTrackerManager.canAddModules must default to true — the boolean is the \
         registration latch: containers may register new tracker modules ONLY while \
         canAddModules is true. The manager flips it to false during specific lifecycle \
         transitions (e.g. UI reload), so the initial-true state is what enables every \
         module to register on first load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_objective_tracker_publishes_base_mixin_tables(env: &WowLuaEnv) {

    for mixin in PUBLIC_BASE_MIXINS {
        let kind: String = env
            .eval(&format!("return type(_G.{mixin})"))
            .unwrap_or_else(|err| panic!("type(_G.{mixin}) probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "_G.{mixin} must publish as a table — these are the foundation mixins composed \
             by every tracker module. ObjectiveTrackerFrameMixin owns the entry frame's \
             OnLoad/OnEvent (registers ZONE_CHANGED_NEW_AREA / ZONE_CHANGED / \
             QUEST_ACCEPTED). ObjectiveTrackerContainerMixin (extends DirtiableMixin) owns \
             container-level Update / HasAnyModules. ObjectiveTrackerModuleMixin (extends \
             ObjectiveTrackerSlidingMixin) owns module-level header layout, block iteration, \
             and scroll-anchor math. ObjectiveTrackerBlockMixin (extends Sliding) owns block \
             allocation/release. ObjectiveTrackerSlidingMixin owns the sliding-animation \
             primitive shared by containers, modules, and blocks. \
             ObjectiveTrackerAnimBlockMixin / ObjectiveTrackerAnimLineMixin add anim hooks. \
             ObjectiveTrackerQuestPOIBlockMixin (extends AnimBlock) is the base for \
             quest-POI-marked blocks. ObjectiveTrackerProgressBarMixin owns the shared \
             progress-bar widget"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_objective_tracker_publishes_eleven_module_mixin_tables(env: &WowLuaEnv) {

    for mixin in PUBLIC_MODULE_MIXINS {
        let kind: String = env
            .eval(&format!("return type(_G.{mixin})"))
            .unwrap_or_else(|err| panic!("type(_G.{mixin}) probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "_G.{mixin} must publish as a table — each module mixin extends \
             ObjectiveTrackerModuleMixin via `CreateFromMixins(ObjectiveTrackerModuleMixin, \
             settings)` (see e.g. AchievementObjectiveTracker.lua / \
             AdventureObjectiveTracker.lua / BonusObjectiveTracker.lua) and provides the \
             content-type-specific block factory + header text + reward-toast hook. The 11 \
             module mixins map 1:1 to the 12 tracker XML files (UIWidget reuses the bonus \
             objective settings). They are the leaf classes that the manager registers as \
             distinct trackers"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_objective_tracker_creates_named_xml_frames_as_globals(env: &WowLuaEnv) {

    for frame_name in NAMED_NON_VIRTUAL_FRAMES {
        let kind: String = env
            .eval(&format!("return type(_G.{frame_name})"))
            .unwrap_or_else(|err| panic!("type(_G.{frame_name}) probe failed: {err}"));
        assert!(
            kind == "table" || kind == "userdata",
            "_G.{frame_name} must be a frame (table or userdata) — every named non-virtual \
             `<Frame name=\"...\">` in the tracker XML files instantiates immediately at XML \
             parse time and publishes to _G via the standard frame-name binding path. The \
             entry-point ObjectiveTrackerFrame inherits \
             `UIParentRightManagedFrameTemplate, EditModeObjectiveTrackerSystemTemplate, \
             ObjectiveTrackerContainerTemplate` and mixes in ObjectiveTrackerFrameMixin; the \
             12 module-level frames each instantiate from a per-module template (e.g. \
             AchievementObjectiveTracker from ObjectiveTrackerModuleTemplate). \
             ObjectiveTrackerUIWidgetContainer is the dedicated UIWidget container module. \
             Got type {kind} for {frame_name}"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_objective_tracker_does_not_leak_virtual_templates_to_globals(env: &WowLuaEnv) {

    for template in VIRTUAL_TEMPLATES_NOT_IN_GLOBALS {
        let kind: String = env
            .eval(&format!("return type(_G.{template})"))
            .unwrap_or_else(|err| panic!("type(_G.{template}) probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "_G.{template} must be nil — virtual templates (`virtual=\"true\"`) live in the \
             template registry, NOT in `_G`. ObjectiveTrackerBlockTemplate / \
             ObjectiveTrackerAnimBlockTemplate / ObjectiveTrackerContainerTemplate are \
             abstract base templates inherited by per-module XML; \
             BonusObjectiveTrackerBlockTemplate / AutoQuestPopUpBlockTemplate are concrete \
             templates instantiated only via `inherits=\"...\"` on dynamically-created \
             child frames (the tracker's block pool). They must NOT publish as globals — a \
             leak would let addons mutate the template definition and break every existing \
             instance"
        );
    }
}
}
