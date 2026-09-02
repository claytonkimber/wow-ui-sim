#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::paths::default_blizzard_ui_addons_path;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn expansion_trial_toc() -> PathBuf {
    let addon_dir = blizzard_ui_dir().join("Blizzard_ExpansionTrial");
    find_toc_file(&addon_dir).expect("Blizzard_ExpansionTrial TOC should resolve")
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
fn blizzard_expansion_trial_toc_is_load_on_demand_with_no_deps_or_saved_vars() {
    let toc = TocFile::from_file(&expansion_trial_toc())
        .expect("Blizzard_ExpansionTrial TOC should parse");

    assert!(
        toc.is_load_on_demand(),
        "Blizzard_ExpansionTrial declares `## LoadOnDemand: 1` — the trial-level-up dialog \
         is brought in on-demand by trial-account state changes (PLAYER_TRIAL_XP_UPDATE / \
         UPDATE_EXPANSION_LEVEL handlers in SetupCheckpoints) and must NOT auto-load on \
         standard Game-screen bring-up"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_ExpansionTrial does not declare `## UseSecureEnvironment` — the dialog \
         interacts with the cash shop (SetStoreUIShown / CatalogShopInboundInterface) but \
         the dialog itself runs in the standard taint environment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_ExpansionTrial has no `## Dependencies` line — it is self-contained and \
         consumes the `BaseExpandableDialog` template + `BaseExpandableDialogMixin` from \
         vendor `Blizzard_SharedXML/SharedBasicControls.lua` (always loaded), plus globals \
         like `EventRegistry`, `Mixin`, `GetExpansionDisplayInfo`, and \
         `GetClampedCurrentExpansionLevel`"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "Blizzard_ExpansionTrial declares `## AllowLoadGameType: mainline`, which matches \
         the retail profile, so `is_game_type_restricted()` returns false and the addon is \
         reachable from standard-retail discovery (just gated behind `LoadOnDemand`)"
    );

    assert!(
        toc.saved_variables().is_empty(),
        "Blizzard_ExpansionTrial declares no `## SavedVariables` — the dialog is purely \
         transient state (queuedDialog / dialogType / baseLevel are session-only fields on \
         the frame instance)"
    );

    let toc_text = std::fs::read_to_string(expansion_trial_toc())
        .expect("Blizzard_ExpansionTrial TOC should read");
    assert!(
        toc_text.contains("## AllowLoadGameType: mainline"),
        "Retail Blizzard_ExpansionTrial_Mainline.toc declares the mainline game type"
    );
    assert!(
        !toc_text.contains("## AllowLoad:"),
        "Blizzard_ExpansionTrial declares no `## AllowLoad:` line — defaults to Game-only \
         per `allows_screen` (src/toc.rs:311). The dialog is only meaningful when the \
         player is logged in, so it does NOT need to be available on glue screens"
    );
}

#[test]
fn blizzard_expansion_trial_allows_only_game_screen_by_default() {
    let toc = TocFile::from_file(&expansion_trial_toc())
        .expect("Blizzard_ExpansionTrial TOC should parse");

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "Missing `## AllowLoad` defaults to Game-only (src/toc.rs:311) — the trial dialog \
         needs the in-game environment (UnitLevel, UnitAffectingCombat, GetExpansionTrialInfo)"
    );
    assert!(
        !toc.allows_screen(ScreenKind::Login),
        "Missing `## AllowLoad` excludes Login (src/toc.rs:311) — there is no concept of a \
         trial-level-up dialog on the login screen"
    );
    assert!(
        !toc.allows_screen(ScreenKind::CharacterSelect),
        "Missing `## AllowLoad` excludes CharacterSelect — same reason: no in-world player \
         state to drive the dialog"
    );
}

#[test]
fn blizzard_expansion_trial_is_absent_from_auto_discovery_on_game_and_login() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_ExpansionTrial");
    assert!(
        !in_game,
        "Blizzard_ExpansionTrial is `## LoadOnDemand: 1`, so it must NOT appear in \
         Game-screen auto-discovery — it is loaded explicitly when expansion-trial state \
         changes occur"
    );

    let login_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_ExpansionTrial");
    assert!(
        !in_login,
        "Blizzard_ExpansionTrial is Game-only AND `## LoadOnDemand: 1`, so it must NOT \
         appear in Login-screen auto-discovery — both the screen filter and the LOD gate \
         keep it out"
    );
}

