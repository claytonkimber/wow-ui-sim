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

fn bag_buttons_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_MainMenuBarBagButtons")
}

fn bag_buttons_toc() -> PathBuf {
    bag_buttons_dir().join("Blizzard_MainMenuBarBagButtons_Mainline.toc")
}

const BAG_BUTTONS_TOC_FILES: &[&str] = &[
    "Shared/MainMenuBarBagManager.lua",
    "Mainline/MainMenuBarBagButtons.lua",
    "Shared/BagsBar.lua",
    "Mainline/MainMenuBarBagButtons.xml",
];

const MAIN_MENU_BAR_BAG_MANAGER_METHODS: &[&str] = &[
    "Init",
    "RegisterBagButton",
    "EnumerateBagButtons",
    "ToggleExpandBar",
    "SetExpandBar",
    "SetExpandBarAuto",
    "ShouldBarExpand",
    "IsBarUserExpanded",
    "OnCursorChanged",
    "OnExpandBarChanged",
];

const BASE_BAG_SLOT_BUTTON_MIXIN_METHODS: &[&str] = &[
    "BagSlotOnLoad",
    "OnLoadInternal",
    "BagSlotOnEvent",
    "BagSlotOnShow",
    "BagSlotOnHide",
    "BagSlotOnClick",
    "PutItemInBag",
    "BagSlotOnDragStart",
    "BagSlotOnReceiveDrag",
    "BagSlotOnEnter",
    "OnEnterInternal",
    "BagSlotOnLeave",
    "UpdateBagMatchesSearch",
    "UpdateBagButtonHighlight",
    "GetItemContextMatchResult",
    "GetIsBarExpanded",
    "GetBagID",
    "HasBagEquipped",
    "IsBackpack",
    "GetSlotAtlases",
    "UpdateItemContextOverlayTextures",
    "UpdateTextures",
    "SetItemButtonTexture",
    "SetItemButtonQuality",
    "SetBarExpanded",
];

const MAIN_MENU_BAR_BACKPACK_MIXIN_OWN_METHODS: &[&str] = &[
    "BagSlotOnShow",
    "BagSlotOnHide",
    "OnLoadInternal",
    "OnEnterInternal",
    "PutItemInBag",
    "HasBagEquipped",
    "BackpackOnEvent",
    "UpdateFreeSlots",
    "SetCountShown",
    "OnBagUpdate",
    "OnPlayerEnteringWorld",
    "OnAzeriteEmpoweredItemLooted",
    "IsBackpack",
    "GetSlotAtlases",
    "UpdateItemContextOverlayTextures",
    "SetBarExpanded",
    "BagSlotOnDragStart",
];

const BAG_BAR_EXPAND_TOGGLE_METHODS: &[&str] = &["OnClick", "GetRotation", "UpdateOrientation"];

const BAGS_BAR_MIXIN_METHODS: &[&str] = &[
    "OnLoad",
    "GetBagButtonAnchorPoints",
    "GetBagBarLength",
    "Layout",
    "IsHorizontal",
    "IsDirectionLeft",
    "IsDirectionUp",
    "MainActionBarStateOverridden",
];

