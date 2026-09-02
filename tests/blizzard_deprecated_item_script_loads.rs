#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn deprecated_item_script_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeprecatedItemScript/Blizzard_DeprecatedItemScript.toc")
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
fn blizzard_deprecated_item_script_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_item_script_toc())
        .expect("Blizzard_DeprecatedItemScript TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedItemScript declares `## LoadOnDemand: 0` — the 47 deprecated \
         item-script globals (GetItemQualityColor / GetItemInfoInstant / IsArtifactPowerItem / \
         IsEquippableItem / etc.) must install before any item / inventory / equipment addon \
         Lua executes that calls them"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedItemScript does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedItemScript declares NO dependencies — every shim simply forwards \
         to the C_Item namespace, which is registered by the simulator's c_api/item_spell/\
         c_item.rs at env init time and gets `__wow_namespace_mt` attached by \
         __wow_seed_namespace_names() (runtime_surface_bootstrap.lua:11929) before any addon \
         loads"
    );

    let toc_text = std::fs::read_to_string(deprecated_item_script_toc())
        .expect("Blizzard_DeprecatedItemScript TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedItemScript omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311), matching the legacy item-API in-game-only surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedItemScript omits `## AllowLoadGameType:` so the shims install on \
         every game type without restriction"
    );
}

