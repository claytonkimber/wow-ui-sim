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

fn deprecated_auto_complete_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_DeprecatedAutoComplete/Blizzard_DeprecatedAutoComplete.toc")
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
fn blizzard_deprecated_auto_complete_toc_is_minimal_with_no_flags_or_deps() {
    let toc = TocFile::from_file(&deprecated_auto_complete_toc())
        .expect("Blizzard_DeprecatedAutoComplete TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_DeprecatedAutoComplete declares `## LoadOnDemand: 0` — the autocomplete \
         deprecation shims (GetAutoCompleteResults / IsRecognizedName / etc.) must install \
         before any chat-input addon Lua executes that calls them"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_DeprecatedAutoComplete does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_DeprecatedAutoComplete declares NO dependencies — every shim simply \
         forwards to the always-loaded C_AutoComplete namespace + Enum.AutoComplete* tables"
    );

    let toc_text = std::fs::read_to_string(deprecated_auto_complete_toc())
        .expect("Blizzard_DeprecatedAutoComplete TOC should read");
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_DeprecatedAutoComplete omits `## AllowLoad:` — defaults to Game-screen-only \
         (src/toc.rs:311), matching the legacy chat-autocomplete in-game-only API surface"
    );
    assert!(
        !toc_text.contains("## AllowLoadGameType:"),
        "Blizzard_DeprecatedAutoComplete omits `## AllowLoadGameType:` so the shims install \
         on every game type without restriction"
    );
}

#[test]
fn blizzard_deprecated_auto_complete_appears_in_game_discovery_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedAutoComplete");
    assert!(
        in_game,
        "Blizzard_DeprecatedAutoComplete (no AllowLoad flag, defaults to Game-only) should \
         appear in Game-screen auto-discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_DeprecatedAutoComplete");
    assert!(
        !in_login,
        "Blizzard_DeprecatedAutoComplete should NOT appear on the Login / glue screens — \
         autocomplete is an in-game chat-input concept"
    );
}