const NAMED_BAG_FRAMES: &[&str] = &[
    "BagsBar",
    "MainMenuBarBackpackButton",
    "BagBarExpandToggle",
    "CharacterBag0Slot",
    "CharacterBag1Slot",
    "CharacterBag2Slot",
    "CharacterBag3Slot",
    "CharacterReagentBag0Slot",
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
fn blizzard_main_menu_bar_bag_buttons_find_toc_resolves_mainline_variant() {
    let resolved =
        find_toc_file(&bag_buttons_dir()).expect("Blizzard_MainMenuBarBagButtons TOC resolves");
    assert_eq!(
        resolved,
        bag_buttons_toc(),
        "Blizzard_MainMenuBarBagButtons ships flavor-suffixed TOCs (_Mainline + _Classic). \
         `find_toc_file` resolves the Mainline variant first per src/loader/mod.rs:67-69 \
         (Mainline preferred, then bare, then any non-Classic). The bag-bar UI differs \
         significantly between retail and classic flavors (retail has a 4-slot equippable + \
         1 reagent bag + expand-toggle; classic has a keyring system instead) so each flavor \
         ships its own TOC + Mainline / Classic / Mists / Wrath subdirectories"
    );
}

#[test]
fn blizzard_main_menu_bar_bag_buttons_toc_declares_default_state_enabled_with_allow_load_game() {
    let toc = TocFile::from_file(&bag_buttons_toc()).expect("Mainline TOC parses");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_MainMenuBarBagButtons omits `## LoadOnDemand:` — `## DefaultState: enabled` \
         makes it eager-load. Bag buttons must be live the moment the player logs into the \
         world (the backpack icon is visible at the very first PLAYER_ENTERING_WORLD event), \
         so deferred load is not viable"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert!(
        toc.dependencies().is_empty(),
        "Mainline TOC declares zero `## Dependencies:` — relies only on globals from the \
         eager-loaded baseline (PaperDollItemSlotButton_OnLoad / EventRegistry / \
         EventUtil.ContinueOnVariablesLoaded / Enum.UICursorType / Enum.BagsDirection / \
         Enum.GameRule.BagsUIDisabled / C_Container / ContainerFrameSettingsManager / \
         ItemButtonMixin / GameTooltip / KeybindFrames_InQuickKeybindMode / etc) supplied by \
         Blizzard_SharedXML / Blizzard_UIParent / Blizzard_ContainerFrame which are themselves \
         eager. Classic TOC carries `## Dependencies: Blizzard_CharacterFrame` for the keyring \
         path, but Mainline doesn't need it"
    );
    assert!(toc.optional_deps().is_empty());
    assert!(
        toc.saved_variables().is_empty(),
        "Zero saved variables — bag-bar visibility is driven by the `expandBagBar` CVar (read \
         via GetCVarBool) on VARIABLES_LOADED, not by addon SVs. CVar persistence is engine-\
         managed, so no SavedVariables blob is needed"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "TOC declares `## AllowLoadGameType: mainline` — `is_game_type_restricted` returns \
         false because mainline is one of the non-restricted values at src/toc.rs:294-302. \
         The mainline-only marker excludes this TOC from Classic flavors which load the \
         _Classic.toc instead"
    );

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "Mainline TOC declares `## AllowLoad: Game` — must auto-discover on the Game screen \
         only (the bag bar is part of the in-world action-bar UI, never on glue screens)"
    );
    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            !toc.allows_screen(screen),
            "Bag-buttons must NOT auto-discover on glue screen {screen:?} — \
             `## AllowLoad: Game` matches only `ScreenKind::Game` at src/toc.rs:308"
        );
    }
}

#[test]
fn blizzard_main_menu_bar_bag_buttons_toc_lists_four_files_in_load_order() {
    let toc = TocFile::from_file(&bag_buttons_toc()).expect("Mainline TOC parses");
    assert_eq!(
        toc.files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        BAG_BUTTONS_TOC_FILES,
        "Mainline TOC body lists exactly 4 files in load order: \
         Shared/MainMenuBarBagManager.lua first (publishes the cross-flavor MainMenuBarBagManager \
         singleton with bag-button registry + expand-bar state machine), \
         Mainline/MainMenuBarBagButtons.lua second (4 mixins: BagSlotItemFlyInMixin, \
         BaseBagSlotButtonMixin, MainMenuBarBackpackMixin via CreateFromMixins(BaseBagSlotButtonMixin), \
         CharacterReagentBagMixin, BagBarExpandToggleMixin), \
         Shared/BagsBar.lua third (BagsBarMixin orientation + Layout dispatcher), \
         Mainline/MainMenuBarBagButtons.xml last (BaseBagSlotButtonTemplate virtual + BagsBar Frame \
         + 8 named child frames). The Lua precedes the XML so the mixins are bound before the \
         XML parser hits `mixin=` attributes"
    );
}

