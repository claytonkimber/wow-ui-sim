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

fn housing_cornerstone_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_HousingCornerstone")
}

fn housing_cornerstone_toc() -> PathBuf {
    housing_cornerstone_dir().join("Blizzard_HousingCornerstone.toc")
}

fn load_housing_cornerstone(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &housing_cornerstone_toc())
        .expect("Blizzard_HousingCornerstone should load via explicit Rust loader call");
}

fn eval_bool(env: &WowLuaEnv, script: &str, context: &str) -> bool {
    env.eval(script).expect(context)
}

#[test]
fn blizzard_housing_cornerstone_find_toc_resolves_bare_variant() {
    let resolved = find_toc_file(&housing_cornerstone_dir())
        .expect("Blizzard_HousingCornerstone TOC should resolve");
    assert_eq!(
        resolved,
        housing_cornerstone_toc(),
        "Blizzard_HousingCornerstone ships exactly one bare TOC — retail-only addon resolves \
         via `find_toc_file` fallthrough"
    );
}

#[test]
fn blizzard_housing_cornerstone_toc_declares_lod_with_two_dependencies() {
    let toc = TocFile::from_file(&housing_cornerstone_toc())
        .expect("HousingCornerstone TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_HousingCornerstone declares `## LoadOnDemand: 1` — pulled via explicit \
         LoadAddOn from HousingControlButton.lua:126 and HousingEventHandler.lua:312"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert_eq!(
        toc.dependencies(),
        vec![
            "Blizzard_HousingTemplates".to_string(),
            "Blizzard_MoneyFrame".to_string(),
        ],
        "Two `## Dependencies:` entries in order: Blizzard_HousingTemplates (housing atlas \
         families + HousingTopBannerFrame) and Blizzard_MoneyFrame (SmallMoneyFrameTemplate / \
         MoneyFrameTemplate + the SmallMoneyFrame_OnLoad / MoneyFrame_SetType / \
         MoneyFrame_Update / SetMoneyFrameColor / GetMoneyString globals)"
    );
}

#[test]
fn blizzard_housing_cornerstone_toc_is_retail_only_and_omits_allow_load() {
    let toc = TocFile::from_file(&housing_cornerstone_toc())
        .expect("HousingCornerstone TOC should parse");
    let toc_text = std::fs::read_to_string(housing_cornerstone_toc()).expect("TOC should read");
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Declares `## AllowLoadGameType: standard` — retail-only Midnight feature"
    );
    assert!(!toc.is_game_type_restricted());
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Omits `## AllowLoad:` — LoadOnDemand precludes auto-discovery gating"
    );
    assert!(!toc_text.contains("## DefaultState:"));
    assert!(
        toc.saved_variables().is_empty(),
        "No `## SavedVariables*` — purchase mode and house info are server-driven via \
         C_HousingNeighborhood and event dispatch"
    );
}

#[test]
fn blizzard_housing_cornerstone_toc_lists_three_files_in_order() {
    let toc = TocFile::from_file(&housing_cornerstone_toc())
        .expect("HousingCornerstone TOC should parse");
    let files: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();
    assert_eq!(
        files,
        vec![
            "Blizzard_HousingCornerstone.lua".to_string(),
            "Blizzard_HousingCornerstone.xml".to_string(),
            "Blizzard_HousingCornerstoneRegistration.lua".to_string(),
        ],
        "TOC body lists exactly 3 source files in this order: .lua (8 mixins) before .xml \
         (mixin= references) before Registration.lua (RegisterUIPanel tail file)"
    );
}

#[test]
fn blizzard_housing_cornerstone_directory_holds_four_entries() {
    let dir = housing_cornerstone_dir();
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_HousingCornerstone directory should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        4,
        "Directory ships exactly 4 entries (3 source files + 1 TOC). Got: {entries:?}"
    );
    assert!(entries.contains(&"Blizzard_HousingCornerstone.toc".to_string()));
    assert!(entries.contains(&"Blizzard_HousingCornerstoneRegistration.lua".to_string()));
}

