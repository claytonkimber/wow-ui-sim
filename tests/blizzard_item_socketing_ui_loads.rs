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

fn item_socketing_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_ItemSocketingUI")
}

fn item_socketing_toc() -> PathBuf {
    item_socketing_dir().join("Blizzard_ItemSocketingUI.toc")
}

const ITEM_SOCKETING_FILES: &[&str] = &[
    "Blizzard_ItemSocketingUI_Bootstrap.lua",
    "Blizzard_ItemSocketingUI.xml",
    "Localization.lua",
];

const SOCKET_BUTTON_METHODS: &[&str] = &[
    "OnLoad",
    "ClickSocketButton",
    "OnClick",
    "OnReceiveDrag",
    "OnDragStart",
    "OnEnter",
    "OnLeave",
    "OnEvent",
];

const SOCKETING_FRAME_METHODS: &[&str] = &[
    "OnLoad",
    "OnShow",
    "OnHide",
    "RegisterEvents",
    "UnregisterEvents",
    "OnEvent",
    "Update",
    "DisableSockets",
    "EnableSockets",
];

const FREE_HELPER_FUNCTIONS: &[&str] = &[
    "ItemSocketingFrame_OnLoad",
    "ItemSocketingFrame_OnEvent",
    "ItemSocketingFrame_Update",
    "ItemSocketingSocketButton_OnScrollRangeChanged",
];

const VIRTUAL_TEMPLATE_NAMES: &[&str] = &[
    "NubTemplate",
    "GenericSocketButtonTemplate",
    "GenericItemSocketingFrameTemplate",
];

fn load_item_socketing_ui(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &item_socketing_toc())
        .expect("Blizzard_ItemSocketingUI should load via explicit Rust loader call");
}

#[test]
fn blizzard_item_socketing_find_toc_resolves_bare_variant() {
    let resolved =
        find_toc_file(&item_socketing_dir()).expect("Blizzard_ItemSocketingUI TOC should resolve");
    assert_eq!(
        resolved,
        item_socketing_toc(),
        "Blizzard_ItemSocketingUI ships exactly one bare TOC — the LoadOnDemand gem-socketing \
         module resolves via `find_toc_file` fallthrough after the `_Mainline.toc` lookup misses"
    );
}

#[test]
fn blizzard_item_socketing_toc_declares_load_on_demand_with_animated_shine_dependency() {
    let toc = TocFile::from_file(&item_socketing_toc())
        .expect("Blizzard_ItemSocketingUI TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_ItemSocketingUI declares `## LoadOnDemand: 1` — the addon stays unloaded until \
         the SOCKET_INFO_UPDATE event fires (server-driven, triggered when the player right-clicks \
         a socketable item) and ItemSocketingFrame_OnEvent calls ItemSocketingFrame_LoadUI"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert_eq!(
        toc.dependencies(),
        vec!["Blizzard_AnimatedShine".to_string()],
        "Blizzard_ItemSocketingUI declares Blizzard_AnimatedShine for its socket animation \
         templates"
    );
    assert!(
        toc.optional_deps().is_empty(),
        "Blizzard_ItemSocketingUI declares zero `## OptionalDeps` — every API it touches \
         (C_ItemSocketInfo, FrameUtil.RegisterFrameForEvents, StaticPopup_Show / Hide, \
         CVarCallbackRegistry, SetDesaturation) is provided unconditionally by the foundational \
         addon set"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_ItemSocketingUI declares zero saved variables — socket state is fully server- \
         driven (C_ItemSocketInfo.GetSocketItemInfo / GetSocketTypes / GetNewSocketInfo / \
         GetExistingSocketInfo) and resets every time the panel opens; no client-side persistence"
    );
}

#[test]
fn blizzard_item_socketing_toc_omits_allow_load_metadata() {
    let toc = TocFile::from_file(&item_socketing_toc())
        .expect("Blizzard_ItemSocketingUI TOC should parse");
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_ItemSocketingUI omits `## AllowLoadGameType:` — `is_game_type_restricted` \
         (src/toc.rs:294-302) returns false when the metadata key is missing, so retail clients \
         keep the addon eligible. Gem socketing has shipped continuously since The Burning Crusade \
         and is part of the mainline retail experience"
    );

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "Blizzard_ItemSocketingUI omits `## AllowLoad:` — `allows_screen` (src/toc.rs:311) \
         defaults missing AllowLoad to Game-only. The socketing panel is an in-game item-action \
         interface (right-click a gem-bearing item to open) so glue-screen access is meaningless"
    );

    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            !toc.allows_screen(screen),
            "Blizzard_ItemSocketingUI must NOT load on glue screens — missing AllowLoad defaults \
             to Game-only at src/toc.rs:311. (Screen tested: {screen:?})"
        );
    }

    let raw = std::fs::read_to_string(item_socketing_toc())
        .expect("Blizzard_ItemSocketingUI TOC should read");
    assert!(
        raw.contains("## LoadOnDemand: 1"),
        "TOC must declare `## LoadOnDemand: 1` exactly — the on-demand panel stays out of the \
         Game-screen auto-discovery sweep until the trigger fires"
    );
    assert!(
        !raw.contains("## AllowLoad"),
        "TOC must NOT declare any `## AllowLoad*` metadata — relying on the Game-only default \
         keeps the on-demand panel out of glue-screen sweeps without an explicit allow"
    );
    assert!(
        raw.contains("## Dependencies: Blizzard_AnimatedShine"),
        "TOC must declare its Blizzard_AnimatedShine dependency"
    );
}

