#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn islands_queue_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_IslandsQueueUI")
}

fn islands_queue_toc() -> PathBuf {
    islands_queue_dir().join("Blizzard_IslandsQueueUI.toc")
}

fn ui_widgets_toc() -> PathBuf {
    find_toc_file(&blizzard_ui_dir().join("Blizzard_UIWidgets"))
        .expect("Blizzard_UIWidgets TOC should resolve")
}

fn help_plate_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_HelpPlate/Blizzard_HelpPlate.toc")
}

const MIXIN_NAMES: &[&str] = &[
    "IslandsQueueWeeklyQuestMixin",
    "IslandsQueueWeeklyQuestRewardMixin",
    "IslandsQueueFrameMixin",
    "IslandsQueueFrameDifficultyMixin",
];

const VIRTUAL_TEMPLATES: &[&str] = &[
    "IslandsQueueFrameTutorialTemplate",
    "IslandsQueueFrameWeeklyQuestFrameTemplate",
    "IslandsQueueFrameDifficultyButtonTemplate",
    "IslandsQueueScreenDifficultySelector",
    "IslandsQueueFrameIslandCardTemplate",
    "IslandsQueueFrameCardFrameTemplate",
];

const WEEKLY_QUEST_METHODS: &[&str] = &[
    "OnEvent",
    "OnShow",
    "OnHide",
    "UpdateRewardInformation",
    "SetElementsEnabled",
    "UpdateQuestProgressBar",
    "Refresh",
];

const DIFFICULTY_METHODS: &[&str] = &[
    "OnQueueClick",
    "QueueButtonSetState",
    "UpdateQueueText",
    "OnShow",
    "OnHide",
    "OnLoad",
    "OnEvent",
    "SetInitialDifficulty",
    "PreloadQuestRewardInformation",
    "RefreshDifficultyButtons",
    "SetActiveDifficulty",
    "GetActiveDifficulty",
];

fn load_islands_queue_ui_with_dependencies(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &ui_widgets_toc())
        .expect("Blizzard_UIWidgets should load via explicit Rust loader call");
    load_addon(&env.loader_env(), &help_plate_toc())
        .expect("Blizzard_HelpPlate should load via explicit Rust loader call");
    load_addon(&env.loader_env(), &islands_queue_toc())
        .expect("Blizzard_IslandsQueueUI should load via explicit Rust loader call");
}

#[test]
fn blizzard_islands_queue_find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&islands_queue_dir()).expect("Blizzard_IslandsQueueUI TOC should resolve");
    assert_eq!(
        resolved,
        islands_queue_toc(),
        "Blizzard_IslandsQueueUI ships exactly one bare TOC — the LoadOnDemand island-queue \
         portrait module resolves via `find_toc_file` fallthrough"
    );
}

#[test]
fn blizzard_islands_queue_toc_declares_lod_with_two_required_deps() {
    let toc =
        TocFile::from_file(&islands_queue_toc()).expect("Blizzard_IslandsQueueUI TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_IslandsQueueUI declares `## LoadOnDemand: 1` — pulled in when the player \
         opens the island-expedition queue panel from the LFG / mission UI; not loaded eagerly \
         because most play sessions never queue for an island"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert_eq!(
        toc.dependencies(),
        vec![
            "Blizzard_UIWidgets".to_string(),
            "Blizzard_HelpPlate".to_string(),
        ],
        "Blizzard_IslandsQueueUI declares `## RequiredDep: Blizzard_UIWidgets, \
         Blizzard_HelpPlate` — `RequiredDep` resolves through the same `dependencies()` \
         accessor as `Dependencies`. Blizzard_UIWidgets provides the \
         UIWidgetContainerNoResizeTemplate the IslandsQueueFrameCardFrameTemplate inherits \
         (server-driven island-card widget set rendering); Blizzard_HelpPlate provides the \
         help-plate / tooltip-driven tutorial overlay infrastructure the \
         IslandsQueueFrameTutorialTemplate consumes for the first-time-user explainer plate"
    );
    assert!(
        toc.optional_deps().is_empty(),
        "Blizzard_IslandsQueueUI declares no `## OptionalDeps` — both required deps cover the \
         external template surface; no optional fallback path"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_IslandsQueueUI declares zero saved variables — every queue session re-fetches \
         from the server via the C_IslandsQueue API; no client-side persistence (active \
         difficulty selection rebuilds from C_IslandsQueue.GetIslandDifficulty on each show)"
    );
}

#[test]
fn blizzard_islands_queue_toc_declares_standard_game_type_only() {
    let toc =
        TocFile::from_file(&islands_queue_toc()).expect("Blizzard_IslandsQueueUI TOC should parse");

    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_IslandsQueueUI declares `## AllowLoadGameType: standard` — \
         `is_game_type_restricted()` (src/toc.rs:294) treats both `standard` and `mainline` as \
         the unrestricted retail flavor, so the addon is available on retail Midnight"
    );

    let raw = std::fs::read_to_string(islands_queue_toc())
        .expect("Blizzard_IslandsQueueUI TOC should read");
    assert!(
        raw.contains("## AllowLoadGameType: standard"),
        "TOC must declare `## AllowLoadGameType: standard` — pins the addon to the retail \
         island-expedition codepath; classic flavors don't ship the C_IslandsQueue bridge this \
         queue UI consumes"
    );
    assert!(
        !raw.contains("## AllowLoad:"),
        "TOC must omit `## AllowLoad:` — LoD addons rely entirely on the explicit LoadAddOn \
         from the islands queue flow, never via the screen auto-discovery sweep"
    );
    assert!(
        !raw.contains("## DefaultState:"),
        "TOC must omit `## DefaultState:` — LoD addons load only when the islands queue flow \
         requests them"
    );
}

