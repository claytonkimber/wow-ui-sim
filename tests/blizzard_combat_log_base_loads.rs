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
fn blizzard_combat_log_base_toc_is_load_on_demand() {
    let toc_path = blizzard_ui_dir().join("Blizzard_CombatLogBase/Blizzard_CombatLogBase.toc");
    let toc = TocFile::from_file(&toc_path).expect("CombatLogBase TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_CombatLogBase declares `## LoadOnDemand: 1` — even though the simulator's \
         dependency-pull pre-emptively materializes it for Game-screen discovery, the TOC \
         flag itself is the source of truth and matches Blizzard's own LOD policy"
    );
    let deps = toc.dependencies();
    assert!(
        deps.contains(&"Blizzard_SharedXMLBase".to_string()),
        "Blizzard_CombatLogBase should declare `## Dependencies: Blizzard_SharedXMLBase` \
         (so its `Enum.CombatLogObject` references resolve), got {deps:?}"
    );
}

#[test]
fn blizzard_combat_log_base_appears_in_game_discovery_via_dependency_pull() {
    let ui = blizzard_ui_dir();
    let addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);

    let pulled_in = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CombatLogBase");
    assert!(
        pulled_in,
        "Blizzard_CombatLogBase is LOD but should be pulled into the Game-screen discovery \
         set by the loader's LOD-dependency promotion (its filter constants and CombatLogUtil \
         helpers are referenced by other Blizzard addons during boot)"
    );
}