#[test]
fn blizzard_item_socketing_toc_lists_bootstrap_xml_and_localization() {
    let toc = TocFile::from_file(&item_socketing_toc())
        .expect("Blizzard_ItemSocketingUI TOC should parse");
    assert_eq!(
        toc.files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ITEM_SOCKETING_FILES,
        "TOC body lists Bootstrap, XML, and Localization in current retail order. The .lua sibling Blizzard_ItemSocketingUI.lua is loaded by the XML's \
         `<Script file=\"Blizzard_ItemSocketingUI.lua\"/>` directive at xml line 3 BEFORE any \
         frame element is parsed, so both mixin tables (GenericSocketButtonMixin, \
         GenericItemSocketingFrameMixin) and the 4 free helpers (ItemSocketingFrame_OnLoad, \
         ItemSocketingFrame_OnEvent, ItemSocketingFrame_Update, \
         ItemSocketingSocketButton_OnScrollRangeChanged) and the UIPanelWindows entry publish at \
         file scope before any `mixin=\"...\"` or `<OnLoad function=\"...\"/>` resolves them \
         through `_G`. Localization.lua is intentionally a single-comment stub — the locale \
         literal `_G[strupper(gemColor) .. \"_GEM\"]` it could populate already lives in the \
         global locale table"
    );
}

#[test]
fn blizzard_item_socketing_directory_holds_five_entries() {
    let entries = std::fs::read_dir(item_socketing_dir())
        .expect("Blizzard_ItemSocketingUI directory should read")
        .count();
    assert_eq!(
        entries, 5,
        "Directory holds the TOC, Bootstrap, Lua, XML, and Localization files"
    );
}