#[test]
fn blizzard_housing_cornerstone_excluded_from_all_screen_auto_discovery_passes() {
    let ui = blizzard_ui_dir();
    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&ui, screen);
        let discovered = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_HousingCornerstone");
        assert!(
            !discovered,
            "Must NOT appear in {screen:?} auto-discovery — `## LoadOnDemand: 1` keeps it out \
             of every screen pass; consumers pull via explicit LoadAddOn"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    let lua_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let related: Vec<&String> = lua_errors
        .iter()
        .filter(|e| {
            e.contains("Blizzard_HousingCornerstone/")
                || e.contains("Blizzard_HousingCornerstone\\")
                || e.contains("HousingCornerstoneFrameMixin")
                || e.contains("HousingCornerstonePurchaseFrameMixin")
                || e.contains("HousingCornerstoneVisitorFrameSharedMixin")
                || e.contains("HousingCornerstoneVisitorFrameMixin")
                || e.contains("HousingCornerstoneHouseInfoFrameMixin")
                || e.contains("BuyHouseConfirmationDialogMixin")
                || e.contains("MoveHouseConfirmationDialogMixin")
                || e.contains("ImportHouseConfirmationDialogMixin")
        })
        .collect();
    assert!(
        related.is_empty(),
        "Blizzard_HousingCornerstone emitted addon-specific Lua errors during explicit LoD \
         load:\n  {}",
        related
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_is_addon_loaded_returns_true_after_explicit_lod_load(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingCornerstone')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        loaded,
        "After explicit LoD load, `IsAddOnLoaded('Blizzard_HousingCornerstone')` must return \
         true"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_dependencies_load_via_game_screen_pass(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    let templates_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_HousingTemplates')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        templates_loaded,
        "Blizzard_HousingTemplates must be auto-loaded — first dependency on Cornerstone's TOC"
    );

    let money_loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_MoneyFrame')")
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        money_loaded,
        "Blizzard_MoneyFrame must be auto-loaded — second dependency, required for 3 \
         SmallMoneyFrameTemplate children + 1 MoneyFrameTemplate child"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_publishes_seven_named_frames_globally(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    for frame_name in [
        "HousingCornerstoneFrame",
        "HousingCornerstonePurchaseFrame",
        "HousingCornerstoneVisitorFrame",
        "HousingCornerstoneHouseInfoFrame",
        "BuyHouseConfirmationDialog",
        "MoveHouseConfirmationDialog",
        "ImportHouseConfirmationDialog",
    ] {
        let exists: bool = env
            .eval(&format!(
                "local f = _G['{frame_name}']; return type(f) == 'table' and type(f.GetName) == 'function'"
            ))
            .expect("Named frame global lookup should succeed");
        assert!(
            exists,
            "After LoD load, `{frame_name}` should publish as a global frame instance — \
             Cornerstone XML declares 7 named non-virtual frames: tabbed master + purchase + \
             visitor + house-info + 3 confirmation dialogs"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_does_not_publish_visitor_template_global(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    let published: bool = env
        .eval("return _G['HousingCornerstoneVisitorTemplate'] ~= nil")
        .expect("Template global lookup should succeed");
    assert!(
        !published,
        "HousingCornerstoneVisitorTemplate is `virtual=\"true\"` — virtual XML templates are \
         not instantiated as global frames; only inheriting frames materialize"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_frame_mixin_publishes_seven_methods(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    for method in [
        "OnLoad",
        "OnEvent",
        "OnShow",
        "OnHide",
        "UpdateTabs",
        "SetToDefaultAvailableTab",
        "SetTab",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCornerstoneFrameMixin['{method}']) == 'function'"
            ))
            .expect("HousingCornerstoneFrameMixin method existence query should succeed");
        assert!(
            exists,
            "HousingCornerstoneFrameMixin must expose `:{method}()` — the master tabbed shell \
             chaining TabSystemOwnerMixin with InfoFrame + DropboxFrame tabs"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_purchase_frame_mixin_publishes_twelve_methods(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    for method in [
        "OnLoad",
        "OnEvent",
        "OnCinematicStopped",
        "OnShow",
        "CheckMoveCooldown",
        "GetTypeString",
        "OnHide",
        "CheckPurchaseEligibility",
        "OnPurchaseClicked",
        "OnConfirmPurchase",
        "OnNeighborhoodInfoUpdated",
        "SetInputMaskShown",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCornerstonePurchaseFrameMixin['{method}']) == 'function'"
            ))
            .expect("HousingCornerstonePurchaseFrameMixin method existence query should succeed");
        assert!(
            exists,
            "HousingCornerstonePurchaseFrameMixin must expose `:{method}()` — the purchase / \
             move / import dispatch frame branching on GetCornerstonePurchaseMode and 9 \
             Enum.PurchaseHouseDisabledReason variants"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_visitor_shared_mixin_publishes_two_methods(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    for method in ["OnLoad", "OnReportClicked"] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCornerstoneVisitorFrameSharedMixin['{method}']) == 'function'"
            ))
            .expect(
                "HousingCornerstoneVisitorFrameSharedMixin method existence query should succeed",
            );
        assert!(
            exists,
            "HousingCornerstoneVisitorFrameSharedMixin must expose `:{method}()` — shared base \
             for visitor + house-info via CreateFromMixins; OnLoad wires GearDropdown report \
             menu, OnReportClicked builds CreateDecorReportInfo and InitiateReport"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_visitor_frame_mixin_inherits_shared_and_overrides_three_methods(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    let shared_inherited: bool = env
        .eval(
            "return HousingCornerstoneVisitorFrameMixin.OnLoad == HousingCornerstoneVisitorFrameSharedMixin.OnLoad",
        )
        .expect("CreateFromMixins shared inheritance query should succeed");
    assert!(
        shared_inherited,
        "HousingCornerstoneVisitorFrameMixin must reuse OnLoad from the shared mixin via \
         CreateFromMixins"
    );

    for method in ["OnEvent", "OnShow", "OnHide"] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCornerstoneVisitorFrameMixin['{method}']) == 'function'"
            ))
            .expect("HousingCornerstoneVisitorFrameMixin method existence query should succeed");
        assert!(
            exists,
            "HousingCornerstoneVisitorFrameMixin must add `:{method}()` — visitor lifecycle \
             dispatching CLOSE_PLOT_CORNERSTONE and registering visitor showing events"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_house_info_frame_mixin_inherits_shared_and_overrides_four_methods(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    let shared_inherited: bool = env
        .eval(
            "return HousingCornerstoneHouseInfoFrameMixin.OnReportClicked == HousingCornerstoneVisitorFrameSharedMixin.OnReportClicked",
        )
        .expect("CreateFromMixins shared inheritance query should succeed");
    assert!(
        shared_inherited,
        "HouseInfo mixin must reuse OnReportClicked from shared mixin via CreateFromMixins"
    );

    for method in ["OnShow", "OnHide", "OnEvent", "UpdateHouseInfo"] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCornerstoneHouseInfoFrameMixin['{method}']) == 'function'"
            ))
            .expect("HousingCornerstoneHouseInfoFrameMixin method existence query should succeed");
        assert!(
            exists,
            "HousingCornerstoneHouseInfoFrameMixin must add `:{method}()` — async house info \
             via CURRENT_HOUSE_INFO_RECIEVED/_UPDATED events (Blizzard's RECIEVED typo \
             preserved); OnHide cross-touches HousingControlsFrame.OwnerControlFrame"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_buy_house_confirmation_dialog_mixin_publishes_three_methods(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    for method in ["OnLoad", "OnShow", "OnHide"] {
        let exists: bool = env
            .eval(&format!(
                "return type(BuyHouseConfirmationDialogMixin['{method}']) == 'function'"
            ))
            .expect("BuyHouseConfirmationDialogMixin method existence query should succeed");
        assert!(
            exists,
            "BuyHouseConfirmationDialogMixin must expose `:{method}()` — basic-purchase dialog \
             wiring AcceptButton/CancelButton via EventRegistry + StaticPopupSpecial_Hide"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_move_house_confirmation_dialog_mixin_publishes_three_methods(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    for method in ["OnLoad", "OnShow", "OnHide"] {
        let exists: bool = env
            .eval(&format!(
                "return type(MoveHouseConfirmationDialogMixin['{method}']) == 'function'"
            ))
            .expect("MoveHouseConfirmationDialogMixin method existence query should succeed");
        assert!(
            exists,
            "MoveHouseConfirmationDialogMixin must expose `:{method}()` — move-with-discount \
             dialog using SmallMoneyFrame on PriceMoneyFrameOriginal/Discount and \
             pricestrikethrough-gray overlay; refund mode when GetDiscountedMovePrice<0"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_import_house_confirmation_dialog_mixin_publishes_three_methods(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    for method in ["OnLoad", "OnShow", "OnHide"] {
        let exists: bool = env
            .eval(&format!(
                "return type(ImportHouseConfirmationDialogMixin['{method}']) == 'function'"
            ))
            .expect("ImportHouseConfirmationDialogMixin method existence query should succeed");
        assert!(
            exists,
            "ImportHouseConfirmationDialogMixin must expose `:{method}()` — rebind-old-house \
             dialog using GetPreviousHouseIdentifier; no PriceMoneyFrame (price inline)"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_purchase_frame_publishes_money_widgets_via_parent_keys(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    let cost_text_frame = eval_bool(
        &env,
        "local f = HousingCornerstonePurchaseFrame.CostTextFrame; return type(f) == 'table' and type(f.GetName) == 'function'",
        "CostTextFrame parentKey lookup should succeed",
    );
    assert!(
        cost_text_frame,
        "PurchaseFrame.CostTextFrame must publish via parentKey"
    );

    let price_money_frame = eval_bool(
        &env,
        "local f = HousingCornerstonePurchaseFrame.CostTextFrame.PriceMoneyFrame; return type(f) == 'table'",
        "PriceMoneyFrame parentKey lookup should succeed",
    );
    assert!(
        price_money_frame,
        "PurchaseFrame.CostTextFrame.PriceMoneyFrame must publish via parentKey — \
         SmallMoneyFrameTemplate child also published as global HousingCornerstonePriceMoneyFrame"
    );

    let price_global = eval_bool(
        &env,
        "local f = _G['HousingCornerstonePriceMoneyFrame']; return type(f) == 'table' and type(f.GetName) == 'function'",
        "HousingCornerstonePriceMoneyFrame global lookup should succeed",
    );
    assert!(
        price_global,
        "_G.HousingCornerstonePriceMoneyFrame must publish — only child with both name= and \
         parentKey= because MoneyFrame_Update / SetMoneyFrameColor look up by string name"
    );

    let purchase_money_frame = eval_bool(
        &env,
        "local f = HousingCornerstonePurchaseFrame.MoneyFrame; return type(f) == 'table' and type(f.GoldButton) ~= nil",
        "MoneyFrame parentKey lookup should succeed",
    );
    assert!(
        purchase_money_frame,
        "PurchaseFrame.MoneyFrame must publish via parentKey — MoneyFrameTemplate (not Small) \
         showing player's current Gold/Silver/Copper denominations"
    );

    let input_mask = eval_bool(
        &env,
        "local f = HousingCornerstonePurchaseFrame.InputMask; return type(f) == 'table' and type(f.GetName) == 'function'",
        "InputMask parentKey lookup should succeed",
    );
    assert!(
        input_mask,
        "PurchaseFrame.InputMask must publish via parentKey — toggled via SetInputMaskShown \
         EventRegistry callback so confirmation dialogs can clear the mask on dismiss"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_frame_publishes_tab_system_subframes(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    for parent_key in ["TabSystem", "InfoFrame", "DropboxFrame", "CloseButton"] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCornerstoneFrame['{parent_key}']) ~= 'nil'"
            ))
            .expect("HousingCornerstoneFrame parentKey lookup should succeed");
        assert!(
            exists,
            "HousingCornerstoneFrame.{parent_key} must publish via parentKey — TabSystem + \
             InfoFrame + DropboxFrame + CloseButton tabbed-shell children"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_visitor_frame_inherits_template_children(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    for parent_key in [
        "Border",
        "Background",
        "Header",
        "HouseNameText",
        "OwnerLabel",
        "OwnerText",
        "LocationLabel",
        "PlotText",
        "NeighborhoodLabel",
        "NeighborhoodText",
        "CloseButton",
        "GearDropdown",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(HousingCornerstoneVisitorFrame['{parent_key}']) ~= 'nil'"
            ))
            .expect("HousingCornerstoneVisitorFrame parentKey lookup should succeed");
        assert!(
            exists,
            "HousingCornerstoneVisitorFrame.{parent_key} must publish via parentKey from \
             inherited HousingCornerstoneVisitorTemplate — shared with HouseInfoFrame"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_house_info_frame_adds_loading_spinner(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    let exists: bool = env
        .eval(
            "local f = HousingCornerstoneHouseInfoFrame.LoadingSpinner; return type(f) == 'table' and type(f.GetName) == 'function'",
        )
        .expect("LoadingSpinner parentKey lookup should succeed");
    assert!(
        exists,
        "HouseInfoFrame.LoadingSpinner must publish via parentKey — SpinnerTemplate child \
         shown while C_Housing.GetCurrentHouseInfo returns incomplete data per HasData helper"
    );
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_move_dialog_publishes_strikethrough_overlay(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    for parent_key in [
        "PriceMoneyFrameOriginal",
        "OriginalStrikethrough",
        "PriceMoneyFrameDiscount",
        "ConfirmButton",
        "CancelButton",
        "ConfirmationText",
        "HouseToMoveLabel",
        "HouseToMoveText",
        "PriceLabel",
    ] {
        let exists: bool = env
            .eval(&format!(
                "return type(MoveHouseConfirmationDialog['{parent_key}']) ~= 'nil'"
            ))
            .expect("MoveHouseConfirmationDialog parentKey lookup should succeed");
        assert!(
            exists,
            "MoveHouseConfirmationDialog.{parent_key} must publish via parentKey — \
             pricestrikethrough overlay + dual SmallMoneyFrame for discount visualization"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_registers_four_named_panels_via_register_ui_panel(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    for frame_name in [
        "HousingCornerstoneFrame",
        "HousingCornerstonePurchaseFrame",
        "HousingCornerstoneVisitorFrame",
        "HousingCornerstoneHouseInfoFrame",
    ] {
        let registered: bool = env
            .eval(&format!(
                "return type(UIPanelWindows['{frame_name}']) == 'table'"
            ))
            .expect("UIPanelWindows registration query should succeed");
        assert!(
            registered,
            "UIPanelWindows['{frame_name}'] must publish after Registration.lua tail file — \
             4 RegisterUIPanel calls sharing center / pushable=0 / allowOtherPanels=1 attrs"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_housing_cornerstone_registration_uses_center_area_with_no_pushable(env: &WowLuaEnv) {
    load_housing_cornerstone(env);

    let area: String = env
        .eval("return UIPanelWindows['HousingCornerstoneFrame'].area")
        .expect("UIPanelWindows.area query should succeed");
    assert_eq!(
        area, "center",
        "Registration.lua attributes.area = \"center\" — purchase/move/import flow needs \
         central focus (vs HousingCharter's area=\"left\")"
    );

    let pushable: i64 = env
        .eval("return UIPanelWindows['HousingCornerstoneFrame'].pushable")
        .expect("UIPanelWindows.pushable query should succeed");
    assert_eq!(
        pushable, 0,
        "Registration.lua attributes.pushable = 0 — overlays center as focused dialog; \
         InputMask child handles modal-like blocking (vs HousingCharter's pushable=2)"
    );

    let allow_other_panels: i64 = env
        .eval("return UIPanelWindows['HousingCornerstoneFrame'].allowOtherPanels")
        .expect("UIPanelWindows.allowOtherPanels query should succeed");
    assert_eq!(
        allow_other_panels, 1,
        "Registration.lua attributes.allowOtherPanels = 1 — coexists with side-anchored \
         HouseEditor and HousingControls strip"
    );
}
}
