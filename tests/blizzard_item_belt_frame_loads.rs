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

fn item_belt_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_ItemBeltFrame")
}

fn item_belt_toc() -> PathBuf {
    item_belt_dir().join("Blizzard_ItemBeltFrame.toc")
}

fn hud_inventory_toc() -> PathBuf {
    blizzard_ui_dir()
        .join("Blizzard_HUDInventoryTemplates")
        .join("Blizzard_HUDInventoryTemplates.toc")
}

const ITEM_BELT_FILES: &[&str] = &["ItemBeltFrame.lua", "ItemBeltFrame.xml"];

const ITEM_BELT_DEPENDENCIES: &[&str] = &[
    "Blizzard_UIPanels_Game",
    "Blizzard_ActionBar",
    "Blizzard_HUDInventoryTemplates",
];

const ITEM_BELT_BUTTON_METHODS: &[&str] = &["UpdateHotkey", "UpdateSpectateState"];

const ITEM_BELT_FRAME_METHODS: &[&str] = &["OnShow", "OnHide", "OnEvent", "UpdateSpectateState"];

const ITEM_BELT_FRAME_KEY_VALUES: &[(&str, &str)] = &[
    ("buttonTemplate", "ItemBeltButtonTemplate"),
    ("commandPrefix", "WOWLABS_ITEM"),
];

fn load_item_belt_frame_with_dependency(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &hud_inventory_toc())
        .expect("Blizzard_HUDInventoryTemplates should load via explicit Rust loader call");
    load_addon(&env.loader_env(), &item_belt_toc())
        .expect("Blizzard_ItemBeltFrame should load via explicit Rust loader call");
}

#[test]
fn blizzard_item_belt_find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&item_belt_dir()).expect("Blizzard_ItemBeltFrame TOC should resolve");
    assert_eq!(
        resolved,
        item_belt_toc(),
        "Blizzard_ItemBeltFrame ships exactly one bare TOC — Plunderstorm-only HUD inventory \
         bar resolves via `find_toc_file` fallthrough"
    );
}