#[test]
fn blizzard_islands_queue_toc_lists_bootstrap_lua_then_xml_in_order() {
    let toc =
        TocFile::from_file(&islands_queue_toc()).expect("Blizzard_IslandsQueueUI TOC should parse");
    assert_eq!(
        toc.files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        vec![
            "Blizzard_IslandsQueueUI_Bootstrap.lua".to_string(),
            "Blizzard_IslandsQueueUI.lua".to_string(),
            "Blizzard_IslandsQueueUI.xml".to_string(),
        ],
        "TOC body lists its Bootstrap file before the Lua and XML body in current retail order"
    );
}

#[test]
fn blizzard_islands_queue_directory_holds_four_entries() {
    let entries = std::fs::read_dir(islands_queue_dir())
        .expect("Blizzard_IslandsQueueUI directory should read")
        .count();
    assert_eq!(
        entries, 4,
        "Directory holds the TOC, Bootstrap, Lua, and XML files"
    );
}

#[test]
fn blizzard_islands_queue_excluded_from_every_screen_auto_discovery() {
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_IslandsQueueUI");
        assert!(
            !found,
            "Blizzard_IslandsQueueUI must NOT appear in any ScreenKind auto-discovery sweep — \
             `## LoadOnDemand: 1` excludes it from every eager pass; only `load_addon` called \
             explicitly by the islands queue flow pulls it in. (Screen tested: {screen:?})"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_islands_queue_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_islands_queue_ui_with_dependencies(env);

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_IslandsQueueUI")
                || message.contains("IslandsQueueFrame")
                || message.contains("IslandsQueueWeeklyQuest")
                || message.contains("IslandsQueueScreen")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_IslandsQueueUI emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_islands_queue_is_addon_loaded_after_explicit_lod_with_deps(env: &WowLuaEnv) {
    load_islands_queue_ui_with_dependencies(env);

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_IslandsQueueUI')")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_IslandsQueueUI') must return true after the explicit \
         load_addon call"
    );

    let widgets_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_UIWidgets')")
        .expect("Blizzard_UIWidgets IsAddOnLoaded probe should succeed");
    assert!(
        widgets_loaded,
        "Blizzard_UIWidgets dep must register as loaded — provides the \
         UIWidgetContainerNoResizeTemplate the IslandsQueueFrameCardFrameTemplate inherits"
    );

    let help_plate_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HelpPlate')")
        .expect("Blizzard_HelpPlate IsAddOnLoaded probe should succeed");
    assert!(
        help_plate_loaded,
        "Blizzard_HelpPlate dep must register as loaded — provides the help-plate / \
         tutorial-overlay infrastructure the IslandsQueueFrameTutorialTemplate consumes"
    );
}
}