prefork_full_ui_case! {
fn blizzard_expansion_trial_loads_via_load_addon_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &expansion_trial_toc())
        .expect("Blizzard_ExpansionTrial should load via Rust loader");

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("ExpansionTrial")
                || message.contains("Blizzard_ExpansionTrial")
                || message.contains("ExpansionTrialCheckPointDialog")
        })
        .cloned()
        .collect();

    assert!(
        load_errors.is_empty(),
        "Blizzard_ExpansionTrial emitted Lua errors during explicit load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_expansion_trial_is_addon_loaded_returns_true_after_explicit_load(env: &WowLuaEnv) {

    let pre_load: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_ExpansionTrial') and true or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        !pre_load,
        "Before the explicit load, IsAddOnLoaded('Blizzard_ExpansionTrial') must return \
         false — confirms Game-screen auto-discovery did not load this LoadOnDemand addon"
    );

    load_addon(&env.loader_env(), &expansion_trial_toc())
        .expect("Blizzard_ExpansionTrial should load via Rust loader");

    let post_load: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_ExpansionTrial') and true or false")
        .expect("IsAddOnLoaded probe should succeed");
    assert!(
        post_load,
        "After explicit `load_addon`, IsAddOnLoaded('Blizzard_ExpansionTrial') must return \
         true — `mark_addon_loaded` (src/loader/addon.rs:131) registers the folder name in \
         the loaded-set"
    );
}
}

prefork_full_ui_case! {
fn blizzard_expansion_trial_check_point_dialog_singleton_publishes_with_correct_parent(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &expansion_trial_toc())
        .expect("Blizzard_ExpansionTrial should load via Rust loader");

    let probe: (String, String) = env
        .eval(
            "return ExpansionTrialCheckPointDialog:GetName(), \
                    ExpansionTrialCheckPointDialog:GetParent():GetName()",
        )
        .expect("ExpansionTrialCheckPointDialog name+parent probe should succeed");
    assert_eq!(
        probe,
        (
            "ExpansionTrialCheckPointDialog".to_string(),
            "UIParent".to_string(),
        ),
        "The non-virtual `<Frame name=\"ExpansionTrialCheckPointDialog\" toplevel=\"true\" \
         frameStrata=\"DIALOG\" parent=\"UIParent\" inherits=\"BaseExpandableDialog, \
         VerticalLayoutFrame\" mixin=\"ExpansionTrialCheckPointDialogMixin\">` (xml:34) must \
         publish as a global table whose `:GetName()` is 'ExpansionTrialCheckPointDialog' \
         and `:GetParent()` is UIParent"
    );
}
}

prefork_full_ui_case! {
fn blizzard_expansion_trial_mixin_publishes_four_dialog_type_enum_constants(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &expansion_trial_toc())
        .expect("Blizzard_ExpansionTrial should load via Rust loader");

    let dialog_type_constants: (f64, f64, f64, f64) = env
        .eval(
            "return ExpansionTrialCheckPointDialogMixin.ReachedLevelLimit, \
                    ExpansionTrialCheckPointDialogMixin.FinishedCampaign, \
                    ExpansionTrialCheckPointDialogMixin.GainedBankedLevel, \
                    ExpansionTrialCheckPointDialogMixin.TrialUpgrade",
        )
        .expect("dialog-type enum probe should succeed");
    assert_eq!(
        dialog_type_constants,
        (1.0, 2.0, 3.0, 4.0),
        "`ExpansionTrialCheckPointDialogMixin = Mixin({{ReachedLevelLimit=1, \
         FinishedCampaign=2, GainedBankedLevel=3, TrialUpgrade=4}}, \
         BaseExpandableDialogMixin)` (lua:1-7) must publish the four dialog-type integer \
         constants — `ShowDialogType(dialogType)` (lua:76) keys into a `dialogData` table \
         using these IDs to dispatch the four UI configurators"
    );
}
}

prefork_full_ui_case! {
fn blizzard_expansion_trial_mixin_inherits_base_expandable_dialog_methods(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &expansion_trial_toc())
        .expect("Blizzard_ExpansionTrial should load via Rust loader");

    let inherited_methods: (bool, bool) = env
        .eval(
            "return type(ExpansionTrialCheckPointDialogMixin.SetupTextureKit) == 'function', \
                    type(ExpansionTrialCheckPointDialogMixin.OnCloseClick) == 'function'",
        )
        .expect("BaseExpandableDialogMixin inheritance probe should succeed");
    assert_eq!(
        inherited_methods,
        (true, true),
        "`Mixin({{...}}, BaseExpandableDialogMixin)` must pull in `SetupTextureKit` (used \
         in OnLoad at lua:21 to apply `textureKitRegionInfo`) and `OnCloseClick` (the close \
         handler — `ExpansionTrialCheckPointDialogMixin:OnCloseClick` at lua:121 explicitly \
         delegates back to `BaseExpandableDialogMixin.OnCloseClick(self)` after the \
         force-logout check). The base mixin lives at \
         `Blizzard_SharedXML/SharedBasicControls.lua:69` and provides both methods"
    );
}
}

