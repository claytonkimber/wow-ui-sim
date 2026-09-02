use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn world_loot_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_WorldLootObjectList")
}

fn world_loot_toc() -> PathBuf {
    world_loot_dir().join("Blizzard_WorldLootObjectList.toc")
}

const BUTTON_MIXIN_METHODS: &[&str] = &[
    "Init",
    "OnUpdate",
    "OnEnter",
    "OnLeave",
    "OnMouseDown",
    "SetDummy",
    "UpdateDisabledState",
    "Refresh",
];

const LIST_MIXIN_METHODS: &[&str] = &[
    "OnLoad",
    "OnUpdate",
    "OnObjectShown",
    "OnObjectHidden",
    "EvaluateVisibility",
    "InsertNewWidget",
    "Refresh",
    "RefreshScrollBox",
];

const VIRTUAL_TEMPLATES: &[&str] = &[
    "WorldLootObjectListButtonTemplate",
    "WorldLootObjectListTemplate",
];

fn load_world_loot_object_list(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &world_loot_toc()).expect(
        "Blizzard_WorldLootObjectList should still load when invoked explicitly even though \
         eager discovery skips it on a mainline build (game-type-restricted)",
    );
}

#[test]
fn find_toc_file_resolves_bare_variant() {
    let resolved =
        find_toc_file(&world_loot_dir()).expect("Blizzard_WorldLootObjectList TOC should resolve");
    assert_eq!(
        resolved,
        world_loot_toc(),
        "Blizzard_WorldLootObjectList ships exactly one bare TOC — find_toc_file probes the \
         `_Mainline.toc` variant first (miss) then falls through to the bare TOC name (hit). \
         The addon ships only on plunderstorm-flavored builds, so no `_Mainline.toc` companion \
         is needed (mainline excludes it via the AllowLoadGameType filter)"
    );
}

#[test]
fn toc_declares_plunderstorm_only_game_type() {
    let toc = TocFile::from_file(&world_loot_toc())
        .expect("Blizzard_WorldLootObjectList TOC should parse");

    assert!(
        toc.is_game_type_restricted(),
        "Blizzard_WorldLootObjectList declares `## AllowLoadGameType: plunderstorm` — toc.rs:294-302 \
         returns true because `plunderstorm` is NOT in the accept-set {{`mainline`, `standard`}}. \
         This is the FIRST campaign addon analyzed exercising a game-type-restricted branch. \
         Plunderstorm is the WoW Remix battle-royale-style limited-time event (introduced in \
         patch 10.2.6); 11 BlizzardUI tree addons share this restriction \
         (Blizzard_ArrowCalloutFrame / Blizzard_EndOfMatchUI / Blizzard_GameMenu_WoWLabs / \
         Blizzard_HUDInventoryTemplates / Blizzard_ItemBeltFrame / Blizzard_PlunderstormPrematchUI / \
         Blizzard_SpectateFrame / Blizzard_SpellPickUpIndicator / and this one) — none of them \
         are eligible for eager loading on a default mainline build"
    );

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "`## AllowLoad: Game` (capital G — case-insensitive at toc.rs:308) restricts the addon \
         to the Game screen IF the game-type filter ever lets it through. On a mainline build \
         the game-type restriction is the gate that excludes it; on a plunderstorm build the \
         AllowLoad filter would let it pass for Game"
    );
    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            !toc.allows_screen(screen),
            "AllowLoad:Game rejects glue screen {screen:?} regardless of game type"
        );
    }
}

#[test]
fn toc_default_state_enabled_with_no_dependencies() {
    let toc = TocFile::from_file(&world_loot_toc())
        .expect("Blizzard_WorldLootObjectList TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_WorldLootObjectList declares no `## LoadOnDemand` directive — toc.rs:259-264 \
         returns false (eager). On a plunderstorm build the addon loads at session start"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_WorldLootObjectList declares no dependencies — the lua file consumes only the \
         C_WorldLootObject.IsWorldLootObjectInRange C API + C_Spell.GetSpellInfo + \
         C_UIWidgetManager.GetSpellDisplayVisualizationInfo + EventRegistry callbacks + \
         CreateScrollBoxListLinearView + CreateDataProvider + UIWidgetTemplateSpellDisplay + \
         WowScrollBoxList. All of those are provided by the eagerly-loaded core (Blizzard_SharedXMLBase \
         publishes EventRegistry; Blizzard_SharedXML publishes the scroll-box helpers; the C API \
         surface provides the rest)"
    );
    assert!(toc.optional_deps().is_empty());
    assert!(toc.saved_variables().is_empty());
}

#[test]
fn toc_lists_lua_then_xml_in_order() {
    let toc = TocFile::from_file(&world_loot_toc())
        .expect("Blizzard_WorldLootObjectList TOC should parse");
    assert_eq!(
        toc.files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        vec![
            "Blizzard_WorldLootObjectList.lua".to_string(),
            "Blizzard_WorldLootObjectList.xml".to_string(),
        ],
        "TOC body lists lua FIRST then xml — the lua file declares both mixin tables \
         (WorldLootObjectListButtonMixin at line 8, WorldLootObjectListMixin at line 101) plus \
         the 4 file-scope upvalues BEFORE the XML's `mixin=\"...\"` attributes try to resolve \
         them by name during element instantiation"
    );
}

