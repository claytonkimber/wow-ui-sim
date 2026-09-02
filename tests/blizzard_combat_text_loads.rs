#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::discover_blizzard_addons_for_screen;
use wow_ui_sim::loader::load_addon;
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn combat_text_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_CombatText/Blizzard_CombatText.toc")
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
fn blizzard_combat_text_toc_is_load_on_demand() {
    let toc = TocFile::from_file(&combat_text_toc()).expect("CombatText TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_CombatText declares `## LoadOnDemand: 1` — it is loaded at runtime by \
         `Settings.LoadAddOnCVarWatcher(\"enableFloatingCombatText\", \"Blizzard_CombatText\")` \
         only when the user has the floating-combat-text CVar enabled"
    );
}

#[test]
fn blizzard_combat_text_is_not_in_game_auto_discovery() {
    let ui = blizzard_ui_dir();
    let addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);

    let auto_loaded = addons.iter().any(|(name, _)| name == "Blizzard_CombatText");
    assert!(
        !auto_loaded,
        "Blizzard_CombatText is `## LoadOnDemand: 1` and must NOT appear in Game-screen \
         auto-discovery — it is loaded explicitly via `UIParentLoadAddOn` from \
         `Settings.LoadAddOnCVarWatcher` when the floating-combat-text CVar is on"
    );
}