prefork_full_ui_case! {
fn blizzard_expansion_trial_dialog_publishes_six_parent_key_children(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &expansion_trial_toc())
        .expect("Blizzard_ExpansionTrial should load via Rust loader");

    let children_present: (bool, bool, bool, bool, bool, bool) = env
        .eval(
            "local d = ExpansionTrialCheckPointDialog \
             return type(d.Title) == 'table', \
                    type(d.GainedLevelContainer) == 'table', \
                    type(d.ExpansionImage) == 'table', \
                    type(d.Description) == 'table', \
                    type(d.Button) == 'table', \
                    type(d.EatAllInput) == 'table'",
        )
        .expect("parent-key child probe should succeed");
    assert_eq!(
        children_present,
        (true, true, true, true, true, true),
        "The dialog XML (xml:34-133) declares six parent-key children that must publish: \
         Title (FontString layoutIndex=1, GameFontHighlightLarge, xml:53), \
         GainedLevelContainer (Frame layoutIndex=2, inherits virtual template \
         ExpansionTrialCheckPointLevelHeaderTemplate, xml:99), ExpansionImage (Texture \
         layoutIndex=3, 256x128, xml:64), Description (FontString layoutIndex=4, \
         GameFontNormalMed2, xml:75), Button (Button layoutIndex=5, UIPanelButtonTemplate, \
         xml:109), and EatAllInput (Frame frameStrata=LOW, full-UIParent overlay that \
         absorbs mouse+keyboard during the modal, xml:90)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_expansion_trial_gained_level_container_inherits_virtual_template_layers(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &expansion_trial_toc())
        .expect("Blizzard_ExpansionTrial should load via Rust loader");

    let template_layers_present: (bool, bool, bool, bool) = env
        .eval(
            "local glc = ExpansionTrialCheckPointDialog.GainedLevelContainer \
             return type(glc.TopLine) == 'table', \
                    type(glc.Header) == 'table', \
                    type(glc.Text) == 'table', \
                    type(glc.BottomLine) == 'table'",
        )
        .expect("virtual-template layer probe should succeed");
    assert_eq!(
        template_layers_present,
        (true, true, true, true),
        "GainedLevelContainer (xml:99) inherits the virtual template \
         `ExpansionTrialCheckPointLevelHeaderTemplate` (xml:5-32, virtual=true, inherits \
         ResizeLayoutFrame) which contributes four ARTWORK-layer regions: TopLine \
         (atlas=levelup-bar-gold), Header (FontString GameFontHighlightHuge, \
         text=EXPANSION_TRIAL_GAINED_LEVEL_HEADER), Text (FontString GameFont_Gigantic — \
         filled by `EXPANSION_TRIAL_GAINED_LEVEL_TEXT:format(self:GetCurrentPlayerLevel())` \
         at lua:60), and BottomLine (atlas=levelup-bar-gold). All four must publish via the \
         template-inheritance pipeline"
    );
}
}

prefork_full_ui_case! {
fn blizzard_expansion_trial_dialog_starts_hidden_after_load(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &expansion_trial_toc())
        .expect("Blizzard_ExpansionTrial should load via Rust loader");

    let is_shown: bool = env
        .eval("return ExpansionTrialCheckPointDialog:IsShown()")
        .expect("ExpansionTrialCheckPointDialog:IsShown() probe should succeed");
    assert!(
        !is_shown,
        "`ExpansionTrialCheckPointDialog` must start hidden after load — the dialog is \
         only summoned via `:ShowDialogType(dialogType)` (lua:76) in response to expansion \
         trial events (PLAYER_LEVEL_CHANGED hitting the cap, PLAYER_TRIAL_XP_UPDATE gaining \
         banked levels, QUEST_TURNED_IN with questID 65794, or UPDATE_EXPANSION_LEVEL with \
         `upgradingFromExpansionTrial`). Auto-showing on load would block the entire UI \
         behind the EatAllInput modal"
    );
}
}
