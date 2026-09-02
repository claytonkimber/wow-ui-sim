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

fn garrison_templates_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_GarrisonTemplates")
}

fn garrison_templates_toc() -> PathBuf {
    garrison_templates_dir().join("Blizzard_GarrisonTemplates.toc")
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
fn blizzard_garrison_templates_resolves_single_unsuffixed_toc() {
    let resolved = find_toc_file(&garrison_templates_dir())
        .expect("Blizzard_GarrisonTemplates directory must contain a discoverable TOC");
    let resolved_name = resolved
        .file_name()
        .expect("resolved TOC must have a filename")
        .to_str()
        .expect("resolved TOC filename must be utf-8");

    assert_eq!(
        resolved_name, "Blizzard_GarrisonTemplates.toc",
        "Blizzard_GarrisonTemplates ships a SINGLE unsuffixed TOC (no `_Mainline`/`_Vanilla` \
         variants); src/loader/mod.rs:65's `find_toc_file` falls through to the bare \
         `<addon>.toc` when no `_Mainline.toc` is present"
    );
}

#[test]
fn blizzard_garrison_templates_toc_declares_lod_and_two_deps() {
    let toc = TocFile::from_file(&garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates TOC parse");

    assert!(
        toc.is_load_on_demand(),
        "Blizzard_GarrisonTemplates declares `## LoadOnDemand: 1` — it is loaded on \
         demand by Blizzard_GarrisonUI / Blizzard_OrderHallUI / Blizzard_AdventureMap when \
         the player opens a garrison mission UI, NOT eagerly at startup"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_GarrisonTemplates does not declare `## UseSecureEnvironment` — mission \
         template scripts read public garrison data, no protected-action surface"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_GarrisonTemplates declares no `## SavedVariables` — the templates do \
         not own persistent state, callers (GarrisonUI / OrderHallUI) own SavedVariables"
    );

    let deps = toc.dependencies();
    assert_eq!(
        deps,
        vec![
            "Blizzard_Colors".to_string(),
            "Blizzard_HelpPlate".to_string(),
        ],
        "`## Dependencies: Blizzard_Colors, Blizzard_HelpPlate` — Colors provides the \
         ITEM_QUALITY_COLORS palette used by GARRISON_FOLLOWER_QUALITY_DESC entries (line \
         5 of Blizzard_GarrisonSharedTemplates.lua), HelpPlate provides the \
         HelpPlateBox/HelpPlate_Show overlay system used by the mission tutorial popups. \
         Got: {:?}",
        deps
    );

    let toc_text = std::fs::read_to_string(garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_GarrisonTemplates declares NO `## AllowLoad:` line — combined with \
         `## LoadOnDemand: 1`, the addon defaults to Game-only AND is excluded from \
         auto-discovery (callers must invoke `LoadAddOn(\"Blizzard_GarrisonTemplates\")`)"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_GarrisonTemplates declares NO `## AllowLoadGameType:` line — the \
         templates are mission-system primitives shared across mainline/classic flavors \
         that ship a Garrison-derived UI"
    );
}

#[test]
fn blizzard_garrison_templates_toc_lists_three_xml_and_localization_lua() {
    let toc_text = std::fs::read_to_string(garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates TOC should read");
    let xml_lines = [
        "Blizzard_GarrisonSharedTemplates.xml",
        "Blizzard_GarrisonMissionTemplates.xml",
        "Blizzard_CovenantMissionTemplates.xml",
    ];
    for xml in xml_lines.iter() {
        assert!(
            toc_text.contains(xml),
            "Blizzard_GarrisonTemplates TOC must list {xml} — the TOC enumerates 3 XML \
             files; each .xml `<Script file=\"X.lua\"/>` directive includes its matching \
             .lua sibling, so the TOC delegates Lua loading to the XML loader"
        );
    }
    assert!(
        toc_text.contains("Localization.lua"),
        "Blizzard_GarrisonTemplates TOC must list Localization.lua directly — it is the \
         lone .lua file enumerated in the TOC (the per-locale l10nTable holds \
         localize() functions that override globals like \
         COLLAPSE_ORDER_HALL_FOLLOWER_ITEM_LEVEL_DISPLAY for zhCN)"
    );
    assert!(
        !toc_text.contains("Blizzard_GarrisonSharedTemplates.lua\n"),
        "TOC must NOT list Blizzard_GarrisonSharedTemplates.lua directly — it is loaded \
         via the .xml file's `<Script file=...>` include directive"
    );
}

#[test]
fn blizzard_garrison_templates_excluded_from_game_auto_discovery_due_to_lod() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_GarrisonTemplates");
    assert!(
        !in_game,
        "Blizzard_GarrisonTemplates declares `## LoadOnDemand: 1` — it must NOT appear \
         in Game-screen auto-discovery (callers like Blizzard_GarrisonUI invoke \
         `LoadAddOn(\"Blizzard_GarrisonTemplates\")` to bring it in lazily)"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_GarrisonTemplates");
    assert!(
        !in_login,
        "LoadOnDemand + Game-only-default also excludes Blizzard_GarrisonTemplates from \
         Login auto-discovery"
    );
}

prefork_full_ui_case! {
fn blizzard_garrison_templates_loads_explicitly_via_load_addon_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let templates_errors: Vec<String> = load_errors
        .iter()
        .filter(|message| {
            message.contains("Garrison")
                || message.contains("Covenant")
                || message.contains("Adventures")
                || message.contains("Mission")
        })
        .cloned()
        .collect();
    assert!(
        templates_errors.is_empty(),
        "Blizzard_GarrisonTemplates emitted Lua errors during explicit load:\n  {}",
        templates_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_garrison_templates_is_addon_loaded_returns_true_after_explicit_load(env: &WowLuaEnv) {

    let before: bool = env
        .eval("return C_AddOns and C_AddOns.IsAddOnLoaded('Blizzard_GarrisonTemplates') or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        !before,
        "IsAddOnLoaded should return false BEFORE explicit LoadAddOn — the LOD flag \
         keeps Blizzard_GarrisonTemplates out of auto-discovery"
    );

    load_addon(&env.loader_env(), &garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates should load via Rust loader");

    let after: bool = env
        .eval("return C_AddOns and C_AddOns.IsAddOnLoaded('Blizzard_GarrisonTemplates') or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        after,
        "C_AddOns.IsAddOnLoaded('Blizzard_GarrisonTemplates') must return true AFTER \
         explicit LoadAddOn (the loader must register the addon's name + state in the \
         addon-info table that backs IsAddOnLoaded)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_garrison_templates_publishes_top_level_named_frames(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates should load");

    let frames: (String, String, String, String, String, String) = env
        .eval(
            "return type(GarrisonFollowerPlacerFrame), \
                    type(GarrisonMissionMechanicTooltip), \
                    type(GarrisonMissionMechanicFollowerCounterTooltip), \
                    type(GarrisonTruncationFrame), \
                    type(GarrisonThreatCountersFrame), \
                    type(GarrisonConfirmFollowerAbilityUpgradeFrame)",
        )
        .expect("Top-level Garrison frame probe should succeed");
    assert_eq!(
        frames,
        (
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
        ),
        "Blizzard_GarrisonTemplates publishes 6 non-virtual top-level named frames: \
         GarrisonFollowerPlacerFrame (MissionTemplates.xml line 1525 — Button on \
         UIParent at HIGH strata covering setAllPoints, the drag-overlay frame caught \
         by GarrisonFollowerPlacerFrame_OnClick to cancel a follower drag), \
         GarrisonMissionMechanicTooltip (MissionTemplates.xml line 1534 — GameTooltip \
         inheriting GameTooltipTemplate at TOOLTIP strata, the mechanic-icon hover \
         tooltip), GarrisonMissionMechanicFollowerCounterTooltip (line 1569 — the \
         follower-counter variant on UIParent at TOOLTIP strata), GarrisonTruncationFrame \
         (SharedTemplates.xml line 615), GarrisonThreatCountersFrame (line 705 — \
         hidden default-instance of GarrisonThreatCountersFrameTemplate), \
         GarrisonConfirmFollowerAbilityUpgradeFrame (line 707 — hidden ability-upgrade \
         confirm popup)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_garrison_templates_virtual_templates_do_not_leak_as_globals(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates should load");

    let leaks: (bool, bool, bool, bool, bool, bool, bool, bool) = env
        .eval(
            "return _G.GarrisonUITemplate == nil, \
                    _G.GarrisonMissionBaseFrameTemplate == nil, \
                    _G.GarrisonFollowerButtonTemplate == nil, \
                    _G.GarrisonMissionFrameTemplate == nil, \
                    _G.GarrisonMissionPageStageTemplate == nil, \
                    _G.CovenantFollowerTabTemplate == nil, \
                    _G.CovenantMissionListTemplate == nil, \
                    _G.AdventuresPuckHealthBarTemplate == nil",
        )
        .expect("Virtual template leak probe should succeed");
    assert_eq!(
        leaks,
        (true, true, true, true, true, true, true, true),
        "All 8 sampled virtual templates must NOT leak as `_G.*` globals: \
         GarrisonUITemplate (SharedTemplates.xml line 5), \
         GarrisonMissionBaseFrameTemplate (line 78), GarrisonFollowerButtonTemplate \
         (line 241), GarrisonMissionFrameTemplate (MissionTemplates.xml line 764 — \
         `parent=\"UIParent\" enableMouse=\"true\" toplevel=\"true\" hidden=\"true\" \
         virtual=\"true\"`), GarrisonMissionPageStageTemplate (line 543), \
         CovenantFollowerTabTemplate (CovenantMissionTemplates.xml line 549), \
         CovenantMissionListTemplate (line 758), AdventuresPuckHealthBarTemplate \
         (line 5) — proves the `virtual=\"true\"` contract holds across all 3 XML files"
    );
}
}

prefork_full_ui_case! {
fn blizzard_garrison_templates_publishes_garrison_shared_mixins(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates should load");

    let shared: (String, String, String, String) = env
        .eval(
            "return type(GarrisonMissionFollowerDurabilityMixin), \
                    type(GarrisonMissionFollowerOrCategoryListButtonMixin), \
                    type(GarrisonFollowerTabMixin), \
                    type(GarrisonFollowerList)",
        )
        .expect("Garrison shared mixin probe should succeed");
    assert_eq!(
        shared,
        (
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
        ),
        "Blizzard_GarrisonSharedTemplates.lua publishes 4 mixin/namespace tables: \
         GarrisonMissionFollowerDurabilityMixin (line 273 — owns the durability-bar \
         render methods consumed by GarrisonMissionFollowerDurabilityFrameTemplate), \
         GarrisonMissionFollowerOrCategoryListButtonMixin (line 343 — owns the \
         category/follower list-button OnClick dispatch), GarrisonFollowerTabMixin \
         (line 1664 — the canonical follower-detail tab mixin overridden by \
         CovenantFollowerTabMixin via `mixin=\"GarrisonFollowerTabMixin,CovenantFollowerTabMixin\"`), \
         GarrisonFollowerList (line 77 — the follower-list namespace, NOT a mixin)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_garrison_templates_publishes_mission_template_mixins(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates should load");

    let mission: (String, String, String) = env
        .eval(
            "return type(GarrisonMissionPageMixin), \
                    type(GarrisonMissionCompleteModelClusterMixin), \
                    type(GarrisonMissionPageCostWithTooltipMixin)",
        )
        .expect("Garrison mission mixin probe should succeed");
    assert_eq!(
        mission,
        (
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
        ),
        "Blizzard_GarrisonMissionTemplates.lua publishes 3 mixin tables: \
         GarrisonMissionPageMixin (line 2255 — the canonical mission-detail page \
         layout/scoring mixin), GarrisonMissionCompleteModelClusterMixin (line 2943 — \
         the post-mission cinematic 3D-model cluster controller), \
         GarrisonMissionPageCostWithTooltipMixin (line 2959 — the cost-frame \
         tooltip-on-hover mixin attached to GarrisonMissionPageCostWithTooltipTemplate \
         via the mixin=... XML attribute)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_garrison_templates_publishes_covenant_mission_mixins(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates should load");

    let covenant: (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ) = env
        .eval(
            "return type(CovenantMissionPageEnemyMixin), \
                    type(CovenantFollowerTabMixin), \
                    type(CovenantMissionListMixin), \
                    type(CovenantMissionEncounterIconMixin), \
                    type(AdventuresTargetingIndicatorMixin), \
                    type(AdventuresFriendlyTargetingIndicatorMixin), \
                    type(SupportColorationAnimatorMixin), \
                    type(CovenantPortraitMixin), \
                    type(AdventuresPuckHealthBarMixin)",
        )
        .expect("Covenant mission mixin probe should succeed");
    assert_eq!(
        covenant,
        (
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
        ),
        "Blizzard_CovenantMissionTemplates.lua publishes 9 mixin tables driving the \
         9.0 Covenant adventure-board UI: CovenantMissionPageEnemyMixin (line 22), \
         CovenantFollowerTabMixin (line 81 — co-mixed with GarrisonFollowerTabMixin on \
         CovenantFollowerTabTemplate), CovenantMissionListMixin (line 254), \
         CovenantMissionEncounterIconMixin (line 540 — drives Encounter / \
         SmallCovenantMissionEncounterIcon / CovenantLandingPageEncounterIcon \
         templates), AdventuresTargetingIndicatorMixin (line 566 — enemy-target \
         crosshair), AdventuresFriendlyTargetingIndicatorMixin (line 625 — ally-target \
         crosshair), SupportColorationAnimatorMixin (line 675 — pulsing tint applied \
         when an ally support spell is staged), CovenantPortraitMixin (line 763 — the \
         hexagonal Covenant portrait frame), AdventuresPuckHealthBarMixin (line 800 — \
         the on-puck health bar inheriting AdventuresPuckHealthBarTemplate)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_garrison_templates_publishes_garrison_follower_helper_globals(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates should load");

    let helpers: (bool, bool, bool, bool, bool, bool) = env
        .eval(
            "return type(GarrisonFollowerListButton_OnDragStart) == 'function', \
                    type(GarrisonFollowerListButton_OnDragStop) == 'function', \
                    type(GarrisonFollowerListButton_OnClick) == 'function', \
                    type(GarrisonFollowerList_SetButtonMode) == 'function', \
                    type(GarrisonFollowerList_SortFollowers) == 'function', \
                    type(GarrisonMission_SetFollowerModel) == 'function'",
        )
        .expect("Garrison follower helper probe should succeed");
    assert_eq!(
        helpers,
        (true, true, true, true, true, true),
        "Blizzard_GarrisonSharedTemplates.lua publishes the canonical \
         follower-list/drag-drop helper API: GarrisonFollowerListButton_OnDragStart \
         (line 329 — initiates a follower drag, sets up GarrisonFollowerPlacerFrame), \
         GarrisonFollowerListButton_OnDragStop (line 336), \
         GarrisonFollowerListButton_OnClick (line 953 — the canonical click router \
         that dispatches expand/select/assign by button + modifier), \
         GarrisonFollowerList_SetButtonMode (line 456 — toggles between collapsed and \
         expanded list-button layouts), GarrisonFollowerList_SortFollowers (line 1290 \
         — drives the sort-by selector via Default/Mission/Specialization variants), \
         GarrisonMission_SetFollowerModel (line 1343 — populates a CinematicModel \
         from a followerID + displayID + showWeapon)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_garrison_templates_mechanic_tooltips_use_tooltip_strata(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates should load");

    let counter_strata: (String, String) = env
        .eval(
            "return GarrisonMissionMechanicFollowerCounterTooltip:GetFrameStrata(), \
                    GarrisonMissionMechanicFollowerCounterTooltip:GetParent():GetName()",
        )
        .expect("MechanicFollowerCounterTooltip probe should succeed");
    assert_eq!(
        counter_strata,
        ("TOOLTIP".to_string(), "UIParent".to_string()),
        "GarrisonMissionMechanicFollowerCounterTooltip (xml line 1569) declares \
         `parent=\"UIParent\" frameStrata=\"TOOLTIP\"` — the follower-counter mechanic \
         tooltip rides at the topmost strata above the mission-detail panel, parented \
         to UIParent so it stays visible when the mission frame closes"
    );

    let mechanic_strata: String = env
        .eval("return GarrisonMissionMechanicTooltip:GetFrameStrata()")
        .expect("MechanicTooltip strata probe should succeed");
    assert_eq!(
        mechanic_strata, "TOOLTIP",
        "GarrisonMissionMechanicTooltip (xml line 1534) declares \
         `frameStrata=\"TOOLTIP\"` — the in-mission mechanic-icon hover rides at the \
         same strata as GameTooltip"
    );
}
}

prefork_full_ui_case! {
fn blizzard_garrison_templates_follower_placer_uses_uiparent_high_strata(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates should load");

    let placer: (String, String, bool) = env
        .eval(
            "return GarrisonFollowerPlacerFrame:GetFrameStrata(), \
                    GarrisonFollowerPlacerFrame:GetParent():GetName(), \
                    GarrisonFollowerPlacerFrame:IsShown() == false",
        )
        .expect("FollowerPlacerFrame probe should succeed");
    assert_eq!(
        placer,
        ("HIGH".to_string(), "UIParent".to_string(), true),
        "GarrisonFollowerPlacerFrame (xml line 1525) declares `parent=\"UIParent\" \
         frameStrata=\"HIGH\" setAllPoints=\"true\" hidden=\"true\"` — when a follower \
         drag begins, this fullscreen Button overlays the entire UI at HIGH strata to \
         catch any click that lands outside a valid drop target (the OnClick handler \
         GarrisonFollowerPlacerFrame_OnClick cancels the drag and hides the overlay)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_garrison_templates_publishes_garrison_follower_quality_descriptors(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &garrison_templates_toc())
        .expect("Blizzard_GarrisonTemplates should load");

    let descriptors: (String, String, String) = env
        .eval(
            "return type(GARRISON_FOLLOWER_QUALITY_DESC), \
                    type(GARRISON_FOLLOWER_BUSY_COLOR), \
                    type(GARRISON_FOLLOWER_INACTIVE_COLOR)",
        )
        .expect("Quality descriptor probe should succeed");
    assert_eq!(
        descriptors,
        (
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
        ),
        "Blizzard_GarrisonSharedTemplates.lua publishes 3 top-level constant tables: \
         GARRISON_FOLLOWER_QUALITY_DESC (line 5 — per-quality (uncommon/rare/epic) \
         display labels keyed by ITEM_QUALITY_COLOR enum), \
         GARRISON_FOLLOWER_BUSY_COLOR (line 1 — RGBA tint for in-mission followers), \
         GARRISON_FOLLOWER_INACTIVE_COLOR (line 2 — RGBA tint for inactive followers)"
    );
}
}
