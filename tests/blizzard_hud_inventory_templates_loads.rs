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

fn hud_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_HUDInventoryTemplates")
}

fn hud_toc() -> PathBuf {
    hud_dir().join("Blizzard_HUDInventoryTemplates.toc")
}

const HUD_INVENTORY_FILES: &[&str] = &[
    "Blizzard_HUDInventoryUtil.lua",
    "Blizzard_HUDInventoryButton.lua",
    "Blizzard_HUDInventoryButton.xml",
    "Blizzard_HUDInventory.lua",
    "Blizzard_HUDInventory.xml",
];

const ALL_MIXINS: &[&str] = &[
    "HUDInventoryButtonMixin",
    "HUDInventoryMixin",
    "HUDInventoryLayoutFrameMixin",
    "HUDInventoryBarMixin",
];

const ALL_VIRTUAL_TEMPLATES: &[&str] = &[
    "HUDInventoryButtonTemplate",
    "HUDInventoryTemplate",
    "HUDInventoryBarTemplate",
];

fn load_hud_inventory_templates(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &hud_toc())
        .expect("Blizzard_HUDInventoryTemplates should load via explicit Rust loader call");
}

fn assert_mixin_methods(env: &WowLuaEnv, mixin: &str, methods: &[&str], rationale: &str) {
    for method in methods {
        let exists: bool = env
            .eval(&format!("return type({mixin}['{method}']) == 'function'"))
            .unwrap_or_else(|err| panic!("{mixin}.{method} existence query failed: {err}"));
        assert!(exists, "{mixin} must expose `:{method}()` — {rationale}");
    }
}

#[test]
fn blizzard_hud_inventory_templates_find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&hud_dir()).expect("Blizzard_HUDInventoryTemplates TOC should resolve");
    assert_eq!(
        resolved,
        hud_toc(),
        "Blizzard_HUDInventoryTemplates ships exactly one bare TOC — Plunderstorm/wowhack-only \
         module resolves via `find_toc_file` fallthrough"
    );
}