#[test]
fn blizzard_main_menu_bar_bag_buttons_directory_holds_five_entries() {
    let entries = std::fs::read_dir(bag_buttons_dir())
        .expect("Blizzard_MainMenuBarBagButtons directory reads")
        .count();
    assert_eq!(
        entries, 5,
        "Directory holds exactly 5 entries — 2 flavor TOCs + 3 source subdirectories \
         (Mainline / Mists / Shared). Classic TOC pulls the Classic + Mists subdirs alongside \
         Shared; Mainline TOC only consumes Mainline + Shared"
    );
}

#[test]
fn blizzard_main_menu_bar_bag_buttons_auto_discovered_on_game_screen_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let game_found = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_MainMenuBarBagButtons");
    assert!(
        game_found,
        "Blizzard_MainMenuBarBagButtons must be auto-discovered on the Game screen — the \
         eager-load combo (`## DefaultState: enabled` + `## AllowLoad: Game` + \
         `## AllowLoadGameType: mainline`) routes the Mainline TOC into the eager `addons` set \
         during Game-screen discovery"
    );

    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_MainMenuBarBagButtons");
        assert!(
            !found,
            "Bag-buttons must NOT be auto-discovered on glue screen {screen:?} — \
             `## AllowLoad: Game` excludes it from glue discovery sweeps"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_main_menu_bar_bag_buttons_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_MainMenuBarBagButtons")
                || message.contains("MainMenuBarBagManager")
                || message.contains("MainMenuBarBagButtons")
                || message.contains("BagsBar")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_MainMenuBarBagButtons emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_main_menu_bar_bag_buttons_is_addon_loaded_after_auto_discovery(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_MainMenuBarBagButtons')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_MainMenuBarBagButtons') must return true after the \
         eager auto-discovery sweep — proves the bag-buttons addon registers with the \
         loaded-set during the standard Game-screen boot pipeline"
    );
}
}