#[test]
fn toc_raw_bytes_pin_directives() {
    let raw = std::fs::read_to_string(world_loot_toc())
        .expect("Blizzard_WorldLootObjectList TOC should read");

    for directive in [
        "## Title: Blizzard World Loot Object List",
        "## Author: Blizzard Entertainment",
        "## DefaultState: enabled",
        "## AllowLoadGameType: plunderstorm",
        "## AllowLoad: Game",
    ] {
        assert!(
            raw.contains(directive),
            "TOC must contain directive line `{directive}`"
        );
    }

    for body_file in [
        "Blizzard_WorldLootObjectList.lua",
        "Blizzard_WorldLootObjectList.xml",
    ] {
        assert!(
            raw.contains(body_file),
            "TOC must contain body file line `{body_file}`"
        );
    }

    for absent_directive in [
        "## Version:",
        "## Notes:",
        "## Dependencies:",
        "## RequiredDep:",
        "## RequiredDeps:",
        "## OptionalDeps:",
        "## SavedVariables:",
        "## LoadOnDemand:",
        "## LoadFirst:",
    ] {
        assert!(
            !raw.contains(absent_directive),
            "TOC must NOT contain `{absent_directive}` — Blizzard_WorldLootObjectList carries \
             only 5 directives + 2 body files"
        );
    }
}

#[test]
fn directory_holds_three_entries() {
    let entries = std::fs::read_dir(world_loot_dir())
        .expect("Blizzard_WorldLootObjectList directory should read")
        .count();
    assert_eq!(
        entries, 3,
        "Directory must hold exactly 3 entries (1 TOC + 1 lua + 1 xml; no flavor subdirectory, \
         no Localization.lua — the addon emits zero strings of its own, every label comes from \
         the live spell info via C_Spell.GetSpellInfo or the widget visualization info)"
    );
}

#[test]
fn excluded_from_every_screen_auto_discovery_on_mainline() {
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_WorldLootObjectList");
        assert!(
            !found,
            "Blizzard_WorldLootObjectList must NOT appear in {screen:?} eager discovery on a \
             default-mainline build — src/loader/mod.rs:527 filters out is_game_type_restricted() \
             addons. The Game-screen filter would otherwise let it through (AllowLoad:Game), but \
             the AllowLoadGameType:plunderstorm filter wins. Even Login / CharacterSelect / \
             CharacterCreate are excluded because AllowLoad:Game already rejects glue screens"
        );
    }
}

prefork_full_ui_case! {
fn loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_world_loot_object_list(env);

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_WorldLootObjectList")
                || message.contains("WorldLootObjectListMixin")
                || message.contains("WorldLootObjectListButtonMixin")
                || message.contains("WorldLootObjectList")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_WorldLootObjectList emitted addon-specific Lua errors during explicit load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn is_addon_loaded_after_explicit_call(env: &WowLuaEnv) {
    load_world_loot_object_list(env);

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_WorldLootObjectList')")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_WorldLootObjectList') must return true after the \
         explicit load_addon call — the addon's lua and XML do load, the AllowLoadGameType \
         filter only affects eager discovery, not the loader entry point itself"
    );
}
}

prefork_full_ui_case! {
fn button_mixin_publishes_with_eight_methods(env: &WowLuaEnv) {
    load_world_loot_object_list(env);

    let kind: String = env
        .eval("return type(WorldLootObjectListButtonMixin)")
        .expect("WorldLootObjectListButtonMixin probe should succeed");
    assert_eq!(
        kind, "table",
        "WorldLootObjectListButtonMixin must publish at `_G` as a table — declared at lua:8, \
         mixed onto WorldLootObjectListButtonTemplate at xml:3 (the per-row scrollbox template). \
         The mixin handles a single loot-pickup widget row in the in-range loot list overlay"
    );

    for method_name in BUTTON_MIXIN_METHODS {
        let method_kind: String = env
            .eval(&format!(
                "return type(WorldLootObjectListButtonMixin.{method_name})"
            ))
            .unwrap_or_else(|err| {
                panic!("WorldLootObjectListButtonMixin.{method_name} probe failed: {err}")
            });
        assert_eq!(
            method_kind, "function",
            "WorldLootObjectListButtonMixin.{method_name} must publish as a function"
        );
    }
}
}