#[test]
fn blizzard_item_socketing_excluded_from_every_screen_auto_discovery() {
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_ItemSocketingUI");
        assert!(
            !found,
            "Blizzard_ItemSocketingUI must be filtered out of auto-discovery on every \
             ScreenKind. The TOC declares `## LoadOnDemand: 1`, and \
             discover_blizzard_addons_for_screen routes LoD addons into the lod_pool \
             (src/loader/mod.rs:530-535) rather than the eager `addons` set. \
             (Screen tested: {screen:?})"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_item_socketing_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_item_socketing_ui(env);

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_ItemSocketingUI")
                || message.contains("GenericSocketButtonMixin")
                || message.contains("GenericItemSocketingFrameMixin")
                || message.contains("ItemSocketingFrame")
                || message.contains("ItemSocketingDescription")
                || message.contains("ItemSocketingScrollFrame")
                || message.contains("ItemSocketingScrollChild")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_ItemSocketingUI emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_item_socketing_is_addon_loaded_via_explicit_load(env: &WowLuaEnv) {
    load_item_socketing_ui(env);

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_ItemSocketingUI')")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_ItemSocketingUI') must return true after the explicit \
         load_addon call — confirms the loader registers the addon with the loaded-set even \
         though the auto-discovery sweep skipped it (LoadOnDemand)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_item_socketing_socket_button_mixin_carries_eight_methods(env: &WowLuaEnv) {
    load_item_socketing_ui(env);

    let kind: String = env
        .eval("return type(GenericSocketButtonMixin)")
        .expect("GenericSocketButtonMixin probe should succeed");
    assert_eq!(
        kind, "table",
        "GenericSocketButtonMixin must publish at `_G` as a table — \
         Blizzard_ItemSocketingUI.lua:73 creates the empty table at file scope before binding 8 \
         methods to it. The mixin drives each socket button's lifecycle (OnLoad / OnEvent), \
         input (OnClick / OnReceiveDrag / OnDragStart), and tooltip surface (OnEnter / OnLeave), \
         routing all server interactions through C_ItemSocketInfo.ClickSocketButton via the \
         shared ClickSocketButton helper"
    );

    for method in SOCKET_BUTTON_METHODS {
        let kind: String = env
            .eval(&format!(
                "return type(GenericSocketButtonMixin['{method}'])"
            ))
            .unwrap_or_else(|err| panic!("GenericSocketButtonMixin.{method} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "GenericSocketButtonMixin.{method} must publish as a function — the socket-button \
             mixin owns 8 methods that drive the per-socket interaction. Missing this method \
             implies the .lua never executed via `<Script file=...>` or the mixin table was \
             overwritten before the method bound"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_item_socketing_frame_mixin_carries_nine_methods(env: &WowLuaEnv) {
    load_item_socketing_ui(env);

    let kind: String = env
        .eval("return type(GenericItemSocketingFrameMixin)")
        .expect("GenericItemSocketingFrameMixin probe should succeed");
    assert_eq!(
        kind, "table",
        "GenericItemSocketingFrameMixin must publish at `_G` as a table — \
         Blizzard_ItemSocketingUI.lua:146 creates the empty table at file scope before binding 9 \
         methods to it. The mixin drives the SocketingContainer (the 3-socket panel inside \
         ItemSocketingFrame): OnLoad seeds the Enum.ItemSocketInfoUIType.RemixArtifactUI default \
         and wires the ApplySocketsButton onClickHandler, OnShow / OnHide drive event \
         registration, RegisterEvents / UnregisterEvents toggle the 5 SOCKET_INFO_* event set, \
         OnEvent dispatches BIND_CONFIRM / REFUNDABLE_CONFIRM / ACCEPT / SUCCESS / FAILURE, \
         Update repaints all 3 sockets per SOCKET_INFO_UPDATE, and DisableSockets / EnableSockets \
         toggle button enabled state during the in-flight accept window"
    );

    for method in SOCKETING_FRAME_METHODS {
        let kind: String = env
            .eval(&format!(
                "return type(GenericItemSocketingFrameMixin['{method}'])"
            ))
            .unwrap_or_else(|err| {
                panic!("GenericItemSocketingFrameMixin.{method} probe failed: {err}")
            });
        assert_eq!(
            kind, "function",
            "GenericItemSocketingFrameMixin.{method} must publish as a function — the frame mixin \
             owns 9 methods. Missing this method implies the .lua never executed via \
             `<Script file=...>` or the mixin table was overwritten"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_item_socketing_publishes_four_free_helper_functions(env: &WowLuaEnv) {
    load_item_socketing_ui(env);

    for helper in FREE_HELPER_FUNCTIONS {
        let kind: String = env
            .eval(&format!("return type({helper})"))
            .unwrap_or_else(|err| panic!("{helper} probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "{helper} must publish at `_G` as a function — Blizzard_ItemSocketingUI.lua defines 4 \
             free-helper globals at file scope: ItemSocketingFrame_OnLoad (xml-wired \
             `<OnLoad function=\"...\"/>` registers SOCKET_INFO_UPDATE / SOCKET_INFO_CLOSE and \
             primes the description tooltip min-width), ItemSocketingFrame_OnEvent (xml-wired \
             `<OnEvent function=\"...\"/>` dispatches the 2 socket events), \
             ItemSocketingFrame_Update (called by OnEvent on SOCKET_INFO_UPDATE — drives \
             SocketingContainer:Update + SetPortraitToAsset + ItemSocketingDescription \
             SetSocketedItem), and ItemSocketingSocketButton_OnScrollRangeChanged (the \
             ScrollFrame:RegisterCallback target that re-renders the description tooltip when \
             the scroll range changes)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_item_socketing_frame_publishes_with_button_frame_template_chain(env: &WowLuaEnv) {
    load_item_socketing_ui(env);

    let kind: String = env
        .eval("return type(ItemSocketingFrame)")
        .expect("ItemSocketingFrame probe should succeed");
    assert_eq!(
        kind, "table",
        "ItemSocketingFrame must publish at `_G` as a table — declared at \
         Blizzard_ItemSocketingUI.xml:143 with `name=\"ItemSocketingFrame\"` `toplevel=\"true\"` \
         `parent=\"UIParent\"` `enableMouse=\"true\"` `hidden=\"true\"` \
         `inherits=\"ButtonFrameTemplate\"`. The ButtonFrameTemplate chain provides the title, \
         portrait, close button, and inset; hidden=\"true\" keeps it invisible until \
         SOCKET_INFO_UPDATE -> ShowUIPanel fires"
    );

    let name: String = env
        .eval("return ItemSocketingFrame:GetName()")
        .expect("ItemSocketingFrame:GetName() probe should succeed");
    assert_eq!(name, "ItemSocketingFrame");

    let visible: bool = env
        .eval("return ItemSocketingFrame:IsShown()")
        .expect("ItemSocketingFrame:IsShown() probe should succeed");
    assert!(
        !visible,
        "ItemSocketingFrame must report IsShown=false on load — XML declares `hidden=\"true\"` \
         and the frame stays invisible until C_ItemSocketInfo fires SOCKET_INFO_UPDATE for a \
         right-clicked gem-bearing item"
    );
}
}

prefork_full_ui_case! {
fn blizzard_item_socketing_publishes_socketing_container_with_three_sockets(env: &WowLuaEnv) {
    load_item_socketing_ui(env);

    let kind: String = env
        .eval("return type(ItemSocketingFrame.SocketingContainer)")
        .expect("SocketingContainer probe should succeed");
    assert_eq!(
        kind, "table",
        "ItemSocketingFrame.SocketingContainer must publish via parentKey — the XML at line 419 \
         declares the inner Frame inheriting GenericItemSocketingFrameTemplate (which carries the \
         3 SocketFrames buttons via parentArray and the ApplySocketsButton). The parentKey wiring \
         is what allows ItemSocketingFrame_OnLoad to call \
         self.SocketingContainer.ApplySocketsButton:ClearAllPoints() at lua line 40"
    );

    for slot in 1..=3u32 {
        let socket_kind: String = env
            .eval(&format!(
                "return type(ItemSocketingFrame.SocketingContainer.Socket{slot})"
            ))
            .unwrap_or_else(|err| panic!("SocketingContainer.Socket{slot} probe failed: {err}"));
        assert_eq!(
            socket_kind, "table",
            "ItemSocketingFrame.SocketingContainer.Socket{slot} must publish via parentKey — the \
             GenericItemSocketingFrameTemplate XML declares 3 socket buttons (Socket1 / Socket2 / \
             Socket3) inheriting GenericSocketButtonTemplate at xml lines 101-126 with id=1/2/3 \
             and parentArray=\"SocketFrames\". GenericItemSocketingFrameMixin:Update iterates \
             self.SocketFrames to repaint each on SOCKET_INFO_UPDATE"
        );
    }

    let apply_button_kind: String = env
        .eval("return type(ItemSocketingFrame.SocketingContainer.ApplySocketsButton)")
        .expect("ApplySocketsButton probe should succeed");
    assert_eq!(
        apply_button_kind, "table",
        "ItemSocketingFrame.SocketingContainer.ApplySocketsButton must publish via parentKey — \
         the GenericItemSocketingFrameTemplate XML declares the apply button at xml line 128 with \
         text=\"APPLY\" inheriting UIPanelButtonNoTooltipTemplate, UIButtonTemplate. \
         GenericItemSocketingFrameMixin:OnLoad assigns its onClickHandler at lua line 162"
    );
}
}

prefork_full_ui_case! {
fn blizzard_item_socketing_registers_ui_panel_window_entry(env: &WowLuaEnv) {
    load_item_socketing_ui(env);

    let area: String = env
        .eval("return tostring(UIPanelWindows['ItemSocketingFrame'].area)")
        .expect("UIPanelWindows entry probe should succeed");
    assert_eq!(
        area, "left",
        "UIPanelWindows['ItemSocketingFrame'].area must equal \"left\" — \
         Blizzard_ItemSocketingUI.lua:1 registers the panel as a left-docked panel via \
         `UIPanelWindows[\"ItemSocketingFrame\"] = {{ area = \"left\", pushable = 0 }}` so \
         ShowUIPanel docks it at the standard CharacterFrame slot"
    );

    let pushable: f64 = env
        .eval("return UIPanelWindows['ItemSocketingFrame'].pushable")
        .expect("pushable probe should succeed");
    assert_eq!(
        pushable, 0.0,
        "UIPanelWindows['ItemSocketingFrame'].pushable must equal 0 — pushable=0 means the \
         socketing frame cannot be pushed aside by any other panel; opening any left-docked panel \
         while socketing closes the socketing frame outright (the socket session is cancelled)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_item_socketing_publishes_description_min_width_global(env: &WowLuaEnv) {
    load_item_socketing_ui(env);

    let value: f64 = env
        .eval("return ITEM_SOCKETING_DESCRIPTION_MIN_WIDTH")
        .expect("ITEM_SOCKETING_DESCRIPTION_MIN_WIDTH probe should succeed");
    assert_eq!(
        value, 240.0,
        "ITEM_SOCKETING_DESCRIPTION_MIN_WIDTH must equal 240 — \
         Blizzard_ItemSocketingUI.lua:28 publishes the magic number as a global so \
         ItemSocketingFrame_OnLoad and ItemSocketingFrame_Update can call \
         ItemSocketingDescription:SetMinimumWidth(240, true) without re-declaring the literal. \
         The 240px floor keeps the description tooltip readable when a low-stat gem produces a \
         narrow socket bonus line"
    );
}
}

prefork_full_ui_case! {
fn blizzard_item_socketing_virtual_templates_stay_nil_at_global_scope(env: &WowLuaEnv) {
    load_item_socketing_ui(env);

    for template in VIRTUAL_TEMPLATE_NAMES {
        let kind: String = env
            .eval(&format!("return type(_G['{template}'])"))
            .unwrap_or_else(|err| panic!("{template} probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "_G['{template}'] must be nil — virtual templates only register in the template \
             registry, NOT at `_G`. Blizzard_ItemSocketingUI ships 3 virtual templates: \
             NubTemplate (Texture, the 11x12 corner-nub decoration applied to the 6 corner/edge \
             nub layers), GenericSocketButtonTemplate (Button, the per-socket button with shine / \
             bracket / icon / filigree composition), GenericItemSocketingFrameTemplate (Frame, \
             the 3-socket container with apply button). Each is consumed only via XML \
             `inherits=\"...\"` resolution"
        );
    }
}
}
