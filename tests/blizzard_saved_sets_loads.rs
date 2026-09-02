use std::path::PathBuf;

use wow_ui_sim::loader::load_addon;
use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn saved_sets_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_SavedSets")
}

fn saved_sets_toc() -> PathBuf {
    saved_sets_dir().join("Blizzard_SavedSets.toc")
}

const TOC_FILES: &[&str] = &["Blizzard_SavedSets.lua"];

const SAVED_SETS_UTIL_METHODS: &[&str] = &[
    "IsLoaded",
    "ContinueOnLoad",
    "HasAny",
    "Set",
    "Check",
    "Empty",
    "Print",
];

const SHORT_FORM_HELPERS: &[&str] = &[
    "SavedSet_IsLoaded",
    "SavedSet_HasAny",
    "SavedSet_Set",
    "SavedSet_Check",
];

const MODULE_LOCAL_NAMES: &[&str] = &["loaded", "pendingCallbacks", "TriggerPendingCallbacks"];

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
fn find_toc_file_resolves_bare_toc() {
    let resolved = find_toc_file(&saved_sets_dir()).expect("Blizzard_SavedSets TOC resolves");
    assert_eq!(
        resolved,
        saved_sets_toc(),
        "Blizzard_SavedSets ships exactly one bare TOC — no flavor split. The addon \
         is a pure account-wide persistence helper around a single SavedVariable, \
         engine-flavor-agnostic by construction"
    );
}

#[test]
fn toc_declares_eager_on_both_screens_with_account_wide_saved_variable() {
    let toc = TocFile::from_file(&saved_sets_toc()).expect("Blizzard_SavedSets TOC parses");

    assert!(
        !toc.is_load_on_demand(),
        "TOC declares `## LoadOnDemand: 0` (explicit zero — eager) so consumers \
         (Blizzard_StoreUI for SeenShopCatalogProductIDs, Blizzard_HousingFrame for \
         SeenHousingMarketDecorIDs) can call `SavedSetsUtil.HasAny(...)` synchronously \
         at OnLoad time without queueing a deferred callback"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());

    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            toc.allows_screen(screen),
            "`## AllowLoad: Both` must enable {screen:?} — `allows_screen` at \
             src/toc.rs:307 returns true for every ScreenKind when AllowLoad is `Both`. \
             Glue-screen consumers (StoreUI catalog new-item glow on character select) \
             need to query the same persisted set as the in-game store"
        );
    }

    let deps = toc.dependencies();
    assert_eq!(
        deps,
        vec!["Blizzard_SharedXML".to_string()],
        "TOC must declare exactly one hard `## Dependencies: Blizzard_SharedXML`. \
         The addon body calls SharedXML primitives `GetOrCreateTableEntry`, \
         `TableIsEmpty`, and `EventUtil.ContinueOnVariablesLoaded` at module load time"
    );
    assert!(
        toc.optional_deps().is_empty(),
        "Zero `## OptionalDeps:` — pure leaf in the dependency graph"
    );

    let saved_vars = toc.saved_variables();
    assert_eq!(
        saved_vars,
        vec!["GlobalBlizzardSavedSets".to_string()],
        "TOC must declare exactly one `## SavedVariables: GlobalBlizzardSavedSets`. \
         Account-wide persistence (NOT per-character — there is no \
         `## SavedVariablesPerCharacter`) is the whole point of this addon. The \
         single global table holds nested keys (SeenShopCatalogProductIDs, \
         SeenHousingMarketDecorIDs) so a single SavedVariables entry covers every \
         registered set"
    );
    assert!(toc.saved_variables_per_character().is_empty());
}

