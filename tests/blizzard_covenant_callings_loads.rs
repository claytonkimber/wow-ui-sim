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

fn covenant_callings_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_CovenantCallings/Blizzard_CovenantCallings.toc")
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
fn blizzard_covenant_callings_toc_is_load_on_demand() {
    let toc = TocFile::from_file(&covenant_callings_toc())
        .expect("Blizzard_CovenantCallings TOC should parse");
    assert!(
        toc.is_load_on_demand(),
        "Blizzard_CovenantCallings declares `## LoadOnDemand: 1` (the Shadowlands covenant \
         callings widget is only created on demand by the Garrison Landing Page when the \
         player opens their covenant sanctum, so it must NOT auto-load on Game-screen bring-up)"
    );
    assert!(
        !toc.is_secure_env(),
        "Blizzard_CovenantCallings does not declare UseSecureEnvironment"
    );
    assert!(
        toc.dependencies().is_empty(),
        "Blizzard_CovenantCallings has no `## Dependencies` line — its TOC is only \
         `## LoadOnDemand: 1` plus `CovenantCallings.xml`"
    );
}

#[test]
fn blizzard_covenant_callings_is_absent_from_game_auto_discovery() {
    let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let in_game = addons
        .iter()
        .any(|(name, _)| name == "Blizzard_CovenantCallings");
    assert!(
        !in_game,
        "Blizzard_CovenantCallings is `## LoadOnDemand: 1`, so it must NOT appear in \
         Game-screen auto-discovery — it is loaded explicitly by the Garrison Landing Page \
         via UIParentLoadAddOn when the covenant callings widget is created"
    );
}