prefork_full_ui_case! {
fn blizzard_deprecated_auto_complete_loads_without_errors(env: &WowLuaEnv) {

    let addon_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("DeprecatedAutoComplete")
                || message.contains("Deprecated_AutoComplete")
        })
        .cloned()
        .collect();
    assert!(
        addon_errors.is_empty(),
        "Blizzard_DeprecatedAutoComplete emitted Lua errors during Game-screen load:\n  {}",
        addon_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_auto_complete_installs_entry_flag_globals(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return AUTOCOMPLETE_FLAG_IN_GROUP == Enum.AutoCompleteEntryFlag.InGroup \
                and AUTOCOMPLETE_FLAG_IN_GUILD == Enum.AutoCompleteEntryFlag.InGuild \
                and AUTOCOMPLETE_FLAG_FRIEND == Enum.AutoCompleteEntryFlag.Friend \
                and AUTOCOMPLETE_FLAG_BNET == Enum.AutoCompleteEntryFlag.Bnet \
                and AUTOCOMPLETE_FLAG_INTERACTED_WITH == Enum.AutoCompleteEntryFlag.InteractedWith \
                and AUTOCOMPLETE_FLAG_ONLINE == Enum.AutoCompleteEntryFlag.Online \
                and AUTO_COMPLETE_IN_AOI == Enum.AutoCompleteEntryFlag.InAOI \
                and AUTO_COMPLETE_ACCOUNT_CHARACTER == Enum.AutoCompleteEntryFlag.AccountCharacter \
                and AUTO_COMPLETE_RECENT_PLAYER == Enum.AutoCompleteEntryFlag.RecentPlayer",
        )
        .expect("AUTOCOMPLETE_FLAG_* / AUTO_COMPLETE_* global query should succeed");
    assert!(
        installed,
        "Deprecated_AutoComplete.lua line 8-16 should publish the 9 legacy bitmask globals \
         (AUTOCOMPLETE_FLAG_IN_GROUP / AUTOCOMPLETE_FLAG_IN_GUILD / AUTOCOMPLETE_FLAG_FRIEND \
         / AUTOCOMPLETE_FLAG_BNET / AUTOCOMPLETE_FLAG_INTERACTED_WITH / AUTOCOMPLETE_FLAG_ONLINE \
         / AUTO_COMPLETE_IN_AOI / AUTO_COMPLETE_ACCOUNT_CHARACTER / AUTO_COMPLETE_RECENT_PLAYER) \
         from the Enum.AutoCompleteEntryFlag namespace via assertsafe(...). Each must equal \
         the matching Enum.AutoCompleteEntryFlag.* value (src/lua_api/globals/enum_data/\
         game_system.rs:720)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_auto_complete_installs_priority_globals(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return LE_AUTOCOMPLETE_PRIORITY_OTHER == Enum.AutoCompletePriority.Other \
                and LE_AUTOCOMPLETE_PRIORITY_INTERACTED == Enum.AutoCompletePriority.Interacted \
                and LE_AUTOCOMPLETE_PRIORITY_IN_GROUP == Enum.AutoCompletePriority.InGroup \
                and LE_AUTOCOMPLETE_PRIORITY_GUILD == Enum.AutoCompletePriority.Guild \
                and LE_AUTOCOMPLETE_PRIORITY_FRIEND == Enum.AutoCompletePriority.Friend \
                and LE_AUTOCOMPLETE_PRIORITY_ACCOUNT_CHARACTER \
                    == Enum.AutoCompletePriority.AccountCharacter \
                and LE_AUTOCOMPLETE_PRIORITY_ACCOUNT_CHARACTER_SAME_REALM \
                    == Enum.AutoCompletePriority.AccountCharacterSameRealm",
        )
        .expect("LE_AUTOCOMPLETE_PRIORITY_* global query should succeed");
    assert!(
        installed,
        "Deprecated_AutoComplete.lua line 18-24 should publish the 7 legacy LE_* priority \
         globals (LE_AUTOCOMPLETE_PRIORITY_OTHER / LE_AUTOCOMPLETE_PRIORITY_INTERACTED / \
         LE_AUTOCOMPLETE_PRIORITY_IN_GROUP / LE_AUTOCOMPLETE_PRIORITY_GUILD / \
         LE_AUTOCOMPLETE_PRIORITY_FRIEND / LE_AUTOCOMPLETE_PRIORITY_ACCOUNT_CHARACTER / \
         LE_AUTOCOMPLETE_PRIORITY_ACCOUNT_CHARACTER_SAME_REALM) from \
         Enum.AutoCompletePriority via assertsafe(...). Each must equal the matching \
         sequential Enum.AutoCompletePriority.* value (src/lua_api/globals/enum_data/\
         game_system.rs:706)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_auto_complete_installs_function_shims(env: &WowLuaEnv) {

    let installed: bool = env
        .eval(
            "return type(GetAutoCompletePresenceID) == 'function' \
                and type(GetAutoCompleteResults) == 'function' \
                and type(GetAutoCompleteRealms) == 'function' \
                and type(IsRecognizedName) == 'function'",
        )
        .expect("AutoComplete function-shim installation query should succeed");
    assert!(
        installed,
        "Deprecated_AutoComplete.lua line 26-42 should publish 4 forwarding global functions: \
         GetAutoCompletePresenceID(name) → C_AutoComplete.GetAutoCompletePresenceID; \
         GetAutoCompleteResults(name, numResults, cursorPosition, allowFullMatch, \
         includeFlags, excludeFlags) → C_AutoComplete.GetAutoCompleteResults (with the \
         pre-conversion `allowFullMatch = not not allowFullMatch` boolean coercion to match \
         the legacy looser type check); GetAutoCompleteRealms() → \
         C_AutoComplete.GetAutoCompleteRealms; IsRecognizedName(name, includeFlags, \
         excludeFlags) → C_AutoComplete.IsRecognizedName"
    );
}
}

prefork_full_ui_case! {
fn blizzard_deprecated_auto_complete_load_deprecation_fallbacks_cvar_is_default_on(env: &WowLuaEnv) {

    let cvar_on: bool = env
        .eval("return GetCVarBool('loadDeprecationFallbacks')")
        .expect("GetCVarBool query should succeed");
    assert!(
        cvar_on,
        "The `loadDeprecationFallbacks` CVar must default to true (src/cvars.yaml:899 sets \
         '1') so the early-return guard at Deprecated_AutoComplete.lua:4 doesn't bail before \
         any shim is defined. If this CVar flips to false, ALL 9 entry-flag + 7 priority + 4 \
         function shims are skipped and any legacy chat-input addon calling them blows up \
         with `attempt to call a nil value` or `attempt to compare nil with number`"
    );
}
}

#[test]
fn blizzard_deprecated_auto_complete_has_no_xml_or_other_assets() {
    let dir = blizzard_ui_dir().join("Blizzard_DeprecatedAutoComplete");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_DeprecatedAutoComplete dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_xml = entries.iter().any(|n| n.ends_with(".xml"));
    assert!(
        !has_xml,
        "Blizzard_DeprecatedAutoComplete has NO XML files — pure Lua function-shim definitions \
         only. Got entries: {entries:?}"
    );

    let has_runtime_shims = entries.iter().any(|n| n == "Deprecated_AutoComplete.lua");
    assert!(
        has_runtime_shims,
        "Blizzard_DeprecatedAutoComplete should ship `Deprecated_AutoComplete.lua` (the runtime \
         shim definitions for the 16 deprecated globals + 4 function shims)"
    );
}