#[test]
fn blizzard_deprecated_item_script_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedItemScript");
    assert!(
        in_game,
        "Blizzard_DeprecatedItemScript (no AllowLoad flag, defaults to Game-only) should \
         appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedItemScript");
    assert!(
        !in_login,
        "Blizzard_DeprecatedItemScript should NOT appear on the Login / glue screens — \
         item / inventory / equipment APIs are an in-game concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_item_script_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedItemScript") || message.contains("Deprecated_ItemScript")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedItemScript emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_item_script_installs_all_47_function_shims(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return type(GetItemQualityColor) == 'function' \
                and type(GetItemInfoInstant) == 'function' \
                and type(GetItemSetInfo) == 'function' \
                and type(GetItemChildInfo) == 'function' \
                and type(DoesItemContainSpec) == 'function' \
                and type(GetItemGem) == 'function' \
                and type(GetItemCreationContext) == 'function' \
                and type(GetItemIcon) == 'function' \
                and type(GetItemFamily) == 'function' \
                and type(GetItemSpell) == 'function' \
                and type(IsArtifactPowerItem) == 'function' \
                and type(IsCurrentItem) == 'function' \
                and type(IsUsableItem) == 'function' \
                and type(IsHelpfulItem) == 'function' \
                and type(IsHarmfulItem) == 'function' \
                and type(IsConsumableItem) == 'function' \
                and type(IsEquippableItem) == 'function' \
                and type(IsEquippedItem) == 'function' \
                and type(IsEquippedItemType) == 'function' \
                and type(ItemHasRange) == 'function' \
                and type(IsItemInRange) == 'function' \
                and type(GetItemClassInfo) == 'function' \
                and type(GetItemInventorySlotInfo) == 'function' \
                and type(BindEnchant) == 'function' \
                and type(ActionBindsItem) == 'function' \
                and type(ReplaceEnchant) == 'function' \
                and type(ReplaceTradeEnchant) == 'function' \
                and type(ConfirmBindOnUse) == 'function' \
                and type(ConfirmOnUse) == 'function' \
                and type(ConfirmNoRefundOnUse) == 'function' \
                and type(DropItemOnUnit) == 'function' \
                and type(EndBoundTradeable) == 'function' \
                and type(EndRefund) == 'function' \
                and type(GetItemInfo) == 'function' \
                and type(GetDetailedItemLevelInfo) == 'function' \
                and type(GetItemSpecInfo) == 'function' \
                and type(GetItemUniqueness) == 'function' \
                and type(GetItemCount) == 'function' \
                and type(PickupItem) == 'function' \
                and type(GetItemSubClassInfo) == 'function' \
                and type(UseItemByName) == 'function' \
                and type(EquipItemByName) == 'function' \
                and type(ReplaceTradeskillEnchant) == 'function' \
                and type(GetItemCooldown) == 'function' \
                and type(IsCorruptedItem) == 'function' \
                and type(IsCosmeticItem) == 'function' \
                and type(IsDressableItem) == 'function'",
        )
        .expect("47-shim function-installation query should succeed");
    assert!(
        installed,
        "Deprecated_ItemScript.lua line 9-55 should publish 47 forwarding global functions. \
         The 29 explicitly registered C_Item methods (src/c_api/item_spell/c_item.rs — \
         GetItemInfoInstant, GetItemIcon (registered as GetItemIconByID under that name), \
         GetItemInfo, GetItemCount, GetItemCooldown, IsConsumableItem, IsEquippableItem, \
         IsItemInRange, etc.) bind directly. The remaining ~18 unstubbed methods \
         (IsArtifactPowerItem, GetItemQualityColor, BindEnchant, etc.) resolve via the \
         `__wow_namespace_mt.__index` metamethod \
         (runtime_surface_bootstrap.lua:2019-2028) which lazily materializes a no-op \
         `function() return nil end` on first read and caches it via `rawset`. \
         __wow_seed_namespace_names() at line 11929 attaches this metatable to all C_* \
         tables (including C_Item) at env init, BEFORE the deprecation shim runs. Result: \
         every alias is a callable function, not nil"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_item_script_get_item_icon_aliases_get_item_icon_by_id(env: &WowLuaEnv) {

    let aliased: bool = env
        .eval("return GetItemIcon == C_Item.GetItemIconByID")
        .expect("GetItemIcon == C_Item.GetItemIconByID query should succeed");
    assert!(
        aliased,
        "Deprecated_ItemScript.lua line 16 reads `GetItemIcon = C_Item.GetItemIconByID;` — \
         note the RENAMED right-hand side: legacy global `GetItemIcon` is bound to the new \
         API name `C_Item.GetItemIconByID`, not `C_Item.GetItemIcon`. This is the only \
         rename in the entire shim file; the other 46 aliases use the same name on both \
         sides. Identity equality confirms the rename"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_item_script_globals_alias_c_item_methods_by_identity(env: &WowLuaEnv) {

    let aliases_match: bool = env
        .eval(
            "return GetItemQualityColor == C_Item.GetItemQualityColor \
                and GetItemInfoInstant == C_Item.GetItemInfoInstant \
                and GetItemSetInfo == C_Item.GetItemSetInfo \
                and IsArtifactPowerItem == C_Item.IsArtifactPowerItem \
                and IsEquippableItem == C_Item.IsEquippableItem \
                and IsConsumableItem == C_Item.IsConsumableItem \
                and IsItemInRange == C_Item.IsItemInRange \
                and GetItemSpell == C_Item.GetItemSpell \
                and BindEnchant == C_Item.BindEnchant \
                and PickupItem == C_Item.PickupItem \
                and GetItemCooldown == C_Item.GetItemCooldown \
                and IsCorruptedItem == C_Item.IsCorruptedItem \
                and GetItemInfo == C_Item.GetItemInfo \
                and GetItemCount == C_Item.GetItemCount",
        )
        .expect("identity-equality query for 14 representative aliases should succeed");
    assert!(
        aliases_match,
        "Each deprecated global must reference identity-equal values to its backing C_Item \
         method. For unstubbed methods (e.g. IsArtifactPowerItem), the namespace __index \
         caches the no-op closure via `rawset(t, key, fn)` so subsequent reads return the \
         SAME function — making both sides identity-equal. For registered methods (e.g. \
         GetItemInfoInstant), both sides see the same Rust-bound function value"
    );

    let modeled_results: bool = env
        .eval(
            "return C_Item.IsEquippableItem(0) == false \
                and C_Item.IsConsumableItem(0) == false \
                and C_Item.IsItemInRange(0, 'player') == true",
        )
        .expect("modeled C_Item query results should succeed");
    assert!(
        modeled_results,
        "The three registered C_Item methods must retain the existing legacy-global \
         SimState behavior; namespace fallback closures return nil instead"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_item_script_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_ItemScript.lua:4 doesn't bail before \
         the 47 globals are defined. If this CVar flips to false, ALL 47 are skipped and \
         legacy item / inventory / equipment addons calling GetItemInfo / IsEquippableItem / \
         IsArtifactPowerItem / etc. blow up with `attempt to call a nil value`"
    );
}
}

#[test]
fn blizzard_deprecated_item_script_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedItemScript");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedItemScript dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedItemScript has NO XML files — pure Lua function-shim \
         definitions only. Got entries: {entries:?}"
    );

    let has_runtime_shims = entries.iter().any(|n| n == "Deprecated_ItemScript.lua");
    assert!(
        has_runtime_shims,
        "Blizzard_DeprecatedItemScript should ship `Deprecated_ItemScript.lua` (the runtime \
         shim definitions for the 47 deprecated item-script globals)"
    );
}