prefork_full_ui_case! {
fn blizzard_islands_queue_publishes_all_four_mixins(env: &WowLuaEnv) {
    load_islands_queue_ui_with_dependencies(env);

    for mixin in MIXIN_NAMES {
        let kind: String = env
            .eval(&format!("return type(_G['{mixin}'])"))
            .unwrap_or_else(|err| panic!("{mixin} probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "{mixin} must publish at `_G` as a table — declared via `{mixin} = {{}}` at file \
             scope (Blizzard_IslandsQueueUI.lua) so each XML-instantiated frame's \
             `mixin=\"{mixin}\"` attribute resolves through global scope at frame-creation time"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_islands_queue_weekly_quest_mixin_carries_seven_methods(env: &WowLuaEnv) {
    load_islands_queue_ui_with_dependencies(env);

    for method in WEEKLY_QUEST_METHODS {
        let kind: String = env
            .eval(&format!(
                "return type(IslandsQueueWeeklyQuestMixin['{method}'])"
            ))
            .unwrap_or_else(|err| {
                panic!("IslandsQueueWeeklyQuestMixin.{method} probe failed: {err}")
            });
        assert_eq!(
            kind, "function",
            "IslandsQueueWeeklyQuestMixin.{method} must publish as a function — drives the \
             weekly quest tracker frame's lifecycle (OnEvent / OnShow / OnHide), reward \
             plumbing (UpdateRewardInformation), enable-state toggling \
             (SetElementsEnabled / UpdateQuestProgressBar), and the catch-all Refresh that \
             coalesces every state change into a single update path"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_islands_queue_difficulty_mixin_carries_twelve_methods(env: &WowLuaEnv) {
    load_islands_queue_ui_with_dependencies(env);

    for method in DIFFICULTY_METHODS {
        let kind: String = env
            .eval(&format!(
                "return type(IslandsQueueFrameDifficultyMixin['{method}'])"
            ))
            .unwrap_or_else(|err| {
                panic!("IslandsQueueFrameDifficultyMixin.{method} probe failed: {err}")
            });
        assert_eq!(
            kind, "function",
            "IslandsQueueFrameDifficultyMixin.{method} must publish as a function — drives the \
             difficulty selector's lifecycle (OnLoad / OnShow / OnHide / OnEvent), queue-button \
             behavior (OnQueueClick / QueueButtonSetState / UpdateQueueText), the active \
             difficulty state machine (SetInitialDifficulty / SetActiveDifficulty / \
             GetActiveDifficulty / RefreshDifficultyButtons), and reward-side preloading \
             (PreloadQuestRewardInformation)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_islands_queue_frame_mixin_carries_three_lifecycle_methods(env: &WowLuaEnv) {
    load_islands_queue_ui_with_dependencies(env);

    for method in &["OnLoad", "OnShow", "OnHide"] {
        let kind: String = env
            .eval(&format!("return type(IslandsQueueFrameMixin['{method}'])"))
            .unwrap_or_else(|err| panic!("IslandsQueueFrameMixin.{method} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "IslandsQueueFrameMixin.{method} must publish as a function — only 3 lifecycle \
             methods on the master IslandsQueueFrame; every other behavior delegates to the \
             nested DifficultySelectorFrame / WeeklyQuestFrame mixins or the inherited \
             PortraitFrameTemplate"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_islands_queue_virtual_templates_stay_nil_at_global_scope(env: &WowLuaEnv) {
    load_islands_queue_ui_with_dependencies(env);

    for template_name in VIRTUAL_TEMPLATES {
        let kind: String = env
            .eval(&format!("return type(_G['{template_name}'])"))
            .unwrap_or_else(|err| panic!("_G[{template_name}] probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "{template_name} must NOT publish at `_G` — declared as `virtual=\"true\"` so the \
             loader keeps it in the template registry only. Consumed via `inherits=` by \
             concrete IslandsQueueFrame children (the difficulty buttons / weekly quest tracker \
             / island cards / tutorial plate), never resolved through global scope"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_islands_queue_named_frame_publishes_with_portrait_template_chain(env: &WowLuaEnv) {
    load_islands_queue_ui_with_dependencies(env);

    let kind: String = env
        .eval("return type(IslandsQueueFrame)")
        .expect("IslandsQueueFrame probe should succeed");
    assert_eq!(
        kind, "table",
        "IslandsQueueFrame must publish at `_G` as a table — declared with \
         `name=\"IslandsQueueFrame\"` `parent=\"UIParent\"` `hidden=\"true\"` \
         `frameStrata=\"HIGH\"` `enableMouse=\"true\"` `inherits=\"PortraitFrameTemplate\"` \
         `mixin=\"IslandsQueueFrameMixin\"`"
    );

    let name: String = env
        .eval("return IslandsQueueFrame:GetName()")
        .expect("IslandsQueueFrame:GetName() probe should succeed");
    assert_eq!(name, "IslandsQueueFrame");

    let strata: String = env
        .eval("return IslandsQueueFrame:GetFrameStrata()")
        .expect("IslandsQueueFrame:GetFrameStrata() probe should succeed");
    assert_eq!(
        strata, "HIGH",
        "IslandsQueueFrame must report HIGH strata — declared `frameStrata=\"HIGH\"` so the \
         queue panel sits above the standard MEDIUM-strata UIPanel surface"
    );

    let hidden: bool = env
        .eval("return IslandsQueueFrame:IsShown() == false")
        .expect("IslandsQueueFrame:IsShown() probe should succeed");
    assert!(
        hidden,
        "IslandsQueueFrame declares `hidden=\"true\"` — must start hidden until the islands \
         queue flow Show()s the frame"
    );
}
}