#[test]
fn blizzard_item_belt_toc_declares_three_dependencies_and_default_enabled() {
    let toc =
        TocFile::from_file(&item_belt_toc()).expect("Blizzard_ItemBeltFrame TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_ItemBeltFrame omits `## LoadOnDemand:` — under the Plunderstorm client it \
         auto-discovers on the Game screen with `## DefaultState: enabled`; under standard \
         retail it stays unloaded because the game type filter excludes it"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert_eq!(
        toc.dependencies(),
        ITEM_BELT_DEPENDENCIES
            .iter()
            .map(|s| (*s).to_string())
            .collect::<Vec<_>>(),
        "Blizzard_ItemBeltFrame declares three `## Dependencies:` entries — UIPanels_Game (panel \
         docking + UIParent infrastructure), ActionBar (provides MultiBarBottomRight the bar \
         anchors LEFT against at +4,+9), and HUDInventoryTemplates (provides \
         HUDInventoryButtonMixin / HUDInventoryBarMixin / HUDInventoryBarTemplate / \
         HUDInventoryButtonTemplate the addon's mixins extend and frames inherit)"
    );
    assert!(
        toc.optional_deps().is_empty(),
        "Blizzard_ItemBeltFrame declares zero `## OptionalDeps` — every collaborating template / \
         mixin lives in the three hard deps; nothing is conditional"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_ItemBeltFrame declares zero saved variables — backpack contents and bar size \
         re-sync from the server every login via WOW_LABS_BACKPACK_SIZE_CHANGED, no client-side \
         persistence"
    );
}

#[test]
fn blizzard_item_belt_toc_is_plunderstorm_only_and_game_only() {
    let toc =
        TocFile::from_file(&item_belt_toc()).expect("Blizzard_ItemBeltFrame TOC should parse");
    assert!(
        toc.is_game_type_restricted(),
        "Blizzard_ItemBeltFrame declares `## AllowLoadGameType: plunderstorm` — does not match \
         `mainline` or `standard`, so is_game_type_restricted (src/toc.rs:294) returns true. The \
         auto-discovery sweep at src/loader/mod.rs:527 filters this addon out on standard retail; \
         only Plunderstorm clients pick it up automatically"
    );

    let raw =
        std::fs::read_to_string(item_belt_toc()).expect("Blizzard_ItemBeltFrame TOC should read");
    assert!(
        raw.contains("## AllowLoadGameType: plunderstorm"),
        "TOC must declare `## AllowLoadGameType: plunderstorm` exactly — Plunderstorm is the only \
         game mode that surfaces a discrete item-belt HUD bar (the standard retail action bar \
         covers the same role through ActionButton + MultiBarBottomRight)"
    );
    assert!(
        raw.contains("## AllowLoad: Game"),
        "TOC must declare `## AllowLoad: Game` — the item-belt overlays the in-game HUD, not the \
         glue / Login / CharacterSelect screens"
    );
    assert!(
        raw.contains("## DefaultState: enabled"),
        "TOC must declare `## DefaultState: enabled` — auto-enabled on first install of the \
         Plunderstorm client, no user opt-in required"
    );
}

#[test]
fn blizzard_item_belt_toc_lists_two_files_with_lua_before_xml() {
    let toc =
        TocFile::from_file(&item_belt_toc()).expect("Blizzard_ItemBeltFrame TOC should parse");
    assert_eq!(
        toc.files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ITEM_BELT_FILES,
        "TOC body must list exactly these 2 files in this order: ItemBeltFrame.lua loads FIRST so \
         ItemBeltButtonMixin and ItemBeltFrameMixin publish at file scope before \
         ItemBeltFrame.xml's `mixin=\"ItemBeltButtonMixin\"` / `mixin=\"ItemBeltFrameMixin\"` \
         attributes resolve them through `_G`"
    );
}

#[test]
fn blizzard_item_belt_directory_holds_three_entries() {
    let entries = std::fs::read_dir(item_belt_dir())
        .expect("Blizzard_ItemBeltFrame directory should read")
        .count();
    assert_eq!(
        entries, 3,
        "Directory must hold exactly 3 entries (1 TOC + 1 lua + 1 xml) — no flavor subdirectory, \
         no Localization.lua. The only locale-driven literal the addon references is \
         RANGE_INDICATOR which lives in the global locale table"
    );
}

#[test]
fn blizzard_item_belt_excluded_from_every_screen_auto_discovery() {
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_ItemBeltFrame");
        assert!(
            !found,
            "Blizzard_ItemBeltFrame must be filtered out of auto-discovery on standard retail \
             across every ScreenKind. The TOC declares `## AllowLoadGameType: plunderstorm`, and \
             discover_blizzard_addons_for_screen skips game-type-restricted addons at \
             src/loader/mod.rs:527 unless the active game type matches. (Screen tested: \
             {screen:?})"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_item_belt_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_item_belt_frame_with_dependency(env);

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_ItemBeltFrame")
                || message.contains("ItemBeltFrameMixin")
                || message.contains("ItemBeltButtonMixin")
                || message.contains("ItemBeltButtonTemplate")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_ItemBeltFrame emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_item_belt_is_addon_loaded_via_explicit_load(env: &WowLuaEnv) {
    load_item_belt_frame_with_dependency(env);

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_ItemBeltFrame')")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_ItemBeltFrame') must return true after the explicit \
         load_addon call — confirms the loader registers the addon with the loaded-set even \
         though the auto-discovery sweep skipped it"
    );
}
}

