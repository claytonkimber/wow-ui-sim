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

fn encounter_journal_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_EncounterJournal/Blizzard_EncounterJournal_Mainline.toc")
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
fn blizzard_encounter_journal_toc_declares_lod_game_only_with_three_deps() {
    let toc = TocFile::from_file(&encounter_journal_toc())
        .expect("Blizzard_EncounterJournal_Mainline TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_EncounterJournal declares `## LoadOnDemand: 1` — the dungeon journal \
         only loads when the player presses the toggle key (or a panel calls \
         LoadAddOn('Blizzard_EncounterJournal')). Auto-loading the 3.6k-line \
         Blizzard_EncounterJournal.lua + 4 sibling Lua/XML pairs at game start would \
         waste startup time"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_EncounterJournal does not declare `## UseSecureEnvironment` — runs in \
         the standard Lua environment (no Blizzard_*Secure namespace)"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_EncounterJournal declares `## AllowLoadGameType: mainline` — \
         is_game_type_restricted() returns false because mainline/standard match the \
         retail game type per src/toc.rs:298-299"
    );

    let deps: Vec<String> = toc.dependencies();
    assert_eq!(
        deps,
        vec![
            "Blizzard_ClassMenu".to_string(),
            "Blizzard_Colors".to_string(),
            "Blizzard_HelpPlate".to_string(),
        ],
        "Blizzard_EncounterJournal must declare exactly three `## Dependencies:` in \
         order: Blizzard_ClassMenu (class-spec icons + role tags read by encounter \
         flag-icon helpers), Blizzard_Colors (RAID_CLASS_COLORS palette referenced by \
         the loot-journal class restriction badges), Blizzard_HelpPlate (tutorial \
         overlay used by `MonthlyActivities_HelpPlate`). Got: {deps:?}"
    );
}

#[test]
fn blizzard_encounter_journal_does_not_appear_in_auto_discovery() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_EncounterJournal");
    assert!(
        !in_game,
        "Blizzard_EncounterJournal must NOT auto-load — it is `## LoadOnDemand: 1` \
         with no non-LOD dependents, so it stays in `lod_pool` and never enters the \
         game-screen discovery list. The retail UI explicitly calls \
         LoadAddOn('Blizzard_EncounterJournal') only when the player opens the journal"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_EncounterJournal");
    assert!(
        !in_login,
        "Blizzard_EncounterJournal must NOT appear on Login / glue screens — TOC \
         declares `## AllowLoad: game` (src/toc.rs:308 restricts to ScreenKind::Game)"
    );
}

#[test]
fn blizzard_encounter_journal_two_toc_variants_ship_for_mainline_and_mists() {
    let dir = blizzard_ui_dir().join("Blizzard_EncounterJournal");
    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("Blizzard_EncounterJournal dir should read")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let has_mainline_toc = entries
        .iter()
        .any(|n| n == "Blizzard_EncounterJournal_Mainline.toc");
    let has_mists_toc = entries
        .iter()
        .any(|n| n == "Blizzard_EncounterJournal_Mists.toc");
    assert!(
        has_mainline_toc && has_mists_toc,
        "Blizzard_EncounterJournal ships both Mainline and Mists TOC variants — the \
         retail loader picks the right one via the `[Game]` template substitution \
         (src/toc.rs:144-146). Got: {entries:?}"
    );
}