#[test]
fn toc_declares_metadata_in_raw_bytes_with_descriptive_comment() {
    let raw =
        std::fs::read_to_string(saved_sets_toc()).expect("Blizzard_SavedSets TOC reads utf-8");
    assert!(
        raw.contains("## Title: Blizzard Saved Sets"),
        "TOC must declare `## Title: Blizzard Saved Sets` exactly (space-and-prose form)"
    );
    assert!(
        raw.contains("## For saving data persistently on a per-account basis."),
        "TOC must carry the descriptive comment line `## For saving data persistently \
         on a per-account basis.` verbatim — `## ` followed by free-form prose is \
         legal TOC syntax (treated as a comment by `parse_toc_metadata` because the \
         text after `## ` lacks a `:` separator). The comment exists to clarify the \
         account-wide scope, contrasting it with addons that use \
         `## SavedVariablesPerCharacter`"
    );
    assert!(raw.contains("## SavedVariables: GlobalBlizzardSavedSets"));
    assert!(raw.contains("## LoadOnDemand: 0"));
    assert!(raw.contains("## AllowLoad: Both"));
    assert!(raw.contains("## Dependencies: Blizzard_SharedXML"));
    assert!(
        !raw.contains("## SavedVariablesPerCharacter"),
        "TOC must NOT declare per-character SVs — the only SV is account-wide"
    );
    assert!(
        !raw.contains("## OptionalDep"),
        "TOC must NOT declare any `## OptionalDep` keys"
    );
    assert!(
        !raw.contains("## Author"),
        "TOC must NOT declare `## Author:` — relies on implicit Blizzard ownership"
    );
}

#[test]
fn toc_lists_single_lua_file_with_no_xml() {
    let toc = TocFile::from_file(&saved_sets_toc()).expect("Blizzard_SavedSets TOC parses");
    let listed: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        listed, TOC_FILES,
        "TOC body must list exactly 1 file: Blizzard_SavedSets.lua. The addon is \
         pure-Lua persistence glue with NO XML — no frames, no templates, no script \
         handlers wired through XML; just a global-scope `SavedSetsUtil` table and a \
         single `EventUtil.ContinueOnVariablesLoaded` callback registration"
    );
}

#[test]
fn appears_on_every_screen_eager_discovery() {
    let ui = blizzard_ui_dir();

    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&ui, screen);
        let found = addons.iter().any(|(name, _)| name == "Blizzard_SavedSets");
        assert!(
            found,
            "Blizzard_SavedSets must auto-discover on screen {screen:?} — eager \
             (LoadOnDemand: 0) AND `## AllowLoad: Both` makes \
             `discover_blizzard_addons_for_screen` include the addon on every screen"
        );
    }
}

#[test]
fn root_directory_holds_single_lua_next_to_toc() {
    let dir = saved_sets_dir();
    assert!(dir.join("Blizzard_SavedSets.lua").is_file());
    assert!(dir.join("Blizzard_SavedSets.toc").is_file());

    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("read addon dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        2,
        "Blizzard_SavedSets directory must contain exactly 2 entries (1 lua + 1 toc), \
         got {entries:?}. No XML, no nested directories, no extra Lua files"
    );
}

prefork_full_ui_case! {
fn loads_without_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_SavedSets")
                || message.contains("SavedSetsUtil")
                || message.contains("GlobalBlizzardSavedSets")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_SavedSets emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn is_addon_loaded_after_eager_sweep(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_SavedSets')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_SavedSets') must return true after the eager \
         Game-screen sweep — LoadOnDemand: 0 puts the addon in the eager set"
    );
}
}

prefork_full_ui_case! {
fn saved_sets_util_publishes_with_full_method_surface(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G.SavedSetsUtil)")
        .expect("SavedSetsUtil type probe succeeds");
    assert_eq!(
        kind, "table",
        "_G.SavedSetsUtil must publish as a table — the public namespace declared at \
         Blizzard_SavedSets.lua:7 (`SavedSetsUtil = {{}}`)"
    );

    for method in SAVED_SETS_UTIL_METHODS {
        let method_kind: String = env
            .eval(&format!("return type(SavedSetsUtil.{method})"))
            .unwrap_or_else(|err| panic!("type(SavedSetsUtil.{method}) probe failed: {err}"));
        assert_eq!(
            method_kind, "function",
            "SavedSetsUtil.{method} must be a function — the namespace publishes 7 \
             public helpers: IsLoaded (returns the file-local `loaded` flag), \
             ContinueOnLoad (queues callback into `pendingCallbacks` table or \
             dispatches immediately), HasAny (returns true pre-load to keep \
             new-item-glow visible until persistence catches up), Set (gates writes \
             behind ContinueOnLoad and supports both single-id and id-table forms), \
             Check (mirrors Set's overload — returns nil for unknown single id, all \
             present for table form), Empty (clears a saved-set key), Print (dumps \
             the saved-set's contents to the chat frame for debugging)"
        );
    }

    let registered_kind: String = env
        .eval("return type(SavedSetsUtil.RegisteredSavedSets)")
        .expect("RegisteredSavedSets probe succeeds");
    assert_eq!(
        registered_kind, "table",
        "SavedSetsUtil.RegisteredSavedSets must publish as a table"
    );
}
}