#[test]
fn blizzard_hud_inventory_templates_toc_declares_plunderstorm_only_with_two_deps() {
    let toc = TocFile::from_file(&hud_toc()).expect("HUDInventoryTemplates TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_HUDInventoryTemplates omits `## LoadOnDemand:` — under the Plunderstorm client \
         it auto-discovers on the Game screen with `## DefaultState: enabled`; under standard \
         retail it stays unloaded because the game type filter excludes it"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert_eq!(
        toc.dependencies(),
        vec![
            "Blizzard_UIPanels_Game".to_string(),
            "Blizzard_ActionBar".to_string(),
        ],
        "Blizzard_HUDInventoryTemplates declares two `## Dependencies:` entries — \
         Blizzard_UIPanels_Game (provides UIParent + the panel docking infrastructure) and \
         Blizzard_ActionBar (provides QuickKeybindButtonTemplate / QuickKeybindButtonTemplateMixin \
         that HUDInventoryButtonTemplate inherits and HUDInventoryButtonMixin's OnEnter/OnLeave/ \
         OnClick delegate to)"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_HUDInventoryTemplates declares zero saved variables — the per-bar layout state \
         (numItemButtons, commandPrefix, bagID, startID) lives on the consuming frame's \
         KeyValues and is rebuilt every login from the bag content"
    );
}

#[test]
fn blizzard_hud_inventory_templates_toc_is_plunderstorm_or_wowhack_only_and_game_only() {
    let toc = TocFile::from_file(&hud_toc()).expect("HUDInventoryTemplates TOC should parse");
    assert!(
        toc.is_game_type_restricted(),
        "Blizzard_HUDInventoryTemplates declares `## AllowLoadGameType: plunderstorm, wowhack` — \
         neither token matches `mainline` or `standard`, so is_game_type_restricted (src/toc.rs:294) \
         returns true. The auto-discovery sweep at src/loader/mod.rs:527 filters this addon out \
         on standard retail; only Plunderstorm or wowhack clients pick it up automatically"
    );

    let raw = std::fs::read_to_string(hud_toc()).expect("HUDInventoryTemplates TOC should read");
    assert!(
        raw.contains("## AllowLoadGameType: plunderstorm, wowhack"),
        "TOC must declare `## AllowLoadGameType: plunderstorm, wowhack` — both Plunderstorm and \
         the experimental wowhack mode share the HUD inventory bar template"
    );
    assert!(
        raw.contains("## AllowLoad: Game"),
        "TOC must declare `## AllowLoad: Game` — the HUD inventory bar overlays the in-game HUD, \
         not the glue / Login / CharacterSelect screens"
    );
    assert!(
        raw.contains("## DefaultState: enabled"),
        "TOC must declare `## DefaultState: enabled` — auto-enabled on first install of the \
         Plunderstorm client, no user opt-in required"
    );
}

#[test]
fn blizzard_hud_inventory_templates_toc_lists_five_files_in_order() {
    let toc = TocFile::from_file(&hud_toc()).expect("HUDInventoryTemplates TOC should parse");
    assert_eq!(
        toc.files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        HUD_INVENTORY_FILES,
        "TOC body must list exactly these 5 files in this exact order: Util.lua loads first to \
         publish the HUDInventoryUtil namespace; Button.lua + .xml load next so \
         HUDInventoryButtonMixin + HUDInventoryButtonTemplate exist before HUDInventory.xml's \
         HUDInventoryTemplate references buttonTemplate=HUDInventoryButtonTemplate via KeyValues; \
         HUDInventory.lua + .xml load last so the bar mixins (HUDInventoryMixin / \
         HUDInventoryLayoutFrameMixin / HUDInventoryBarMixin) and the bar templates publish"
    );
}

#[test]
fn blizzard_hud_inventory_templates_directory_holds_six_entries() {
    let entries = std::fs::read_dir(hud_dir())
        .expect("HUDInventoryTemplates directory should read")
        .count();
    assert_eq!(
        entries, 6,
        "Directory must hold exactly 6 entries (5 source + 1 TOC) — no flavor subdirectory, no \
         Localization.lua, no separate fallback TOC. Strings come from the global locale table \
         (RANGE_INDICATOR is the only locale-driven literal the addon references)"
    );
}

#[test]
fn blizzard_hud_inventory_templates_excluded_from_all_screen_auto_discovery() {
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_HUDInventoryTemplates");
        assert!(
            !found,
            "Blizzard_HUDInventoryTemplates must be filtered out of auto-discovery on standard \
             retail across every ScreenKind. The TOC declares `## AllowLoadGameType: plunderstorm, \
             wowhack`, and discover_blizzard_addons_for_screen skips game-type-restricted addons \
             at src/loader/mod.rs:527 unless the active game type matches. (Screen tested: \
             {screen:?})"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_hud_inventory_templates_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_hud_inventory_templates(env);

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_HUDInventory")
                || message.contains("HUDInventoryMixin")
                || message.contains("HUDInventoryButtonMixin")
                || message.contains("HUDInventoryLayoutFrameMixin")
                || message.contains("HUDInventoryBarMixin")
                || message.contains("HUDInventoryUtil")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_HUDInventoryTemplates emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_hud_inventory_templates_is_addon_loaded_via_explicit_load(env: &WowLuaEnv) {
    load_hud_inventory_templates(env);

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HUDInventoryTemplates')")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_HUDInventoryTemplates') must return true after the \
         explicit load_addon call — confirms the loader registers the addon with the loaded-set \
         even though the auto-discovery sweep skipped it"
    );
}
}

prefork_full_ui_case! {
fn blizzard_hud_inventory_templates_dependencies_load_via_game_screen_pass(env: &WowLuaEnv) {
    load_hud_inventory_templates(env);

    let panels: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_UIPanels_Game')")
        .expect("Blizzard_UIPanels_Game probe should succeed");
    assert!(
        panels,
        "Blizzard_UIPanels_Game (the first declared dep) must auto-load via the Game-screen \
         discovery pass before the explicit HUD-inventory LoD load runs"
    );

    let actionbar: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_ActionBar')")
        .expect("Blizzard_ActionBar probe should succeed");
    assert!(
        actionbar,
        "Blizzard_ActionBar (the second declared dep, source of QuickKeybindButtonTemplate) must \
         auto-load via the Game-screen discovery pass before the explicit HUD-inventory LoD load \
         runs"
    );
}
}