prefork_full_ui_case! {
fn list_mixin_publishes_with_eight_methods(env: &WowLuaEnv) {
    load_world_loot_object_list(env);

    let kind: String = env
        .eval("return type(WorldLootObjectListMixin)")
        .expect("WorldLootObjectListMixin probe should succeed");
    assert_eq!(
        kind, "table",
        "WorldLootObjectListMixin must publish at `_G` as a table — declared at lua:101, mixed \
         onto WorldLootObjectListTemplate at xml:31 (the parent scrollbox container). The mixin \
         drives the OnLoad bootstrap (CreateScrollBoxListLinearView + EventRegistry registration \
         for WorldLootObject.ObjectShown / ObjectHidden) and the OnUpdate-driven 0.25-second \
         refresh cadence that calls C_WorldLootObject.IsWorldLootObjectInRange to repaint the \
         per-button alpha"
    );

    for method_name in LIST_MIXIN_METHODS {
        let method_kind: String = env
            .eval(&format!(
                "return type(WorldLootObjectListMixin.{method_name})"
            ))
            .unwrap_or_else(|err| {
                panic!("WorldLootObjectListMixin.{method_name} probe failed: {err}")
            });
        assert_eq!(
            method_kind, "function",
            "WorldLootObjectListMixin.{method_name} must publish as a function"
        );
    }
}
}

prefork_full_ui_case! {
fn xml_templates_are_registered(env: &WowLuaEnv) {
    load_world_loot_object_list(env);

    for template_name in VIRTUAL_TEMPLATES {
        assert!(
            wow_ui_sim::xml::get_template(template_name).is_some(),
            "{template_name} (`<Frame virtual=\"true\">` from Blizzard_WorldLootObjectList.xml) \
             must be registered in the template registry after the addon loads"
        );
    }
}
}

prefork_full_ui_case! {
fn world_loot_object_list_named_frame_publishes(env: &WowLuaEnv) {
    load_world_loot_object_list(env);

    let kind: String = env
        .eval("return type(WorldLootObjectList)")
        .expect("WorldLootObjectList probe should succeed");
    assert_eq!(
        kind, "table",
        "WorldLootObjectList must publish at `_G` as a table — declared at xml:46 with \
         `parent=\"UIParent\"` `inherits=\"WorldLootObjectListTemplate\"` `toplevel=\"true\"` \
         and a 300-wide by 500-tall size anchored LEFT at x=100. The frame is the singleton in-range loot \
         overlay shown on the left edge of the screen during plunderstorm matches when 3+ loot \
         objects come into range and hidden when fewer than 1 remain in range"
    );

    let name: String = env
        .eval("return WorldLootObjectList:GetName()")
        .expect("WorldLootObjectList:GetName() probe should succeed");
    assert_eq!(name, "WorldLootObjectList");
}
}

prefork_full_ui_case! {
fn list_template_owns_scroll_box_child_with_high_strata(env: &WowLuaEnv) {
    load_world_loot_object_list(env);

    let scroll_box_strata: String = env
        .eval("return WorldLootObjectList.ScrollBox:GetFrameStrata()")
        .expect("WorldLootObjectList.ScrollBox:GetFrameStrata probe should succeed");
    assert_eq!(
        scroll_box_strata, "HIGH",
        "WorldLootObjectList.ScrollBox must inherit frameStrata=\"HIGH\" from xml:33 — the \
         in-range loot list floats above the default UI strata so it remains visible during \
         the heads-up plunderstorm action even when other UI panels are open"
    );

    let scroll_box_hidden: bool = env
        .eval("return WorldLootObjectList.ScrollBox:IsShown()")
        .expect("WorldLootObjectList.ScrollBox:IsShown probe should succeed");
    assert!(
        !scroll_box_hidden,
        "WorldLootObjectList.ScrollBox must remain hidden after load — `hidden=\"true\"` in XML \
         and EvaluateVisibility only Show()s it once 3+ loot objects come into range \
         (WorldLootObjectListMinimumInRangeCountToShow), preventing the frame from appearing \
         empty at session start before any loot widgets have been broadcast"
    );
}
}

prefork_full_ui_case! {
fn button_template_owns_widget_display_child_with_inherited_template(env: &WowLuaEnv) {
    load_world_loot_object_list(env);

    let button_size: f64 = env
        .eval("return select(1, WorldLootObjectList:GetSize())")
        .expect("WorldLootObjectList:GetSize() probe should succeed");
    assert!(
        (button_size - 300.0).abs() < 0.001,
        "WorldLootObjectList width must equal 300 from the explicit `<Size x=\"300\" y=\"500\"/>` \
         on the named frame at xml:47 (overrides any size on the inherited template). Got \
         {button_size}"
    );

    let template_button_size_x: f64 = env
        .eval(
            "local f = CreateFrame('Frame', nil, nil, 'WorldLootObjectListButtonTemplate') \
             return select(1, f:GetSize())",
        )
        .expect("CreateFrame from WorldLootObjectListButtonTemplate probe should succeed");
    assert!(
        (template_button_size_x - 200.0).abs() < 0.001,
        "WorldLootObjectListButtonTemplate must declare `<Size x=\"200\" y=\"50\"/>` at xml:4 — \
         each row in the loot list is 200x50, with a 5px-left-anchored WidgetDisplay child \
         (UIWidgetTemplateSpellDisplay) and a Name FontString anchored to its right edge. \
         Got {template_button_size_x}"
    );
}
}