prefork_full_ui_case! {
fn registered_saved_sets_holds_two_self_referential_string_values(env: &WowLuaEnv) {

    let shop_key: String = env
        .eval("return SavedSetsUtil.RegisteredSavedSets.SeenShopCatalogProductIDs")
        .expect("SeenShopCatalogProductIDs probe succeeds");
    assert_eq!(
        shop_key, "SeenShopCatalogProductIDs",
        "RegisteredSavedSets.SeenShopCatalogProductIDs must equal the literal string \
         'SeenShopCatalogProductIDs' — the table is a controlled-vocabulary registry \
         where the key and value match. This pattern lets consumer addons reference \
         the canonical key as `SavedSetsUtil.RegisteredSavedSets.SeenShopCatalogProductIDs` \
         instead of an inline string literal, catching typos at parse time \
         (RegisteredSavedSets.Typo would resolve to nil) while keeping the persisted \
         GlobalBlizzardSavedSets table key as a plain string"
    );

    let housing_key: String = env
        .eval("return SavedSetsUtil.RegisteredSavedSets.SeenHousingMarketDecorIDs")
        .expect("SeenHousingMarketDecorIDs probe succeeds");
    assert_eq!(
        housing_key, "SeenHousingMarketDecorIDs",
        "RegisteredSavedSets.SeenHousingMarketDecorIDs must equal the literal string \
         'SeenHousingMarketDecorIDs' — the housing-market new-decor-glow set, the \
         11.x-cycle addition that motivated extracting this addon out of \
         Blizzard_StoreUI"
    );

    let count: f64 = env
        .eval(
            "local n = 0; for _ in pairs(SavedSetsUtil.RegisteredSavedSets) do n = n + 1 end; return n",
        )
        .expect("RegisteredSavedSets count probe succeeds");
    assert_eq!(
        count, 2.0,
        "RegisteredSavedSets must hold exactly 2 entries — adding a third without \
         matching consumer code in StoreUI/HousingFrame would silently break the \
         new-item-glow flow"
    );
}
}

prefork_full_ui_case! {
fn short_form_helpers_publish_globally_and_target_shop_catalog(env: &WowLuaEnv) {

    for helper in SHORT_FORM_HELPERS {
        let kind: String = env
            .eval(&format!("return type(_G.{helper})"))
            .unwrap_or_else(|err| panic!("type(_G.{helper}) probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "_G.{helper} must be a function — the 4 short-form helpers \
             (SavedSet_IsLoaded / SavedSet_HasAny / SavedSet_Set / SavedSet_Check) \
             are positional shorthands hardcoded to the SeenShopCatalogProductIDs key \
             at Blizzard_SavedSets.lua:96-106. They predate the 2-key \
             RegisteredSavedSets registry and stay published as globals for \
             backwards-compatibility with Blizzard_StoreUI's existing call sites"
        );
    }
}
}

prefork_full_ui_case! {
fn global_blizzard_saved_sets_publishes_as_table_after_load(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G.GlobalBlizzardSavedSets)")
        .expect("GlobalBlizzardSavedSets probe succeeds");
    assert_eq!(
        kind, "table",
        "_G.GlobalBlizzardSavedSets must publish as a table — \
         Blizzard_SavedSets.lua:2 declares `GlobalBlizzardSavedSets = \
         GlobalBlizzardSavedSets or {{}};` so the global is at minimum an empty \
         table after load even when the WTF SavedVariables file is missing. The \
         `or` idiom preserves any persisted state the SV restoration restored before \
         the addon body executes"
    );
}
}

prefork_full_ui_case! {
fn is_loaded_flips_true_after_variables_loaded_event(env: &WowLuaEnv) {

    let is_loaded: bool = env
        .eval("return SavedSetsUtil.IsLoaded()")
        .expect("IsLoaded probe succeeds");
    assert!(
        is_loaded,
        "SavedSetsUtil.IsLoaded() must return true after `fire_startup_events_for_screen` \
         dispatched VARIABLES_LOADED — the file-local `loaded` flag flips inside \
         TriggerPendingCallbacks (registered via \
         `EventUtil.ContinueOnVariablesLoaded(TriggerPendingCallbacks)` at module \
         end). Before VARIABLES_LOADED fires, IsLoaded would return false and \
         HasAny/Check would optimistically return true to keep new-item glows visible \
         until persistence catches up"
    );
}
}

