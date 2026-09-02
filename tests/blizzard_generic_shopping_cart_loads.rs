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

fn shopping_cart_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_GenericShoppingCart")
}

fn shopping_cart_toc() -> PathBuf {
    shopping_cart_dir().join("Blizzard_GenericShoppingCart.toc")
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
fn blizzard_generic_shopping_cart_resolves_bare_toc() {
    let resolved = find_toc_file(&shopping_cart_dir())
        .expect("Blizzard_GenericShoppingCart directory must contain a discoverable TOC");
    let resolved_name = resolved
        .file_name()
        .expect("resolved TOC must have a filename")
        .to_str()
        .expect("resolved TOC filename must be utf-8");

    assert_eq!(
        resolved_name, "Blizzard_GenericShoppingCart.toc",
        "Blizzard_GenericShoppingCart ships only the bare `Blizzard_GenericShoppingCart.toc` \
         (no `_Mainline.toc` variant); src/loader/mod.rs:65's `find_toc_file` falls through \
         the `_Mainline.toc` lookup and resolves the bare suffix"
    );
}

#[test]
fn blizzard_generic_shopping_cart_toc_declares_lod_game_only_standard_no_deps() {
    let toc =
        TocFile::from_file(&shopping_cart_toc()).expect("Blizzard_GenericShoppingCart TOC parse");

    assert!(
        toc.is_load_on_demand(),
        "Blizzard_GenericShoppingCart declares `## LoadOnDemand: 1` — the cart shell is \
         pulled in by callers like the Trading Post / Perks Program / Account Store UI \
         when they want to mount a cart-style purchase flow, not eagerly at startup"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_GenericShoppingCart does not declare `## UseSecureEnvironment` — purchase \
         flows route through unprotected store APIs, no protected-action surface"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_GenericShoppingCart declares no `## SavedVariables` — cart contents are \
         transient session state, not persisted client-side"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_GenericShoppingCart declares `## AllowLoadGameType: standard` — \
         is_game_type_restricted() must return false because src/toc.rs:299 treats both \
         `mainline` and `standard` as the unrestricted retail game type (src/toc.rs:56 \
         uses the same allowlist for the loader-level game-type filter)"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_GenericShoppingCart declares NO `## Dependencies` / `## RequiredDep` — \
         the addon only consumes globally-available shared mixins (CallbackRegistrantMixin \
         from Blizzard_SharedXMLBase/CallbackRegistrant.lua, ResizeLayoutFrame from \
         Blizzard_SharedXML, SharedButtonLargeTemplate, NineSlicePanelTemplate, \
         WowScrollBoxList, MinimalScrollBar, UIButtonTemplate) so it doesn't need the TOC \
         to enumerate transitive parents. Got: {:?}",
        toc.dependencies()
    );
}

#[test]
fn blizzard_generic_shopping_cart_toc_declares_allow_load_game_and_standard_default_enabled() {
    let toc_text = std::fs::read_to_string(shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart TOC should read");
    assert!(
        toc_text.contains("## AllowLoad: Game"),
        "Blizzard_GenericShoppingCart must declare `## AllowLoad: Game` — purchase carts \
         are meaningful only on the in-world Game screen, never on Login/CharacterSelect"
    );
    assert!(
        toc_text.contains("## AllowLoadGameType: standard"),
        "Blizzard_GenericShoppingCart must declare `## AllowLoadGameType: standard` — the \
         cart shell is retail-only (the `standard` alias for `mainline`); classic flavors \
         lack the Trading Post / Perks Program callers"
    );
    assert!(
        toc_text.contains("## DefaultState: enabled"),
        "Blizzard_GenericShoppingCart must declare `## DefaultState: enabled` — the addon \
         is bundled with the client and active by default; the LoadOnDemand flag governs \
         WHEN it loads, not whether it's allowed to load"
    );
}

#[test]
fn blizzard_generic_shopping_cart_toc_lists_three_lua_files_plus_one_xml() {
    let toc_text = std::fs::read_to_string(shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart TOC should read");
    let lua_count = toc_text.matches(".lua").count();
    let xml_count = toc_text.matches(".xml").count();
    assert_eq!(
        lua_count, 3,
        "Blizzard_GenericShoppingCart TOC enumerates exactly 3 .lua files: \
         ServicesBase (the ShoppingCartServiceRegistrantMixin / ShoppingCartServiceButtonMixin \
         base mixins shared by the data manager + visuals frame), DataManager (the \
         ShoppingCartDataManagerMixin owning cartList + add/remove/clear/purchase/update), \
         Templates (the ShoppingCartVisualsFrameMixin + 9 sibling mixins driving the XML \
         template surface). Got: {lua_count}"
    );
    assert_eq!(
        xml_count, 1,
        "Blizzard_GenericShoppingCart TOC enumerates exactly 1 .xml file \
         (Blizzard_GenericShoppingCartTemplates.xml — owns the 5 virtual templates: \
         ShoppingCartServiceButtonTemplate, ShoppingCartPriceTemplate, \
         ShoppingCartCheckoutButtonTemplate, ShoppingCartRemoveFromCartButtonTemplate, \
         ShoppingCartVisualsFrameTemplate). Got: {xml_count}"
    );
    assert!(
        toc_text.contains("Blizzard_GenericShoppingCartServicesBase.lua"),
        "TOC must list ServicesBase first — its mixins are referenced by DataManager's \
         CreateFromMixins(ShoppingCartServiceRegistrantMixin) at line 9, so file order \
         matters"
    );
}

#[test]
fn blizzard_generic_shopping_cart_excluded_from_game_auto_discovery_due_to_lod() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_GenericShoppingCart");
    assert!(
        !in_game,
        "Blizzard_GenericShoppingCart declares `## LoadOnDemand: 1` — it must NOT appear \
         in Game-screen auto-discovery; callers (Perks Program / Account Store / Trading \
         Post UIs) invoke `LoadAddOn(\"Blizzard_GenericShoppingCart\")` lazily when they \
         need a cart shell"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_GenericShoppingCart");
    assert!(
        !in_login,
        "Blizzard_GenericShoppingCart declares `## AllowLoad: Game` — combined with \
         LoadOnDemand it must also be absent from Login auto-discovery"
    );
}

prefork_full_ui_case! {
fn blizzard_generic_shopping_cart_loads_explicitly_via_load_addon_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let cart_errors: Vec<String> = load_errors
        .iter()
        .filter(|message| message.contains("ShoppingCart"))
        .cloned()
        .collect();
    assert!(
        cart_errors.is_empty(),
        "Blizzard_GenericShoppingCart emitted Lua errors during explicit load:\n  {}",
        cart_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_generic_shopping_cart_is_addon_loaded_returns_true_after_explicit_load(env: &WowLuaEnv) {

    let before: bool = env
        .eval("return C_AddOns and C_AddOns.IsAddOnLoaded('Blizzard_GenericShoppingCart') or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        !before,
        "IsAddOnLoaded should return false BEFORE explicit LoadAddOn — LoadOnDemand keeps \
         Blizzard_GenericShoppingCart out of auto-discovery"
    );

    load_addon(&env.loader_env(), &shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart should load via Rust loader");

    let after: bool = env
        .eval("return C_AddOns and C_AddOns.IsAddOnLoaded('Blizzard_GenericShoppingCart') or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        after,
        "C_AddOns.IsAddOnLoaded('Blizzard_GenericShoppingCart') must return true AFTER \
         explicit LoadAddOn (the loader registers the addon's name + state in the \
         addon-info table that backs IsAddOnLoaded)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_generic_shopping_cart_publishes_service_base_mixins(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart should load");

    let mixins: (String, String, bool) = env
        .eval(
            "return type(ShoppingCartServiceRegistrantMixin), \
                    type(ShoppingCartServiceButtonMixin), \
                    type(ShoppingCartServiceRegistrantMixin.AddServiceEvents) == 'function'",
        )
        .expect("Service base mixin probe should succeed");
    assert_eq!(
        mixins,
        ("table".to_string(), "table".to_string(), true),
        "Blizzard_GenericShoppingCartServicesBase.lua publishes 2 base mixins: \
         ShoppingCartServiceRegistrantMixin (line 1 — `CreateFromMixins(CallbackRegistrantMixin)` \
         so it carries the AddStaticEventMethod / RegisterCallback surface; owns \
         AddServiceEvents which folds an {{eventName=event}} dict into self.Events and wires \
         each entry to EventRegistry via AddStaticEventMethod) and \
         ShoppingCartServiceButtonMixin (line 16 — owns BaseService_OnClick which \
         triggers `EventRegistry:TriggerEvent(eventNamespace.\".\".serviceName, GetEventData())` \
         + GetEventData stub overridden in derived button mixins)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_generic_shopping_cart_publishes_data_manager_mixin_with_cart_lifecycle(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart should load");

    let manager: (String, String, bool, bool, bool, bool, bool, bool) = env
        .eval(
            "return type(ShoppingCartDataManagerMixin), \
                    type(ShoppingCartClearCartServiceMixin), \
                    type(ShoppingCartDataManagerMixin.Init) == 'function', \
                    type(ShoppingCartDataManagerMixin.AddToCart) == 'function', \
                    type(ShoppingCartDataManagerMixin.RemoveFromCart) == 'function', \
                    type(ShoppingCartDataManagerMixin.ClearCart) == 'function', \
                    type(ShoppingCartDataManagerMixin.PurchaseCart) == 'function', \
                    type(ShoppingCartDataManagerMixin.GetNumItemsInCart) == 'function'",
        )
        .expect("Data manager probe should succeed");
    assert_eq!(
        manager,
        (
            "table".to_string(),
            "table".to_string(),
            true,
            true,
            true,
            true,
            true,
            true,
        ),
        "Blizzard_GenericShoppingCartDataManager.lua publishes \
         ShoppingCartDataManagerMixin (line 9 — `CreateFromMixins(\
         ShoppingCartServiceRegistrantMixin)` so the manager itself is also a service \
         registrant; owns Init/AddToCart/RemoveFromCart/ClearCart/PurchaseCart/UpdateCart/\
         GetNumItemsInCart driving the underlying cartList) + ShoppingCartClearCartServiceMixin \
         (line 128 — owns GetEventData returning `clearRequiresConfirmation = true`, the \
         tag the ClearCart service handler reads to know whether to prompt before wiping \
         the cart)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_generic_shopping_cart_publishes_data_services_constants_table(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart should load");

    let services: (String, String, String, String, String) = env
        .eval(
            "return ShoppingCartDataServices.AddToCart, \
                    ShoppingCartDataServices.RemoveFromCart, \
                    ShoppingCartDataServices.UpdateCart, \
                    ShoppingCartDataServices.ClearCart, \
                    ShoppingCartDataServices.PurchaseCart",
        )
        .expect("Data services probe should succeed");
    assert_eq!(
        services,
        (
            "AddToCart".to_string(),
            "RemoveFromCart".to_string(),
            "UpdateCart".to_string(),
            "ClearCart".to_string(),
            "PurchaseCart".to_string(),
        ),
        "Blizzard_GenericShoppingCartDataManager.lua publishes the canonical \
         ShoppingCartDataServices constants table (line 1) — 5 string entries that \
         AddServiceEvents joins with eventNamespace to form EventRegistry callback names \
         (e.g. `GenericCartEvents.AddToCart`); the values must equal the keys verbatim so \
         that `ShoppingCartServiceButtonTemplate`'s XML KeyValues `serviceName=\"AddToCart\"` \
         resolves to `eventNamespace .. \".\" .. self.serviceName` correctly"
    );
}
}

prefork_full_ui_case! {
fn blizzard_generic_shopping_cart_publishes_visuals_frame_mixin_with_full_lifecycle(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart should load");

    let visuals: (bool, bool, bool, bool, bool, bool, bool) = env
        .eval(
            "return type(ShoppingCartVisualsFrameMixin) == 'table', \
                    type(ShoppingCartVisualsFrameMixin.OnLoad) == 'function', \
                    type(ShoppingCartVisualsFrameMixin.SetupScrollBox) == 'function', \
                    type(ShoppingCartVisualsFrameMixin.FullUpdate) == 'function', \
                    type(ShoppingCartVisualsFrameMixin.SetCartShown) == 'function', \
                    type(ShoppingCartVisualsFrameMixin.UpdateScrollBar) == 'function', \
                    type(ShoppingCartVisualsFrameMixin.UpdateCurrencyTotal) == 'function'",
        )
        .expect("Visuals frame mixin probe should succeed");
    assert_eq!(
        visuals,
        (true, true, true, true, true, true, true),
        "Blizzard_GenericShoppingCartTemplates.lua publishes \
         ShoppingCartVisualsFrameMixin (line 8 — `CreateFromMixins(\
         ShoppingCartServiceRegistrantMixin)` so the visuals frame is itself a service \
         registrant — that's how the cart shell wires HideCartButton / ClearCartButton / \
         PurchaseCartButton / ViewCartButton clicks back to EventRegistry events). The 6 \
         lifecycle methods drive the canonical cart UI flow: OnLoad initializes itemList + \
         dynamically creates the PurchaseCartButton in BOTH containers (line 16 and line \
         20) using self.purchaseButtonTemplate; SetupScrollBox builds a \
         ScrollBoxListLinearView + AddSelectionBehavior(MultiSelect, Intrusive); FullUpdate \
         triggers UpdateScrollBar + RefreshScrollElements + Rebuild + UpdateNumItemsInCart \
         + UpdateCurrencyTotal + RefreshDividers; SetCartShown swaps \
         CartVisibleContainer/CartHiddenContainer visibility; UpdateScrollBar \
         dynamically resizes ScrollBox based on item count vs maxItemsToShow; \
         UpdateCurrencyTotal pulls GetMoney() and reanchors PlayerTotalCurrencyDisplay"
    );
}
}

prefork_full_ui_case! {
fn blizzard_generic_shopping_cart_publishes_button_and_currency_mixins(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart should load");

    let button_mixins: (String, String, String, String, String, String) = env
        .eval(
            "return type(ShoppingCartPriceContainerMixin), \
                    type(ShoppingCartViewCartButtonMixin), \
                    type(ShoppingCartShowCartServiceMixin), \
                    type(ShoppingCartHideCartServiceMixin), \
                    type(ShoppingCartRemoveFromCartItemButtonContainerMixin), \
                    type(ShoppingCartRemoveFromCartItemButtonMixin)",
        )
        .expect("Button mixin probe should succeed");
    assert_eq!(
        button_mixins,
        (
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
            "table".to_string(),
        ),
        "Blizzard_GenericShoppingCartTemplates.lua publishes 6 button/container mixins: \
         ShoppingCartPriceContainerMixin (line 294 — drives ShoppingCartPriceTemplate's \
         price/sale-price layout with GameFontNormalMed3 / GameFontNormalLarge font \
         switching based on isLarge KeyValue), ShoppingCartViewCartButtonMixin (line 373 — \
         the show-cart toggle that wires the item-count badge), \
         ShoppingCartShowCartServiceMixin / ShoppingCartHideCartServiceMixin (lines 385 + \
         392 — return GetEventData() for the SetCartShown service event), \
         ShoppingCartRemoveFromCartItemButtonContainerMixin / \
         ShoppingCartRemoveFromCartItemButtonMixin (lines 399 + 413 — the trash-can hover \
         pair where the container shows the button on OnEnter and hides it on OnLeave \
         once both halves agree the mouse has left)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_generic_shopping_cart_publishes_visual_services_and_currency_mixin(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart should load");

    let visual_services: (String, String, String, String) = env
        .eval(
            "return type(ShoppingCartVisualServices), \
                    ShoppingCartVisualServices.SetCartFrameShown, \
                    ShoppingCartVisualServices.SetCartShown, \
                    ShoppingCartVisualServices.OnCartItemInteraction",
        )
        .expect("Visual services probe should succeed");
    assert_eq!(
        visual_services,
        (
            "table".to_string(),
            "SetCartFrameShown".to_string(),
            "SetCartShown".to_string(),
            "OnCartItemInteraction".to_string(),
        ),
        "Blizzard_GenericShoppingCartTemplates.lua publishes ShoppingCartVisualServices \
         (line 2 — 3 string entries: SetCartFrameShown / SetCartShown / \
         OnCartItemInteraction; these are the visual-layer service event names paired \
         with ShoppingCartDataServices and consumed via AddServiceEvents at line 30 of \
         OnLoad)"
    );

    let currency: (String, bool, bool) = env
        .eval(
            "return type(ShoppingCartPlayerTotalCurrencyMixin), \
                    type(ShoppingCartPlayerTotalCurrencyMixin.OnEnter) == 'function', \
                    type(ShoppingCartPlayerTotalCurrencyMixin.OnLeave) == 'function'",
        )
        .expect("Currency mixin probe should succeed");
    assert_eq!(
        currency,
        ("table".to_string(), true, true),
        "Blizzard_GenericShoppingCartTemplates.lua publishes \
         ShoppingCartPlayerTotalCurrencyMixin (line 427 — drives the \
         PlayerTotalCurrencyDisplay frame's OnEnter/OnLeave; OnEnter calls GameTooltip \
         with the localized tooltip text + title, OnLeave hides GameTooltip via \
         GameTooltip_Hide)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_generic_shopping_cart_virtual_templates_do_not_leak_as_globals(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart should load");

    let leaks: (bool, bool, bool, bool, bool) = env
        .eval(
            "return _G.ShoppingCartServiceButtonTemplate == nil, \
                    _G.ShoppingCartPriceTemplate == nil, \
                    _G.ShoppingCartCheckoutButtonTemplate == nil, \
                    _G.ShoppingCartRemoveFromCartButtonTemplate == nil, \
                    _G.ShoppingCartVisualsFrameTemplate == nil",
        )
        .expect("Virtual template leak probe should succeed");
    assert_eq!(
        leaks,
        (true, true, true, true, true),
        "Blizzard_GenericShoppingCartTemplates.xml declares all 5 templates with \
         `virtual=\"true\"` so they MUST NOT leak as `_G.*` globals: \
         ShoppingCartServiceButtonTemplate (Button — base for HideCartButton, \
         ClearCartButton, ViewCartButton, RemoveFromListButton, the dynamically-created \
         PurchaseCartButton), ShoppingCartPriceTemplate (Frame — Price/SalePrice/PriceIcon \
         layout), ShoppingCartCheckoutButtonTemplate (Button inheriting \
         SharedButtonLargeTemplate + ShoppingCartServiceButtonTemplate, embeds a \
         ShoppingCartPriceTemplate as PriceContainer with isLarge=true), \
         ShoppingCartRemoveFromCartButtonTemplate (Frame with hover-coordinated \
         RemoveFromListButton child), ShoppingCartVisualsFrameTemplate (Frame inheriting \
         ResizeLayoutFrame — the canonical cart shell that callers instantiate with \
         CreateFrame(\"Frame\", nil, parent, \"ShoppingCartVisualsFrameTemplate\"))"
    );
}
}

prefork_full_ui_case! {
fn blizzard_generic_shopping_cart_data_manager_init_seeds_cart_and_callbacks(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart should load");

    let init_state: (bool, bool, bool, bool, bool, bool, f64) = env
        .eval(
            "local mgr = CreateFromMixins(ShoppingCartDataManagerMixin); \
             mgr:Init('TestCart'); \
             return type(mgr.cartList) == 'table', \
                    type(mgr.RemovalPredicate) == 'function', \
                    mgr.UpdateCartCallback == nil, \
                    mgr.AddToCartCallback == nil, \
                    mgr.PurchaseCartCallback == nil, \
                    mgr.eventNamespace == 'TestCart', \
                    mgr:GetNumItemsInCart()",
        )
        .expect("Data manager init probe should succeed");
    assert_eq!(
        init_state,
        (true, true, true, true, true, true, 0.0),
        "ShoppingCartDataManagerMixin:Init(eventNamespace) (line 11) must seed cartList = \
         {{}}, install a default RemovalPredicate that compares by `id` field (line 15), \
         leave the 5 cart callbacks (UpdateCart/AddToCart/RemoveFromCart/ClearCart/\
         PurchaseCart) nil so consumers explicitly opt-in via the corresponding Set*Callback \
         setters, store eventNamespace verbatim, and call AddServiceEvents(\
         ShoppingCartDataServices) — the GetNumItemsInCart accessor must return 0 from a \
         freshly-initialized manager"
    );
}
}

prefork_full_ui_case! {
fn blizzard_generic_shopping_cart_data_manager_add_remove_clear_round_trips(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &shopping_cart_toc())
        .expect("Blizzard_GenericShoppingCart should load");

    let round_trip: (f64, f64, f64, f64) = env
        .eval(
            "local mgr = CreateFromMixins(ShoppingCartDataManagerMixin); \
             mgr:Init('TestCart'); \
             mgr:AddToCart({ id = 1, name = 'Apple' }); \
             mgr:AddToCart({ id = 2, name = 'Bread' }); \
             mgr:AddToCart({ id = 3, name = 'Cheese' }); \
             local afterAdd = mgr:GetNumItemsInCart(); \
             mgr:RemoveFromCart({ id = 2 }); \
             local afterRemove = mgr:GetNumItemsInCart(); \
             mgr:ClearCart(); \
             local afterClear = mgr:GetNumItemsInCart(); \
             local firstID = mgr.cartList[1] and mgr.cartList[1].cartID or -1; \
             return afterAdd, afterRemove, afterClear, firstID",
        )
        .expect("Data manager round-trip probe should succeed");
    assert_eq!(
        round_trip.0, 3.0,
        "AddToCart 3x must grow the cart to 3 items"
    );
    assert_eq!(
        round_trip.1, 2.0,
        "RemoveFromCart with the default RemovalPredicate (compare by `id`) must remove \
         the item with id == 2, leaving 2 items"
    );
    assert_eq!(
        round_trip.2, 0.0,
        "ClearCart must wipe the cartList back to {{}} and GetNumItemsInCart must return 0"
    );
    assert_eq!(
        round_trip.3, -1.0,
        "After ClearCart the cartList[1] entry must be nil, proving the table was \
         re-initialized to {{}} (not just emptied via table.remove which would leave the \
         cartID monotonic counter visible)"
    );
}
}