prefork_full_ui_case! {
fn blizzard_combat_text_loads_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &combat_text_toc())
        .expect("Blizzard_CombatText should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_CombatText emitted Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_text_constants_are_populated(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &combat_text_toc()).expect("Blizzard_CombatText should load");

    let constants_present: bool = env
        .eval(
            "return type(CombatTextConstants) == 'table' \
                and CombatTextConstants.NumCombatTextLines == 20 \
                and CombatTextConstants.MessageScrollSpeed == 1.9 \
                and CombatTextConstants.MessageFadeOutTime == 1.3 \
                and CombatTextConstants.MessageHeight == 25 \
                and CombatTextConstants.CriticalHitMaxHeight == 60 \
                and CombatTextConstants.CriticalHitMinHeight == 30 \
                and CombatTextConstants.CriticalHitScaleTime == 0.05 \
                and CombatTextConstants.CriticalHitShrinkTime == 0.2 \
                and CombatTextConstants.StaggerRange == 20 \
                and CombatTextConstants.LowHealthThreshold == 0.2 \
                and CombatTextConstants.LowManaThreshold == 0.2",
        )
        .expect("CombatTextConstants query should succeed");
    assert!(
        constants_present,
        "CombatTextConstants should expose its 11 documented numeric fields after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_text_type_info_is_populated_with_mainline_overrides(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &combat_text_toc()).expect("Blizzard_CombatText should load");

    let type_info_present: bool = env
        .eval(
            "return type(CombatTextTypeInfo) == 'table' \
                and type(CombatTextTypeInfo.DAMAGE) == 'table' \
                and CombatTextTypeInfo.DAMAGE.r == 1 and CombatTextTypeInfo.DAMAGE.show == 1 \
                and CombatTextTypeInfo.DAMAGE.isStaggered == 1 \
                and type(CombatTextTypeInfo.HEAL) == 'table' \
                and CombatTextTypeInfo.HEAL.g == 1 \
                and type(CombatTextTypeInfo.SPELL_DAMAGE) == 'table' \
                and type(CombatTextTypeInfo.PERIODIC_HEAL) == 'table' \
                and type(CombatTextTypeInfo.HEAL_CRIT) == 'table' \
                and type(CombatTextTypeInfo.REFLECT) == 'table' \
                and type(CombatTextTypeInfo.SPELL_DAMAGE_CRIT) == 'table' \
                and type(CombatTextTypeInfo.PERIODIC_ENERGIZE) == 'table' \
                and type(CombatTextTypeInfo.SPELL_AURA_END) == 'table' \
                and type(CombatTextTypeInfo.SPELL_AURA_END_HARMFUL) == 'table' \
                and type(CombatTextTypeInfo.DAMAGE_SHIELD) == 'table' \
                and type(CombatTextTypeInfo.PLUNDER_UPDATE) == 'table' \
                and CombatTextTypeInfo.PLUNDER_UPDATE.show == 1",
        )
        .expect("CombatTextTypeInfo query should succeed");
    assert!(
        type_info_present,
        "CombatTextTypeInfo should expose the Shared event-color entries (DAMAGE/HEAL/SPELL_*) \
         AND the Mainline overrides (REFLECT, SPELL_DAMAGE_CRIT, PERIODIC_ENERGIZE, \
         SPELL_AURA_END, SPELL_AURA_END_HARMFUL, DAMAGE_SHIELD, PLUNDER_UPDATE) added by \
         CombatTextConstantsOverrides.lua"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_text_lookup_tables_are_populated(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &combat_text_toc()).expect("Blizzard_CombatText should load");

    let lookups_present: bool = env
        .eval(
            "return type(CombatTextPowerEnumFromEnergizeStringLookup) == 'table' \
                and CombatTextPowerEnumFromEnergizeStringLookup.MANA == Enum.PowerType.Mana \
                and CombatTextPowerEnumFromEnergizeStringLookup.RAGE == Enum.PowerType.Rage \
                and CombatTextPowerEnumFromEnergizeStringLookup.HOLY_POWER == Enum.PowerType.HolyPower \
                and type(CombatTextFrameEvents) == 'table' \
                and CombatTextFrameEvents.COMBAT_TEXT_UPDATE == true \
                and CombatTextFrameEvents.UNIT_HEALTH == true \
                and CombatTextFrameEvents.UNIT_POWER_UPDATE == true \
                and CombatTextFrameEvents.PLAYER_REGEN_DISABLED == true \
                and CombatTextFrameEvents.CURRENCY_DISPLAY_UPDATE == true \
                and type(CombatTextCachableCVars) == 'table' \
                and CombatTextCachableCVars.floatingCombatTextLowManaHealth_v2 == true",
        )
        .expect("CombatText lookup tables query should succeed");
    assert!(
        lookups_present,
        "CombatTextPowerEnumFromEnergizeStringLookup, CombatTextFrameEvents (with the Mainline \
         CURRENCY_DISPLAY_UPDATE entry added by the override file), and CombatTextCachableCVars \
         should all be populated after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_text_util_helpers_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &combat_text_toc()).expect("Blizzard_CombatText should load");

    let util_present: bool = env
        .eval(
            "return type(CombatTextUtil) == 'table' \
                and type(CombatTextUtil.StandardScroll) == 'function' \
                and type(CombatTextUtil.FountainScroll) == 'function' \
                and type(CombatTextUtil.GetPowerEnumFromEnergizeString) == 'function' \
                and type(CombatTextUtil.RegisterCachableCVars) == 'function' \
                and type(CombatTextUtil.UpdateEventRegistration) == 'function' \
                and type(CombatTextUtil.GetFormattedBlockMessage) == 'function' \
                and type(CombatTextUtil.GetBasicPowerTypeColor) == 'function' \
                and type(CombatTextUtil.GetComboPointsMessageInfo) == 'function' \
                and type(CombatTextUtil.GetRunePowerUpdateMessage) == 'function'",
        )
        .expect("CombatTextUtil query should succeed");
    assert!(
        util_present,
        "CombatTextUtil and its 9 documented helpers (StandardScroll/FountainScroll/\
         GetPowerEnumFromEnergizeString/RegisterCachableCVars/UpdateEventRegistration/\
         GetFormattedBlockMessage/GetBasicPowerTypeColor/GetComboPointsMessageInfo/\
         GetRunePowerUpdateMessage) should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_text_get_power_enum_resolves_known_strings(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &combat_text_toc()).expect("Blizzard_CombatText should load");

    let resolves_correctly: bool = env
        .eval(
            "return CombatTextUtil.GetPowerEnumFromEnergizeString('MANA') == Enum.PowerType.Mana \
                and CombatTextUtil.GetPowerEnumFromEnergizeString('HOLY_POWER') \
                    == Enum.PowerType.HolyPower \
                and CombatTextUtil.GetPowerEnumFromEnergizeString('UNKNOWN_FAKE_POWER') \
                    == Enum.PowerType.NumPowerTypes",
        )
        .expect("GetPowerEnumFromEnergizeString query should succeed");
    assert!(
        resolves_correctly,
        "GetPowerEnumFromEnergizeString should look up known energize strings in \
         CombatTextPowerEnumFromEnergizeStringLookup, and fall back to \
         Enum.PowerType.NumPowerTypes for unknown inputs"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_text_mixin_methods_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &combat_text_toc()).expect("Blizzard_CombatText should load");

    let mixin_present: bool = env
        .eval(
            "return type(CombatTextMixin) == 'table' \
                and type(CombatTextMixin.OnLoad) == 'function' \
                and type(CombatTextMixin.OnEvent) == 'function' \
                and type(CombatTextMixin.OnUpdate) == 'function' \
                and type(CombatTextMixin.AddMessage) == 'function' \
                and type(CombatTextMixin.EnumerateActiveFontStrings) == 'function' \
                and type(CombatTextMixin.ReleaseFontString) == 'function' \
                and type(CombatTextMixin.AcquireFontString) == 'function' \
                and type(CombatTextMixin.InitializeFontString) == 'function' \
                and type(CombatTextMixin.ClearAnimationList) == 'function' \
                and type(CombatTextMixin.UpdateDisplayedMessages) == 'function'",
        )
        .expect("CombatTextMixin query should succeed");
    assert!(
        mixin_present,
        "CombatTextMixin and its 10 methods (OnLoad/OnEvent/OnUpdate/AddMessage/\
         EnumerateActiveFontStrings/ReleaseFontString/AcquireFontString/InitializeFontString/\
         ClearAnimationList/UpdateDisplayedMessages) should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_text_toplevel_frame_is_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &combat_text_toc()).expect("Blizzard_CombatText should load");

    let frame_present: bool = env
        .eval(
            "return _G.CombatText ~= nil \
                and CombatText:GetParent() == UIParent",
        )
        .expect("CombatText frame query should succeed");
    assert!(
        frame_present,
        "CombatText (toplevel Frame, mixin=CombatTextMixin, parent=UIParent) should be defined \
         and parented to UIParent after load"
    );
}
}