prefork_full_ui_case! {
fn module_locals_stay_off_global_scope(env: &WowLuaEnv) {

    for name in MODULE_LOCAL_NAMES {
        let kind: String = env
            .eval(&format!("return type(_G.{name})"))
            .unwrap_or_else(|err| panic!("type(_G.{name}) probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "_G.{name} must be nil — the 3 module-locals (the `loaded` boolean flag, \
             the `pendingCallbacks` deferred-call queue, the local function \
             `TriggerPendingCallbacks` that flips the flag and drains the queue) are \
             intentionally file-local: leaking `loaded` would let consumers \
             clear-and-retry the load gate; leaking `pendingCallbacks` would let \
             consumers steal each other's callbacks; leaking TriggerPendingCallbacks \
             would let consumers spuriously fire the load completion before \
             VARIABLES_LOADED actually arrives"
        );
    }
}
}

prefork_full_ui_case! {
fn check_optimistic_pre_load_semantics_match_blizzard_contract(env: &WowLuaEnv) {

    let unseen_id: bool = env
        .eval("return SavedSetsUtil.HasAny(SavedSetsUtil.RegisteredSavedSets.SeenShopCatalogProductIDs)")
        .expect("HasAny probe succeeds");
    assert!(
        !unseen_id,
        "After load + VARIABLES_LOADED with no persisted entries, \
         SavedSetsUtil.HasAny(SeenShopCatalogProductIDs) must return false — once \
         `loaded == true`, HasAny consults the saved-set table and returns \
         `not TableIsEmpty(savedSet)`. The optimistic pre-load `return true` arm \
         only covers the brief window between addon load and VARIABLES_LOADED"
    );

    let empty_table_check: bool = env
        .eval("return SavedSetsUtil.Check(SavedSetsUtil.RegisteredSavedSets.SeenShopCatalogProductIDs, {})")
        .expect("Check empty-table probe succeeds");
    assert!(
        empty_table_check,
        "SavedSetsUtil.Check(_, {{}}) must return true — the table-form Check loop \
         iterates zero times when the input is empty and falls through to \
         `return true`. This is intentional: 'check that every id in this table \
         was seen' is vacuously true for an empty table"
    );
}
}

prefork_full_ui_case! {
fn set_then_check_round_trip_persists_within_session(env: &WowLuaEnv) {

    env.exec(
        "SavedSetsUtil.Set(SavedSetsUtil.RegisteredSavedSets.SeenShopCatalogProductIDs, 12345)",
    )
    .expect("Set probe succeeds");

    let single_check: bool = env
        .eval("return SavedSetsUtil.Check(SavedSetsUtil.RegisteredSavedSets.SeenShopCatalogProductIDs, 12345) and true or false")
        .expect("Check single id probe succeeds");
    assert!(
        single_check,
        "SavedSetsUtil.Check(key, 12345) must return true after Set(key, 12345) — \
         single-id Set writes `savedSet[id] = true` and single-id Check reads back \
         `savedSet[id]` directly"
    );

    env.exec(
        "SavedSetsUtil.Set(SavedSetsUtil.RegisteredSavedSets.SeenShopCatalogProductIDs, {100, 200, 300})",
    )
    .expect("Set table-form probe succeeds");

    let table_check: bool = env
        .eval("return SavedSetsUtil.Check(SavedSetsUtil.RegisteredSavedSets.SeenShopCatalogProductIDs, {100, 200, 300})")
        .expect("Check table-form probe succeeds");
    assert!(
        table_check,
        "SavedSetsUtil.Check(key, {{100, 200, 300}}) must return true after the \
         matching Set call — table-form Set iterates via ipairs and writes each id; \
         table-form Check iterates via ipairs and short-circuits on the first \
         missing id. Both overloads share the same `idOrTable` parameter dispatched \
         on `type(idOrTable) == 'table'`"
    );

    let partial_check: bool = env
        .eval("return SavedSetsUtil.Check(SavedSetsUtil.RegisteredSavedSets.SeenShopCatalogProductIDs, {100, 999})")
        .expect("partial Check probe succeeds");
    assert!(
        !partial_check,
        "SavedSetsUtil.Check(key, {{100, 999}}) must return false because 999 was \
         never written — table-form Check returns false on the first missing id"
    );
}
}