prefork_full_ui_case! {
fn blizzard_hud_inventory_templates_publishes_all_four_mixins(env: &WowLuaEnv) {
    load_hud_inventory_templates(env);

    for mixin in ALL_MIXINS {
        let kind: String = env
            .eval(&format!("return type({mixin})"))
            .unwrap_or_else(|err| panic!("{mixin} type probe should succeed: {err}"));
        assert_eq!(
            kind, "table",
            "{mixin} must publish at `_G` as a table after Blizzard_HUDInventoryTemplates loads — \
             one of the addon's 4 public mixins (the button mixin in Button.lua and the 3 bar/ \
             layout mixins in HUDInventory.lua)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_hud_inventory_templates_button_mixin_exposes_lifecycle_and_action_methods(env: &WowLuaEnv) {
    load_hud_inventory_templates(env);

    assert_mixin_methods(
        &env,
        "HUDInventoryButtonMixin",
        &[
            "OnLoad",
            "OnDragStart",
            "OnReceiveDrag",
            "OnEnter",
            "OnLeave",
            "OnClick",
            "HandleModifiedClick",
            "HandleClick",
            "SetInfo",
            "SetCommandName",
            "GetBagID",
            "GetID",
            "UpdateCooldown",
            "UpdateItem",
            "UpdateHotkey",
        ],
        "HUDInventoryButtonMixin owns 15 methods: 6 XML-wired script handlers (OnLoad / \
         OnDragStart / OnReceiveDrag / OnEnter / OnLeave / OnClick) plus 9 helpers driving \
         drag-pickup, modified-click routing, cooldown / texture / hotkey refresh, and \
         per-bag/buttonID configuration",
    );
}
}

prefork_full_ui_case! {
fn blizzard_hud_inventory_templates_bar_mixin_exposes_lifecycle_and_layout_methods(env: &WowLuaEnv) {
    load_hud_inventory_templates(env);

    assert_mixin_methods(
        &env,
        "HUDInventoryMixin",
        &[
            "OnLoad",
            "OnShow",
            "OnHide",
            "OnEvent",
            "SetNumItemButtons",
            "GetNumItemButtons",
            "UseItemButton",
            "SetupItems",
            "UpdateItems",
            "SetCommandPrefix",
            "GetCommandForIndex",
            "DoQuickKeybindModeChange",
            "LayoutItemButtons",
        ],
        "HUDInventoryMixin owns 13 methods covering 4 XML-wired handlers (OnLoad / OnShow / \
         OnHide / OnEvent) plus the public bar API (SetNumItemButtons / GetNumItemButtons / \
         UseItemButton / SetupItems / UpdateItems / SetCommandPrefix / GetCommandForIndex / \
         DoQuickKeybindModeChange) and a no-op LayoutItemButtons that derived mixins override",
    );
}
}

prefork_full_ui_case! {
fn blizzard_hud_inventory_templates_layout_frame_mixin_overrides_setup_and_layout(env: &WowLuaEnv) {
    load_hud_inventory_templates(env);

    assert_mixin_methods(
        &env,
        "HUDInventoryLayoutFrameMixin",
        &["OnShow", "SetupItems", "LayoutItemButtons"],
        "HUDInventoryLayoutFrameMixin owns 3 override methods. OnShow chains \
         HUDInventoryMixin.OnShow + BaseLayoutMixin.OnShow; SetupItems chains the base SetupItems \
         then MarkDirty; LayoutItemButtons acquires buttons from the pool, calls SetInfo and \
         registers Item:CreateFromBagAndSlot continuables, then MarkDirty",
    );
}
}

prefork_full_ui_case! {
fn blizzard_hud_inventory_templates_bar_mixin_inherits_both_parent_mixins(env: &WowLuaEnv) {
    load_hud_inventory_templates(env);

    let setup_kind: String = env
        .eval("return type(HUDInventoryBarMixin.SetupItems)")
        .expect("HUDInventoryBarMixin.SetupItems probe should succeed");
    assert_eq!(
        setup_kind, "function",
        "HUDInventoryBarMixin = CreateFromMixins(HUDInventoryMixin, HUDInventoryLayoutFrameMixin) \
         must carry SetupItems (proves the second parent's override copies in — the \
         HUDInventoryLayoutFrameMixin variant runs MarkDirty after the base SetupItems)"
    );

    let layout_kind: String = env
        .eval("return type(HUDInventoryBarMixin.LayoutItemButtons)")
        .expect("HUDInventoryBarMixin.LayoutItemButtons probe should succeed");
    assert_eq!(
        layout_kind, "function",
        "HUDInventoryBarMixin must carry LayoutItemButtons — the override from \
         HUDInventoryLayoutFrameMixin (the second parent in CreateFromMixins) takes precedence \
         over the no-op LayoutItemButtons from HUDInventoryMixin"
    );

    let on_event_kind: String = env
        .eval("return type(HUDInventoryBarMixin.OnEvent)")
        .expect("HUDInventoryBarMixin.OnEvent probe should succeed");
    assert_eq!(
        on_event_kind, "function",
        "HUDInventoryBarMixin must carry OnEvent — copied from the first parent \
         HUDInventoryMixin, which is what fires UpdateItems on BAG_UPDATE / ITEM_LOCK_CHANGED / \
         BAG_NEW_ITEMS_UPDATED / UNIT_INVENTORY_CHANGED / UPDATE_BINDINGS / \
         GAME_PAD_ACTIVE_CHANGED"
    );
}
}

prefork_full_ui_case! {
fn blizzard_hud_inventory_templates_publishes_util_namespace(env: &WowLuaEnv) {
    load_hud_inventory_templates(env);

    let kind: String = env
        .eval("return type(HUDInventoryUtil)")
        .expect("HUDInventoryUtil probe should succeed");
    assert_eq!(
        kind, "table",
        "HUDInventoryUtil must publish at `_G` as a table — the file-scoped helper namespace \
         that Blizzard_HUDInventoryUtil.lua creates first so HUDInventoryMixin:OnLoad can call \
         HUDInventoryUtil.RegisterHUDElement(self) when the bar instantiates"
    );

    for fn_name in ["RegisterHUDElement", "DoQuickKeybindModeChange"] {
        let fn_kind: String = env
            .eval(&format!("return type(HUDInventoryUtil.{fn_name})"))
            .unwrap_or_else(|err| panic!("HUDInventoryUtil.{fn_name} probe failed: {err}"));
        assert_eq!(
            fn_kind, "function",
            "HUDInventoryUtil.{fn_name} must publish — the namespace exposes exactly two \
             functions: RegisterHUDElement(hudElement) appends to the file-local hudElements \
             list, DoQuickKeybindModeChange(showQuickKeybindMode) iterates that list calling \
             :DoQuickKeybindModeChange(...) on each registered HUD element"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_hud_inventory_templates_virtual_templates_stay_nil_at_global_scope(env: &WowLuaEnv) {
    load_hud_inventory_templates(env);

    for template in ALL_VIRTUAL_TEMPLATES {
        let kind: String = env
            .eval(&format!("return type(_G['{template}'])"))
            .unwrap_or_else(|err| panic!("{template} probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "{template} must NOT publish at `_G` — declared as `virtual=\"true\"` in \
             Blizzard_HUDInventory*.xml so the loader keeps it in the template registry only. \
             Consumed by other addons (or the Plunderstorm HUD layout) via `inherits=` / \
             `buttonTemplate` KeyValues, never resolved through the global scope"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_hud_inventory_templates_no_named_non_virtual_frames_publish(env: &WowLuaEnv) {
    load_hud_inventory_templates(env);

    let bar: String = env
        .eval("return type(_G['HUDInventoryBar'])")
        .expect("HUDInventoryBar probe should succeed");
    assert_eq!(
        bar, "nil",
        "Blizzard_HUDInventoryTemplates ships ZERO named non-virtual frames — every XML element \
         in HUDInventory*.xml carries `virtual=\"true\"`. The actual HUD inventory bar instance \
         is created by the Plunderstorm HUD layout addon (which inherits HUDInventoryBarTemplate); \
         this template module only registers the reusable widget shapes"
    );
}
}