prefork_full_ui_case! {
fn blizzard_encounter_journal_loads_explicitly_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &encounter_journal_toc())
        .expect("Blizzard_EncounterJournal_Mainline should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    let journal_errors: Vec<String> = load_errors
        .iter()
        .filter(|message| {
            message.contains("EncounterJournal")
                || message.contains("Journeys")
                || message.contains("LootJournal")
                || message.contains("MonthlyActivities")
        })
        .cloned()
        .collect();
    assert!(
        journal_errors.is_empty(),
        "Blizzard_EncounterJournal emitted Lua errors during explicit load:\n  {}",
        journal_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_journal_creates_named_toplevel_singleton(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &encounter_journal_toc())
        .expect("Blizzard_EncounterJournal should load");

    let (kind, name): (String, String) = env
        .eval(
            "return type(EncounterJournal), \
                    (type(EncounterJournal) == 'table') \
                        and EncounterJournal:GetName() or ''",
        )
        .expect("EncounterJournal singleton probe should succeed");
    assert_eq!(
        kind, "table",
        "Blizzard_EncounterJournal.xml:1333 declares \
         `<Frame name=\"EncounterJournal\" inherits=\"PortraitFrameTemplate\" \
         toplevel=\"true\" parent=\"UIParent\" hidden=\"true\">` — must register as a \
         global table after load"
    );
    assert_eq!(
        name, "EncounterJournal",
        "EncounterJournal:GetName() must round-trip the XML name attribute"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_journal_starts_hidden(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &encounter_journal_toc())
        .expect("Blizzard_EncounterJournal should load");

    let starts_hidden: bool = env
        .eval("return EncounterJournal:IsShown() == false")
        .expect("EncounterJournal visibility probe should succeed");
    assert!(
        starts_hidden,
        "EncounterJournal must start HIDDEN — Blizzard_EncounterJournal.xml:1333 \
         declares `hidden=\"true\"`. The LOD addon is loaded eagerly when the user \
         opens the journal, but the frame remains hidden until ToggleEncounterJournal \
         flips visibility"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_journal_tooltip_singleton_exists_on_tooltip_strata(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &encounter_journal_toc())
        .expect("Blizzard_EncounterJournal should load");

    let (kind, strata, hidden): (String, String, bool) = env
        .eval(
            "return type(EncounterJournalTooltip), \
                    (type(EncounterJournalTooltip) == 'table') \
                        and EncounterJournalTooltip:GetFrameStrata() or '', \
                    (type(EncounterJournalTooltip) == 'table') \
                        and EncounterJournalTooltip:IsShown() == false",
        )
        .expect("EncounterJournalTooltip probe should succeed");
    assert_eq!(
        kind, "table",
        "Blizzard_EncounterJournal.xml:2360 declares \
         `<Frame name=\"EncounterJournalTooltip\" parent=\"UIParent\" \
         frameStrata=\"TOOLTIP\" clampedToScreen=\"true\" hidden=\"true\" \
         inherits=\"TooltipBackdropTemplate\">` — must register as a global"
    );
    assert_eq!(
        strata, "TOOLTIP",
        "EncounterJournalTooltip frameStrata must be TOOLTIP — the encounter-detail \
         hover tooltip floats above the journal panel itself (DIALOG strata)"
    );
    assert!(
        hidden,
        "EncounterJournalTooltip must start hidden — only shown on hover"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_journal_publishes_per_panel_mixin_globals(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &encounter_journal_toc())
        .expect("Blizzard_EncounterJournal should load");

    let all_present: bool = env
        .eval(
            "return type(LootJournalMixin) == 'table' \
                and type(LootJournalItemsMixin) == 'table' \
                and type(LootJournalItemSetsMixin) == 'table' \
                and type(JourneysFrameMixin) == 'table' \
                and type(MonthlyActivitiesFrameMixin) == 'table' \
                and type(EncounterJournalItemMixin) == 'table' \
                and type(EncounterBossButtonMixin) == 'table' \
                and type(GreatVaultButtonMixin) == 'table' \
                and type(ModifiedInstanceIconMixin) == 'table'",
        )
        .expect("Per-panel mixin globals probe should succeed");
    assert!(
        all_present,
        "Blizzard_EncounterJournal publishes one mixin per major panel: \
         LootJournalMixin (Blizzard_LootJournal.lua:69), LootJournalItemsMixin / \
         LootJournalItemSetsMixin (Blizzard_LootJournalItems.lua:1, 67), \
         JourneysFrameMixin (Blizzard_Journeys.lua:47), MonthlyActivitiesFrameMixin \
         (Blizzard_MonthlyActivities.lua:755), EncounterJournalItemMixin \
         (Blizzard_EncounterJournal.lua:182), EncounterBossButtonMixin (line 258), \
         GreatVaultButtonMixin (line 3624), ModifiedInstanceIconMixin (line 3582) — \
         each XML `<Frame mixin=\"...\">` references one of these"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_journal_publishes_aj_max_num_suggestions(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &encounter_journal_toc())
        .expect("Blizzard_EncounterJournal should load");

    let value: i64 = env
        .eval("return AJ_MAX_NUM_SUGGESTIONS")
        .expect("AJ_MAX_NUM_SUGGESTIONS probe should succeed");
    assert_eq!(
        value, 3,
        "Blizzard_EncounterJournal.lua:23 declares `AJ_MAX_NUM_SUGGESTIONS = 3` — the \
         Adventure Journal suggestions row builds exactly three slots, so the constant \
         must equal 3 after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_journal_publishes_flag_icon_atlas_table(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &encounter_journal_toc())
        .expect("Blizzard_EncounterJournal should load");

    let kind: String = env
        .eval("return type(EncounterJournalFlagIconAtlases)")
        .expect("EncounterJournalFlagIconAtlases probe should succeed");
    assert_eq!(
        kind, "table",
        "Blizzard_EncounterJournal.lua:2358 declares the global \
         `EncounterJournalFlagIconAtlases = {{ ... }}` — the lookup table maps \
         encounter-flag enum values to atlas keys for the boss-card flag icons. Must \
         exist as a global table after load"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_journal_publishes_expansion_to_ej_tier_data_table(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &encounter_journal_toc())
        .expect("Blizzard_EncounterJournal should load");

    let kind: String = env
        .eval("return type(ExpansionEnumToEJTierDataTableId)")
        .expect("ExpansionEnumToEJTierDataTableId probe should succeed");
    assert_eq!(
        kind, "table",
        "Blizzard_EncounterJournal.lua:124 declares the global \
         `ExpansionEnumToEJTierDataTableId = {{ ... }}` mapping each `LE_EXPANSION_*` \
         enum to its EJ_TIER_DATA index. Used by GetEJTierDataForExpansion (line 117) \
         to look up the right tier descriptor for the player's current expansion"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_journal_publishes_monthly_activities_help_plate(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &encounter_journal_toc())
        .expect("Blizzard_EncounterJournal should load");

    let kind: String = env
        .eval("return type(MonthlyActivities_HelpPlate)")
        .expect("MonthlyActivities_HelpPlate probe should succeed");
    assert_eq!(
        kind, "table",
        "Blizzard_MonthlyActivities.lua:23 declares the global \
         `MonthlyActivities_HelpPlate = {{ ... }}` — the HelpPlate descriptor consumed \
         by the `Blizzard_HelpPlate` dependency to render the tutorial overlay over \
         the Monthly Activities panel"
    );
}
}

prefork_full_ui_case! {
fn blizzard_encounter_journal_is_addon_loaded_returns_true_after_explicit_load(env: &WowLuaEnv) {

    let loaded_before: bool = env
        .eval(
            "local ok = (C_AddOns and C_AddOns.IsAddOnLoaded \
                or IsAddOnLoaded)('Blizzard_EncounterJournal'); \
                return ok == true or ok == 1",
        )
        .expect("IsAddOnLoaded pre-load probe should succeed");
    assert!(
        !loaded_before,
        "Blizzard_EncounterJournal must NOT report loaded before explicit load — it \
         is `## LoadOnDemand: 1` with no non-LOD dependents, so the standard \
         game-screen discovery skips it. If this returns true, the loader is \
         eagerly pulling LOD addons against the TOC declaration"
    );

    load_addon(&env.loader_env(), &encounter_journal_toc())
        .expect("Blizzard_EncounterJournal should load via Rust loader");

    let loaded_after: bool = env
        .eval(
            "local ok = (C_AddOns and C_AddOns.IsAddOnLoaded \
                or IsAddOnLoaded)('Blizzard_EncounterJournal'); \
                return ok == true or ok == 1",
        )
        .expect("IsAddOnLoaded post-load probe should succeed");
    assert!(
        loaded_after,
        "After explicit LoadAddOn-style call, IsAddOnLoaded('Blizzard_EncounterJournal') \
         must report true — the loader must register the addon name in the loaded set"
    );
}
}