prefork_full_ui_case! {
fn blizzard_combat_log_base_is_loaded_as_combatlog_dependency(env: &WowLuaEnv) {

    let already_loaded: bool = env
        .eval(
            "return type(C_AddOns) == 'table' \
                and type(C_AddOns.IsAddOnLoaded) == 'function' \
                and (C_AddOns.IsAddOnLoaded('Blizzard_CombatLogBase') and true or false)",
        )
        .expect("IsAddOnLoaded query should succeed");
    assert!(
        already_loaded,
        "Blizzard_CombatLogBase should be loaded by the time the Game UI is up — it is the \
         declared `## Dependencies:` of Blizzard_CombatLog (which Blizzard_ChatFrameBase \
         runtime-LoadAddOns) and Blizzard_CombatLogProcessor"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_base_loads_without_errors(env: &WowLuaEnv) {

    let base_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| message.contains("Blizzard_CombatLogBase"))
        .cloned()
        .collect();
    assert!(
        base_errors.is_empty(),
        "Blizzard_CombatLogBase emitted Lua errors during load:\n  {}",
        base_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_base_filter_constants_are_populated(env: &WowLuaEnv) {

    let filters_present: bool = env
        .eval(
            "return type(COMBATLOG_FILTER_ME) == 'number' \
                and type(COMBATLOG_FILTER_MINE) == 'number' \
                and type(COMBATLOG_FILTER_MY_PET) == 'number' \
                and type(COMBATLOG_FILTER_FRIENDLY_UNITS) == 'number' \
                and type(COMBATLOG_FILTER_HOSTILE_PLAYERS) == 'number' \
                and type(COMBATLOG_FILTER_HOSTILE_UNITS) == 'number' \
                and type(COMBATLOG_FILTER_NEUTRAL_UNITS) == 'number' \
                and COMBATLOG_FILTER_UNKNOWN_UNITS == Enum.CombatLogObject.None \
                and COMBATLOG_FILTER_EVERYTHING == bit.bnot(0)",
        )
        .expect("filter-constant query should succeed");
    assert!(
        filters_present,
        "All nine COMBATLOG_FILTER_* bitmask constants from \
         Blizzard_CombatLogBase/Shared/CombatLogFilters.lua should be populated"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_base_mainline_constants_are_populated(env: &WowLuaEnv) {

    let constants_present: bool = env
        .eval(
            "return type(COMBAT_LOG_POWER_TYPE_STRINGS) == 'table' \
                and COMBAT_LOG_POWER_TYPE_STRINGS[Enum.PowerType.Mana] == MANA \
                and COMBAT_LOG_POWER_TYPE_STRINGS[Enum.PowerType.Rage] == RAGE \
                and COMBAT_LOG_POWER_TYPE_STRINGS[Enum.PowerType.Energy] == ENERGY \
                and COMBAT_LOG_POWER_TYPE_STRINGS[Enum.PowerType.HolyPower] == HOLY_POWER \
                and COMBAT_LOG_SCHOOL_MASK_NONE == Enum.Damageclass.MaskNone",
        )
        .expect("Mainline-constants query should succeed");
    assert!(
        constants_present,
        "COMBAT_LOG_POWER_TYPE_STRINGS should map every PowerType to its localized string and \
         COMBAT_LOG_SCHOOL_MASK_NONE should equal Enum.Damageclass.MaskNone"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_base_default_colors_are_populated(env: &WowLuaEnv) {

    let colors_present: bool = env
        .eval(
            "return type(COMBATLOG_DEFAULT_COLORS) == 'table' \
                and type(COMBATLOG_DEFAULT_COLORS.unitColoring) == 'table' \
                and type(COMBATLOG_DEFAULT_COLORS.schoolColoring) == 'table' \
                and type(COMBATLOG_DEFAULT_COLORS.defaults) == 'table' \
                and type(COMBATLOG_DEFAULT_COLORS.eventColoring) == 'table' \
                and type(COMBATLOG_DEFAULT_COLORS.highlightedEvents) == 'table' \
                and COMBATLOG_DEFAULT_COLORS.highlightedEvents.PARTY_KILL == true \
                and COMBATLOG_DEFAULT_COLORS.unitColoring[COMBATLOG_FILTER_MINE] ~= nil \
                and COMBATLOG_DEFAULT_COLORS.schoolColoring[Enum.Damageclass.MaskFire] ~= nil",
        )
        .expect("default-colors query should succeed");
    assert!(
        colors_present,
        "COMBATLOG_DEFAULT_COLORS should expose unitColoring/schoolColoring/defaults/\
         eventColoring/highlightedEvents subtables with the documented entries"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_base_combat_log_util_methods_are_defined(env: &WowLuaEnv) {

    let util_present: bool = env
        .eval(
            "return type(CombatLogUtil) == 'table' \
                and type(CombatLogUtil.WrapTextInColor) == 'function' \
                and type(CombatLogUtil.GetColorByEventType) == 'function' \
                and type(CombatLogUtil.GetColorByUnitType) == 'function' \
                and type(CombatLogUtil.GetColorBySchool) == 'function' \
                and type(CombatLogUtil.HighlightColor) == 'function' \
                and type(CombatLogUtil.GetRaidTargetIcon) == 'function' \
                and type(CombatLogUtil.GetUnitIcon) == 'function' \
                and type(CombatLogUtil.GetRaidTargetBraceCode) == 'function' \
                and type(CombatLogUtil.GetPowerTypeString) == 'function' \
                and type(CombatLogUtil.GetSpellSchoolString) == 'function' \
                and type(CombatLogUtil.GenerateDamageResultString) == 'function'",
        )
        .expect("CombatLogUtil query should succeed");
    assert!(
        util_present,
        "CombatLogUtil and its 11 methods (WrapTextInColor/GetColorByEventType/\
         GetColorByUnitType/GetColorBySchool/HighlightColor/GetRaidTargetIcon/GetUnitIcon/\
         GetRaidTargetBraceCode/GetPowerTypeString/GetSpellSchoolString/\
         GenerateDamageResultString) should be defined after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_base_wrap_text_in_color_works(env: &WowLuaEnv) {

    let wrapped: String = env
        .eval("return CombatLogUtil.WrapTextInColor('hi', {r = 1.0, g = 0.5, b = 0.25, a = 1.0})")
        .expect("WrapTextInColor query should succeed");
    assert_eq!(
        wrapped, "|cffff7f3fhi|r",
        "WrapTextInColor should emit `|cffRRGGBBtext|r` with channels scaled to 0–255"
    );
}
}

prefork_full_ui_case! {
fn blizzard_combat_log_base_raid_target_brace_code_is_localized(env: &WowLuaEnv) {

    let bracketed: bool = env
        .eval(
            "local code1 = CombatLogUtil.GetRaidTargetBraceCode(Enum.CombatLogObjectTarget.Raidtarget1); \
             local empty = CombatLogUtil.GetRaidTargetBraceCode(nil); \
             return type(code1) == 'string' and code1 ~= '' \
                 and code1:sub(1,1) == '{' and code1:sub(-1) == '}' \
                 and empty == ''",
        )
        .expect("GetRaidTargetBraceCode query should succeed");
    assert!(
        bracketed,
        "GetRaidTargetBraceCode(Raidtarget1) should return a brace-wrapped lowercased \
         RAID_TARGET_1 token, and GetRaidTargetBraceCode(nil) should return the empty string"
    );
}
}