prefork_full_ui_case! {
fn blizzard_covenant_callings_loads_via_load_addon_without_errors(env: &WowLuaEnv) {

    {
        let mut state = env.state().borrow_mut();
        state.lua_errors.clear();
        state.lua_error_records.clear();
        state.lua_error_counts.clear();
    }

    load_addon(&env.loader_env(), &covenant_callings_toc())
        .expect("Blizzard_CovenantCallings should load via Rust loader");

    let load_errors: Vec<String> = env.state().borrow().lua_errors.clone();
    assert!(
        load_errors.is_empty(),
        "Blizzard_CovenantCallings emitted Lua errors during explicit load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_callings_quest_mixin_methods_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_callings_toc())
        .expect("Blizzard_CovenantCallings should load via Rust loader");

    let methods_present: bool = env
        .eval(
            "return type(CovenantCallingQuestMixin) == 'table' \
                and type(CovenantCallingQuestMixin.Set) == 'function' \
                and type(CovenantCallingQuestMixin.Update) == 'function' \
                and type(CovenantCallingQuestMixin.UpdateIcon) == 'function' \
                and type(CovenantCallingQuestMixin.UpdateBang) == 'function' \
                and type(CovenantCallingQuestMixin.GetDaysUntilNext) == 'function' \
                and type(CovenantCallingQuestMixin.GetDaysUntilNextString) == 'function' \
                and type(CovenantCallingQuestMixin.UpdateTooltip) == 'function' \
                and type(CovenantCallingQuestMixin.UpdateTooltipCheckHasQuestData) == 'function' \
                and type(CovenantCallingQuestMixin.UpdateTooltipQuestOffer) == 'function' \
                and type(CovenantCallingQuestMixin.UpdateTooltipQuestActive) == 'function' \
                and type(CovenantCallingQuestMixin.OnEnter) == 'function' \
                and type(CovenantCallingQuestMixin.OnLeave) == 'function' \
                and type(CovenantCallingQuestMixin.OnMouseUp) == 'function'",
        )
        .expect("CovenantCallingQuestMixin query should succeed");
    assert!(
        methods_present,
        "CovenantCallingQuestMixin should expose its 13 methods (Set / Update / UpdateIcon / \
         UpdateBang / GetDaysUntilNext / GetDaysUntilNextString / UpdateTooltip / \
         UpdateTooltipCheckHasQuestData / UpdateTooltipQuestOffer / UpdateTooltipQuestActive / \
         OnEnter / OnLeave / OnMouseUp) for per-quest icon/glow/bang display + \
         `CovenantCallingQuestTemplate` script delegation"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_callings_mixin_methods_are_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_callings_toc())
        .expect("Blizzard_CovenantCallings should load via Rust loader");

    let methods_present: bool = env
        .eval(
            "return type(CovenantCallingsMixin) == 'table' \
                and type(CovenantCallingsMixin.OnLoad) == 'function' \
                and type(CovenantCallingsMixin.OnShow) == 'function' \
                and type(CovenantCallingsMixin.OnHide) == 'function' \
                and type(CovenantCallingsMixin.OnEvent) == 'function' \
                and type(CovenantCallingsMixin.CheckUpdateForQuestID) == 'function' \
                and type(CovenantCallingsMixin.OnQuestTurnedIn) == 'function' \
                and type(CovenantCallingsMixin.OnQuestAccepted) == 'function' \
                and type(CovenantCallingsMixin.Update) == 'function' \
                and type(CovenantCallingsMixin.UpdateBackground) == 'function' \
                and type(CovenantCallingsMixin.OnCovenantCallingsUpdated) == 'function' \
                and type(CovenantCallingsMixin.ProcessCallings) == 'function' \
                and type(CovenantCallingsMixin.GetHelptipTargetFrame) == 'function' \
                and type(CovenantCallingsMixin.GetDaysUntilNext) == 'function' \
                and type(CovenantCallingsMixin.CheckDisplayHelpTip) == 'function'",
        )
        .expect("CovenantCallingsMixin query should succeed");
    assert!(
        methods_present,
        "CovenantCallingsMixin should expose its 14 methods (OnLoad creating the \
         CovenantCallingQuestTemplate FramePool + grid layout; OnShow/OnHide registering \
         COVENANT_CALLINGS_UPDATED / QUEST_TURNED_IN / QUEST_ACCEPTED via FrameUtil; OnEvent \
         dispatch; CheckUpdateForQuestID + OnQuestTurnedIn + OnQuestAccepted re-fetching when \
         a calling quest changes; Update / UpdateBackground refreshing covenant data; \
         OnCovenantCallingsUpdated rebuilding pooled frames + AnchorUtil grid layout; \
         ProcessCallings sorting by lock/active/timeRemaining; GetHelptipTargetFrame / \
         GetDaysUntilNext / CheckDisplayHelpTip for the LE_FRAME_TUTORIAL_9_0 hint)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_callings_namespace_create_helper_is_defined(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_callings_toc())
        .expect("Blizzard_CovenantCallings should load via Rust loader");

    let helper_present: bool = env
        .eval(
            "return type(CovenantCallings) == 'table' \
                and type(CovenantCallings.Create) == 'function'",
        )
        .expect("CovenantCallings.Create query should succeed");
    assert!(
        helper_present,
        "Blizzard_CovenantCallings.lua line 331-334 should publish the global namespace \
         `CovenantCallings = {{}}` with a single helper `CovenantCallings.Create(parent)` \
         that returns `CreateFrame('Frame', nil, parent, 'CovenantCallingsTemplate')` — this \
         is what the Garrison Landing Page calls to spawn the embedded callings widget"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_callings_xml_templates_are_registered(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_callings_toc())
        .expect("Blizzard_CovenantCallings should load via Rust loader");

    for template_name in ["CovenantCallingQuestTemplate", "CovenantCallingsTemplate"] {
        assert!(
            wow_ui_sim::xml::get_template(template_name).is_some(),
            "{template_name} (`<Frame virtual=\"true\">` from CovenantCallings.xml) should be \
             registered in the template registry after Blizzard_CovenantCallings loads"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_covenant_callings_create_helper_returns_a_frame_using_the_template(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_callings_toc())
        .expect("Blizzard_CovenantCallings should load via Rust loader");

    let create_works: bool = env
        .eval(
            "local f = CovenantCallings.Create(UIParent); \
             return type(f) == 'table' \
                and f:GetParent() == UIParent \
                and type(f.OnLoad) == 'function' \
                and type(f.pool) == 'table' \
                and type(f.layout) == 'table'",
        )
        .expect("CovenantCallings.Create query should succeed");
    assert!(
        create_works,
        "CovenantCallings.Create(UIParent) should return a Frame parented to UIParent that \
         inherits the CovenantCallingsTemplate (mixin=CovenantCallingsMixin) — OnLoad must \
         have populated `self.pool` (CreateFramePool 'CovenantCallingQuestTemplate') and \
         `self.layout` (AnchorUtil.CreateGridLayout with Constants.Callings.MaxCallings=3)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_covenant_callings_calling_states_enum_matches_expected_values(env: &WowLuaEnv) {
    load_addon(&env.loader_env(), &covenant_callings_toc())
        .expect("Blizzard_CovenantCallings should load via Rust loader");

    let enum_ok: bool = env
        .eval(
            "return type(Enum.CallingStates) == 'table' \
                and Enum.CallingStates.QuestOffer == 0 \
                and Enum.CallingStates.QuestActive == 1 \
                and Enum.CallingStates.QuestCompleted == 2 \
                and type(Constants.Callings) == 'table' \
                and Constants.Callings.MaxCallings == 3",
        )
        .expect("Enum.CallingStates / Constants.Callings query should succeed");
    assert!(
        enum_ok,
        "Blizzard_CovenantCallings depends on Enum.CallingStates \
         (QuestOffer=0/QuestActive=1/QuestCompleted=2 — driving GetState() in \
         Blizzard_ObjectAPI/CovenantCalling.lua and the icon/bang/glow logic here) and \
         Constants.Callings.MaxCallings=3 (the OnLoad grid layout column count + \
         ProcessCallings iteration bound) — both must be populated by the simulator's \
         missing_enums.lua / constants_values.lua before this addon can drive its widgets"
    );
}
}