prefork_full_ui_case! {
fn blizzard_main_menu_bar_bag_manager_publishes_with_methods(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(MainMenuBarBagManager)")
        .expect("MainMenuBarBagManager type probe succeeds");
    assert_eq!(
        kind, "table",
        "MainMenuBarBagManager must publish at `_G` as a table — declared at line 1 of \
         Shared/MainMenuBarBagManager.lua. Acts as the cross-flavor singleton: \
         maintains the `allBagButtons` registry, owns the expand-bar state, hooks \
         VARIABLES_LOADED + EXPAND_BAG_BAR_CHANGED + CURSOR_CHANGED via EventRegistry, \
         dispatches OnExpandBarChanged via TriggerEvent('MainMenuBarManager.OnExpandChanged')"
    );

    for method in MAIN_MENU_BAR_BAG_MANAGER_METHODS {
        let method_kind: String = env
            .eval(&format!("return type(MainMenuBarBagManager.{method})"))
            .unwrap_or_else(|err| panic!("MainMenuBarBagManager.{method} probe failed: {err}"));
        assert_eq!(
            method_kind, "function",
            "MainMenuBarBagManager.{method} must be a function — drives the bag-button registry \
             + expand-bar state machine. Init runs at file scope (line 79: \
             `MainMenuBarBagManager:Init()`), so all methods must be bound before that call"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_main_menu_bar_bag_manager_starts_with_empty_button_registry(env: &WowLuaEnv) {

    let registry_kind: String = env
        .eval("return type(MainMenuBarBagManager.allBagButtons)")
        .expect("allBagButtons probe succeeds");
    assert_eq!(
        registry_kind, "table",
        "MainMenuBarBagManager.allBagButtons must publish as a table — initialized in \
         MainMenuBarBagManager:Init at line 4 (`self.allBagButtons = {{}}`). Each \
         BaseBagSlotButtonMixin:BagSlotOnLoad call inserts the button via RegisterBagButton, \
         which is the canonical iteration source for the BagsBar:Layout call"
    );

    let bag_count: i64 = env
        .eval("return #MainMenuBarBagManager.allBagButtons")
        .expect("allBagButtons count probe succeeds");
    assert_eq!(
        bag_count, 6,
        "MainMenuBarBagManager.allBagButtons must hold exactly 6 entries after BagSlotOnLoad \
         fires for every named bag button — MainMenuBarBackpackButton + CharacterBag0Slot..3 + \
         CharacterReagentBag0Slot. The `tContains` guard at line 18 prevents duplicate inserts \
         on re-OnLoad"
    );
}
}

prefork_full_ui_case! {
fn blizzard_base_bag_slot_button_mixin_publishes_with_methods(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(BaseBagSlotButtonMixin)")
        .expect("BaseBagSlotButtonMixin type probe succeeds");
    assert_eq!(
        kind, "table",
        "BaseBagSlotButtonMixin must publish at `_G` as a table — declared at \
         Mainline/MainMenuBarBagButtons.lua:11. The base mixin for every bag slot button; \
         MainMenuBarBackpackMixin extends it via CreateFromMixins at line 222"
    );

    for method in BASE_BAG_SLOT_BUTTON_MIXIN_METHODS {
        let method_kind: String = env
            .eval(&format!("return type(BaseBagSlotButtonMixin.{method})"))
            .unwrap_or_else(|err| panic!("BaseBagSlotButtonMixin.{method} probe failed: {err}"));
        assert_eq!(
            method_kind, "function",
            "BaseBagSlotButtonMixin.{method} must be a function — XML script handlers in \
             BaseBagSlotButtonTemplate (MainMenuBarBagButtons.xml lines 31-41) reference these \
             via `<OnLoad method=\"BagSlotOnLoad\"/>`, so they MUST resolve at the point the \
             parser binds the handler"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_main_menu_bar_backpack_mixin_inherits_base_via_create_from_mixins(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(MainMenuBarBackpackMixin)")
        .expect("MainMenuBarBackpackMixin type probe succeeds");
    assert_eq!(
        kind, "table",
        "MainMenuBarBackpackMixin must publish at `_G` as a table — declared at line 222 via \
         `CreateFromMixins(BaseBagSlotButtonMixin)`, which copies all base methods onto the \
         new table. The backpack overrides BagSlotOnShow / BagSlotOnHide / OnLoadInternal / \
         etc to add backpack-specific behavior (free-slot count, Azerite tutorial, \
         remains-shown-on-collapse)"
    );

    for method in MAIN_MENU_BAR_BACKPACK_MIXIN_OWN_METHODS {
        let method_kind: String = env
            .eval(&format!("return type(MainMenuBarBackpackMixin.{method})"))
            .unwrap_or_else(|err| panic!("MainMenuBarBackpackMixin.{method} probe failed: {err}"));
        assert_eq!(
            method_kind, "function",
            "MainMenuBarBackpackMixin.{method} must be a function — own methods declared after \
             the CreateFromMixins call shadow the base mixin's implementation"
        );
    }

    let inherited_kind: String = env
        .eval("return type(MainMenuBarBackpackMixin.BagSlotOnLoad)")
        .expect("inherited method probe succeeds");
    assert_eq!(
        inherited_kind, "function",
        "MainMenuBarBackpackMixin.BagSlotOnLoad must inherit from BaseBagSlotButtonMixin — \
         `CreateFromMixins(BaseBagSlotButtonMixin)` shallow-copies all base methods onto the \
         new table, including the un-overridden BagSlotOnLoad / BagSlotOnEvent / etc. \
         MainMenuBarBackpackButton's OnLoad XML handler still invokes BagSlotOnLoad which is \
         only inherited, never re-defined"
    );
}
}

prefork_full_ui_case! {
fn blizzard_character_reagent_bag_mixin_publishes_with_overrides(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(CharacterReagentBagMixin)")
        .expect("CharacterReagentBagMixin type probe succeeds");
    assert_eq!(
        kind, "table",
        "CharacterReagentBagMixin must publish at `_G` as a table — declared at line 382. \
         CharacterReagentBag0Slot mixes this in alongside BaseBagSlotButtonTemplate \
         (MainMenuBarBagButtons.xml:127) to override GetSlotAtlases (returns reagent-specific \
         atlases) and SetBarExpanded (no-op so the reagent bag stays visible regardless of \
         expand state)"
    );

    let get_atlases_kind: String = env
        .eval("return type(CharacterReagentBagMixin.GetSlotAtlases)")
        .expect("GetSlotAtlases probe succeeds");
    assert_eq!(
        get_atlases_kind, "function",
        "CharacterReagentBagMixin.GetSlotAtlases must shadow the base implementation — \
         returns 'bag-reagent-border' / 'bag-reagent-border-empty' / 'bag-border-highlight' \
         instead of the standard 'bag-border' atlases"
    );

    let set_bar_expanded_kind: String = env
        .eval("return type(CharacterReagentBagMixin.SetBarExpanded)")
        .expect("SetBarExpanded probe succeeds");
    assert_eq!(
        set_bar_expanded_kind, "function",
        "CharacterReagentBagMixin.SetBarExpanded must shadow the base — the reagent bag stays \
         visible regardless of expand-bar state (no-op override)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_bag_bar_expand_toggle_mixin_publishes_with_methods(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(BagBarExpandToggleMixin)")
        .expect("BagBarExpandToggleMixin type probe succeeds");
    assert_eq!(
        kind, "table",
        "BagBarExpandToggleMixin must publish at `_G` as a table — declared at line 392. \
         The BagBarExpandToggle button mixes it in (MainMenuBarBagButtons.xml:68) to drive \
         the expand-bar arrow rotation in 4 orientations (left/right horizontal + up/down \
         vertical) via OnClick + GetRotation + UpdateOrientation"
    );

    for method in BAG_BAR_EXPAND_TOGGLE_METHODS {
        let method_kind: String = env
            .eval(&format!("return type(BagBarExpandToggleMixin.{method})"))
            .unwrap_or_else(|err| panic!("BagBarExpandToggleMixin.{method} probe failed: {err}"));
        assert_eq!(
            method_kind, "function",
            "BagBarExpandToggleMixin.{method} must be a function — drives the toggle's \
             4-orientation arrow rotation"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_bag_slot_item_fly_in_mixin_publishes(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(BagSlotItemFlyInMixin)")
        .expect("BagSlotItemFlyInMixin type probe succeeds");
    assert_eq!(
        kind, "table",
        "BagSlotItemFlyInMixin must publish at `_G` as a table — declared at line 1. \
         The XML AnimationGroup `parentKey=\"FlyIn\"` mixes it in to drive the ITEM_PUSH \
         fly-in animation (icon scales from 0.125 to 1, slides along a 2-control-point \
         SMOOTH curve)"
    );

    let on_play_kind: String = env
        .eval("return type(BagSlotItemFlyInMixin.OnPlay)")
        .expect("OnPlay probe succeeds");
    assert_eq!(
        on_play_kind, "function",
        "BagSlotItemFlyInMixin.OnPlay must be a function — shows the parent's AnimIcon when \
         the fly-in animation starts (line 3-5)"
    );

    let on_finished_kind: String = env
        .eval("return type(BagSlotItemFlyInMixin.OnFinished)")
        .expect("OnFinished probe succeeds");
    assert_eq!(
        on_finished_kind, "function",
        "BagSlotItemFlyInMixin.OnFinished must be a function — hides the parent's AnimIcon \
         when the fly-in animation ends (line 7-9)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_bags_bar_mixin_publishes_with_methods(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(BagsBarMixin)")
        .expect("BagsBarMixin type probe succeeds");
    assert_eq!(
        kind, "table",
        "BagsBarMixin must publish at `_G` as a table — declared at Shared/BagsBar.lua:2. \
         The BagsBar Frame mixes it in (MainMenuBarBagButtons.xml:44) to drive the bag-bar's \
         orientation-aware Layout dispatch (horizontal vs vertical, left/right vs up/down)"
    );

    for method in BAGS_BAR_MIXIN_METHODS {
        let method_kind: String = env
            .eval(&format!("return type(BagsBarMixin.{method})"))
            .unwrap_or_else(|err| panic!("BagsBarMixin.{method} probe failed: {err}"));
        assert_eq!(
            method_kind, "function",
            "BagsBarMixin.{method} must be a function — drives the bag-bar's orientation \
             + Layout dispatch chain"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_calculate_total_number_of_free_bag_slots_is_owned_by_c_container(env: &WowLuaEnv) {

    let kinds: (String, String) = env
        .eval(
            "return type(C_Container.CalculateTotalNumberOfFreeBagSlots), \
                    type(CalculateTotalNumberOfFreeBagSlots)",
        )
        .expect("free-bag-slot owner probe succeeds");
    assert_eq!(
        kinds,
        ("function".to_string(), "nil".to_string()),
        "MainMenuBarBackpackMixin:UpdateFreeSlots calls the current \
         C_Container.CalculateTotalNumberOfFreeBagSlots API. MainMenuBarBagButtons.lua no \
         longer publishes a same-named global helper"
    );
}
}

prefork_full_ui_case! {
fn blizzard_backpack_button_on_modified_click_publishes(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(BackpackButton_OnModifiedClick)")
        .expect("BackpackButton_OnModifiedClick probe succeeds");
    assert_eq!(
        kind, "function",
        "BackpackButton_OnModifiedClick must publish at `_G` as a function — declared at \
         line 166. Used by the in-game OPENALLBAGS modified-click binding to toggle every bag \
         simultaneously (Shift-click on the backpack)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_named_bag_frames_resolve_globally(env: &WowLuaEnv) {

    for name in NAMED_BAG_FRAMES {
        let exists: bool = env
            .eval(&format!("return _G[{name:?}] ~= nil"))
            .unwrap_or_else(|err| panic!("{name} existence probe failed: {err}"));
        assert!(
            exists,
            "{name} must publish at `_G` after addon load — declared with `name=\"...\"` in \
             Mainline/MainMenuBarBagButtons.xml. Missing implies the XML parser dropped the \
             element or the named frame creation pipeline didn't register the global"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_bags_bar_anchored_to_micro_button_and_bags_bar(env: &WowLuaEnv) {

    let parent_name: String = env
        .eval("return BagsBar:GetParent():GetName()")
        .expect("BagsBar:GetParent():GetName() probe succeeds");
    assert_eq!(
        parent_name, "UIParent",
        "BagsBar must parent to UIParent — XML declares `parent=\"UIParent\"` at \
         MainMenuBarBagButtons.xml:44. UIParent is the canonical scaling root for in-game UI"
    );

    let key_values_horizontal: bool = env
        .eval("return BagsBar.isHorizontal == true")
        .expect("BagsBar.isHorizontal probe succeeds");
    assert!(
        key_values_horizontal,
        "BagsBar.isHorizontal must equal true — declared via XML KeyValues `isHorizontal=true` \
         (line 50). The retail bag bar is always horizontal; vertical orientation is reserved \
         for Classic / EditMode reflow"
    );
}
}

prefork_full_ui_case! {
fn blizzard_main_menu_bar_backpack_button_anchored_inside_bags_bar(env: &WowLuaEnv) {

    let parent_name: String = env
        .eval("return MainMenuBarBackpackButton:GetParent():GetName()")
        .expect("MainMenuBarBackpackButton:GetParent():GetName() probe succeeds");
    assert_eq!(
        parent_name, "BagsBar",
        "MainMenuBarBackpackButton must parent to BagsBar — declared as a child Frame inside \
         BagsBar's `<Frames>` block (XML line 55). The backpack is the canonical anchor for \
         the rest of the bag-button strip; CharacterBag0Slot..3 + CharacterReagentBag0Slot \
         all RIGHT-anchor relative to it via the BagBarExpandToggle chain"
    );

    let backpack_id: i64 = env
        .eval("return MainMenuBarBackpackButton:GetID()")
        .expect("MainMenuBarBackpackButton:GetID() probe succeeds");
    assert_eq!(
        backpack_id, 0,
        "MainMenuBarBackpackButton:GetID() must return 0 — XML declares `id=\"0\"` at \
         MainMenuBarBagButtons.xml:55. The 0 ID corresponds to Enum.BagIndex.Backpack which \
         GetBagID returns directly (early-out at line 154)"
    );
}
}