prefork_full_ui_case! {
fn blizzard_item_belt_publishes_two_mixin_tables(env: &WowLuaEnv) {
    load_item_belt_frame_with_dependency(env);

    for mixin in ["ItemBeltButtonMixin", "ItemBeltFrameMixin"] {
        let kind: String = env
            .eval(&format!("return type({mixin})"))
            .unwrap_or_else(|err| panic!("{mixin} type probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "{mixin} must publish at `_G` as a table after Blizzard_ItemBeltFrame loads — \
             ItemBeltFrame.lua creates the two empty mixin tables at file scope (lines 2 and 22) \
             before binding methods to them"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_item_belt_button_mixin_carries_two_spectate_methods(env: &WowLuaEnv) {
    load_item_belt_frame_with_dependency(env);

    for method in ITEM_BELT_BUTTON_METHODS {
        let kind: String = env
            .eval(&format!("return type(ItemBeltButtonMixin['{method}'])"))
            .unwrap_or_else(|err| panic!("ItemBeltButtonMixin.{method} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "ItemBeltButtonMixin.{method} must publish as a function — the button mixin owns \
             exactly two methods that override HUDInventoryButtonMixin behavior to react to \
             spectator mode (UpdateHotkey hides the binding text and shows RANGE_INDICATOR while \
             spectating; UpdateSpectateState toggles SetEnabled and reapplies the hotkey)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_item_belt_frame_mixin_carries_four_lifecycle_methods(env: &WowLuaEnv) {
    load_item_belt_frame_with_dependency(env);

    for method in ITEM_BELT_FRAME_METHODS {
        let kind: String = env
            .eval(&format!("return type(ItemBeltFrameMixin['{method}'])"))
            .unwrap_or_else(|err| panic!("ItemBeltFrameMixin.{method} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "ItemBeltFrameMixin.{method} must publish as a function — the bar mixin owns 4 \
             methods: 3 XML-wired script handlers (OnShow registers \
             WOW_LABS_BACKPACK_SIZE_CHANGED / SPECTATE_BEGIN / SPECTATE_END via \
             FrameUtil.RegisterFrameForEvents and chains HUDInventoryBarMixin.OnShow; OnHide \
             unregisters the same set and chains HUDInventoryBarMixin.OnHide; OnEvent chains \
             HUDInventoryBarMixin.OnEvent then routes the three plunderstorm events) plus \
             UpdateSpectateState which fans out :UpdateSpectateState() across every active button \
             from itemButtonPool"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_item_belt_named_frame_publishes_with_inherits_and_mixin_chain(env: &WowLuaEnv) {
    load_item_belt_frame_with_dependency(env);

    let kind: String = env
        .eval("return type(ItemBeltFrame)")
        .expect("ItemBeltFrame probe should succeed");
    assert_eq!(
        kind, "table",
        "ItemBeltFrame must publish at `_G` as a table — declared at ItemBeltFrame.xml:7 with \
         `name=\"ItemBeltFrame\"` `parent=\"UIParent\"` `frameStrata=\"LOW\"` \
         `inherits=\"HUDInventoryBarTemplate\"` `mixin=\"ItemBeltFrameMixin\"` \
         `toplevel=\"true\"` `movable=\"true\"`"
    );

    let name: String = env
        .eval("return ItemBeltFrame:GetName()")
        .expect("ItemBeltFrame:GetName() probe should succeed");
    assert_eq!(name, "ItemBeltFrame");

    let strata: String = env
        .eval("return ItemBeltFrame:GetFrameStrata()")
        .expect("ItemBeltFrame:GetFrameStrata() probe should succeed");
    assert_eq!(
        strata, "LOW",
        "ItemBeltFrame must report LOW strata — declared `frameStrata=\"LOW\"` so the bar sits \
         beneath the standard MEDIUM-strata UIPanel surface (the in-combat pickup HUD must not \
         occlude tooltips / dialogs)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_item_belt_button_template_stays_nil_at_global_scope(env: &WowLuaEnv) {
    load_item_belt_frame_with_dependency(env);

    let kind: String = env
        .eval("return type(_G['ItemBeltButtonTemplate'])")
        .expect("ItemBeltButtonTemplate probe should succeed");
    assert_eq!(
        kind, "nil",
        "ItemBeltButtonTemplate must NOT publish at `_G` — declared as `virtual=\"true\"` at \
         ItemBeltFrame.xml:4 so the loader keeps it in the template registry only. Consumed by \
         ItemBeltFrame's `buttonTemplate=\"ItemBeltButtonTemplate\"` KeyValue, never resolved \
         through the global scope"
    );
}
}

prefork_full_ui_case! {
fn blizzard_item_belt_frame_carries_three_string_and_number_key_values(env: &WowLuaEnv) {
    load_item_belt_frame_with_dependency(env);

    for (key, expected) in ITEM_BELT_FRAME_KEY_VALUES {
        let actual: String = env
            .eval(&format!("return tostring(ItemBeltFrame['{key}'])"))
            .unwrap_or_else(|err| panic!("ItemBeltFrame.{key} probe failed: {err}"));
        assert_eq!(
            actual, *expected,
            "ItemBeltFrame.{key} must equal {expected:?} — declared via \
             `<KeyValue key=\"{key}\" value=\"{expected}\" type=\"string\"/>` at \
             ItemBeltFrame.xml so HUDInventoryBarTemplate's pool / command routing reads the \
             template name and command prefix from instance state"
        );
    }

    let base_count: f64 = env
        .eval("return ItemBeltFrame.baseNumItemButtons")
        .expect("ItemBeltFrame.baseNumItemButtons probe should succeed");
    assert_eq!(
        base_count, 2.0,
        "ItemBeltFrame.baseNumItemButtons must equal 2 — declared via \
         `<KeyValue key=\"baseNumItemButtons\" value=\"2\" type=\"number\"/>` so \
         HUDInventoryMixin:OnLoad spawns 2 buttons by default before \
         WOW_LABS_BACKPACK_SIZE_CHANGED fires the live size update"
    );
}
}
