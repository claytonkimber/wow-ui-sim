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

fn console_toc() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_Console/Blizzard_Console.toc")
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
fn blizzard_console_toc_declares_command_line_util_dep_and_machine_savedvars() {
    let toc = TocFile::from_file(&console_toc()).expect("Blizzard_Console TOC should parse");
    assert!(
        !toc.is_load_on_demand(),
        "Blizzard_Console is a non-LOD addon (it should auto-load on Game and Login screens)"
    );
    let deps = toc.dependencies();
    assert!(
        deps.contains(&"Blizzard_CommandLineUtil".to_string()),
        "Blizzard_Console should declare `## RequiredDep: Blizzard_CommandLineUtil` (so its \
         CommandLineUtil-driven autocomplete scoring resolves), got {deps:?}"
    );
    let saved = toc.saved_variables();
    assert!(
        saved.contains(&"Blizzard_Console_SavedVars".to_string()),
        "Blizzard_Console declares `## SavedVariablesMachine: Blizzard_Console_SavedVars` (so \
         the per-machine console history persists across reloads), got {saved:?}"
    );
}

#[test]
fn blizzard_console_appears_in_both_screens_discovery() {
    let ui = blizzard_ui_dir();

    let game_addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_Console");
    assert!(
        in_game,
        "Blizzard_Console (`## AllowLoad: Both`, non-LOD) should appear in Game-screen discovery"
    );

    let login_addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Login);
    let in_login = login_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_Console");
    assert!(
        in_login,
        "Blizzard_Console (`## AllowLoad: Both`) should also appear in Login-screen discovery so \
         the developer console is reachable before entering world"
    );
}

prefork_full_ui_case! {
fn blizzard_console_loads_without_errors(env: &WowLuaEnv) {

    let console_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_Console") || message.contains("DeveloperConsole")
        })
        .cloned()
        .collect();
    assert!(
        console_errors.is_empty(),
        "Blizzard_Console emitted Lua errors during load:\n  {}",
        console_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_console_developer_console_frame_is_defined(env: &WowLuaEnv) {

    let frame_present: bool = env
        .eval(
            "return type(_G.DeveloperConsole) == 'table' \
                and DeveloperConsole:GetFrameStrata() == 'FULLSCREEN_DIALOG'",
        )
        .expect("DeveloperConsole query should succeed");
    assert!(
        frame_present,
        "Blizzard_Console should define the top-level `DeveloperConsole` frame \
         (frameStrata=FULLSCREEN_DIALOG, frameLevel=9000, hidden=true, mixin=DeveloperConsoleMixin) \
         after Blizzard_Console.xml loads"
    );
}
}

prefork_full_ui_case! {
fn blizzard_console_developer_console_mixin_methods_are_defined(env: &WowLuaEnv) {

    let methods_present: bool = env
        .eval(
            "return type(DeveloperConsoleMixin) == 'table' \
                and type(DeveloperConsoleMixin.OnLoad) == 'function' \
                and type(DeveloperConsoleMixin.OnEvent) == 'function' \
                and type(DeveloperConsoleMixin.AddMessage) == 'function' \
                and type(DeveloperConsoleMixin.AddMessageInternal) == 'function' \
                and type(DeveloperConsoleMixin.Clear) == 'function' \
                and type(DeveloperConsoleMixin.SetFontHeight) == 'function' \
                and type(DeveloperConsoleMixin.RefreshMessageFrame) == 'function' \
                and type(DeveloperConsoleMixin.Toggle) == 'function' \
                and type(DeveloperConsoleMixin.OnEscapePressed) == 'function' \
                and type(DeveloperConsoleMixin.ExecuteCommand) == 'function' \
                and type(DeveloperConsoleMixin.CheckExecuteOverrideCommand) == 'function' \
                and type(DeveloperConsoleMixin.SetExecuteCommandOverrideFunction) == 'function' \
                and type(DeveloperConsoleMixin.AddToCommandHistory) == 'function' \
                and type(DeveloperConsoleMixin.InsertLinkedCommand) == 'function' \
                and type(DeveloperConsoleMixin.OnEditBoxArrowPressed) == 'function' \
                and type(DeveloperConsoleMixin.OnEditBoxTabPressed) == 'function' \
                and type(DeveloperConsoleMixin.OnFilterEditBoxTextChanged) == 'function' \
                and type(DeveloperConsoleMixin.SetAutoCompleteText) == 'function' \
                and type(DeveloperConsoleMixin.FindBestEditCommand) == 'function' \
                and type(DeveloperConsoleMixin.ResetCommandHistoryIndex) == 'function' \
                and type(DeveloperConsoleMixin.GetCommandHistoryIndex) == 'function' \
                and type(DeveloperConsoleMixin.SetCommandHistoryIndex) == 'function' \
                and type(DeveloperConsoleMixin.HasSetCommandHistoryIndex) == 'function'",
        )
        .expect("DeveloperConsoleMixin method query should succeed");
    assert!(
        methods_present,
        "DeveloperConsoleMixin should expose its 23 documented methods covering lifecycle \
         (OnLoad/OnEvent), message handling (AddMessage/AddMessageInternal/Clear/RefreshMessageFrame), \
         appearance (SetFontHeight), visibility (Toggle/OnEscapePressed), command execution \
         (ExecuteCommand/CheckExecuteOverrideCommand/SetExecuteCommandOverrideFunction/\
         AddToCommandHistory/InsertLinkedCommand), edit-box input (OnEditBoxArrowPressed/\
         OnEditBoxTabPressed/OnFilterEditBoxTextChanged/SetAutoCompleteText/FindBestEditCommand), \
         and command-history-index helpers (Reset/Get/Set/HasSetCommandHistoryIndex)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_console_autocomplete_mixin_methods_are_defined(env: &WowLuaEnv) {

    let methods_present: bool = env
        .eval(
            "return type(DeveloperConsoleAutoCompleteMixin) == 'table' \
                and type(DeveloperConsoleAutoCompleteMixin.OnLoad) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.OnAvailableHeightChanged) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.SetText) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.ShouldTextBeVisible) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.ForceHide) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.GetSelectedText) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.NextEntry) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.PreviousEntry) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.GetNumEntries) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.GetEntryAtIndex) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.ClearEntryIndex) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.SetEntryIndex) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.GetEntryIndex) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.HasUserChosenEntryIndex) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.MarkDirty) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.OnUpdate) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.DisplayResults) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.OnEntryClicked) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.StartSearch) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.CancelSearch) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.FinishWork) == 'function' \
                and type(DeveloperConsoleAutoCompleteMixin.StepAutoCompleteSearchCoroutine) == 'function'",
        )
        .expect("DeveloperConsoleAutoCompleteMixin method query should succeed");
    assert!(
        methods_present,
        "DeveloperConsoleAutoCompleteMixin should expose its 22 documented methods covering \
         lifecycle (OnLoad/OnAvailableHeightChanged/OnUpdate), text driving (SetText / \
         ShouldTextBeVisible / ForceHide / GetSelectedText), entry navigation (NextEntry / \
         PreviousEntry / GetNumEntries / GetEntryAtIndex / Clear/Set/Get/HasUserChosenEntryIndex), \
         redraw (MarkDirty / DisplayResults / OnEntryClicked), and the coroutine search loop \
         (StartSearch / CancelSearch / FinishWork / StepAutoCompleteSearchCoroutine)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_console_global_helpers_are_defined(env: &WowLuaEnv) {

    let globals_present: bool = env
        .eval(
            "return type(_G.BlizzardConsoleMessageFrame_OnHyperlinkClick) == 'function' \
                and type(_G.DeveloperConsole_GetLastCommand) == 'function' \
                and type(_G.DeveloperConsole_RepeatLastCommand) == 'function'",
        )
        .expect("Console global helper query should succeed");
    assert!(
        globals_present,
        "Blizzard_Console.lua should define three module-global helpers: \
         BlizzardConsoleMessageFrame_OnHyperlinkClick (called from MessageFrame's hyperlink \
         OnHyperlinkClick — Shift inserts the command into the EditBox, plain click runs it via \
         ConsoleExec under forceinsecure), DeveloperConsole_GetLastCommand (returns the last \
         entry of Blizzard_Console_SavedVars.commandHistory or nil), and \
         DeveloperConsole_RepeatLastCommand (re-runs the last command via ConsoleExec)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_console_get_last_command_reads_saved_var_history_tail(env: &WowLuaEnv) {

    let history_lookup_works: bool = env
        .eval(
            "Blizzard_Console_SavedVars = Blizzard_Console_SavedVars or {}; \
             Blizzard_Console_SavedVars.commandHistory = { 'first', 'middle', 'last_cmd' }; \
             local last = DeveloperConsole_GetLastCommand(); \
             Blizzard_Console_SavedVars.commandHistory = {}; \
             local none = DeveloperConsole_GetLastCommand(); \
             Blizzard_Console_SavedVars.commandHistory = nil; \
             local nil_history = DeveloperConsole_GetLastCommand(); \
             return last == 'last_cmd' and none == nil and nil_history == nil",
        )
        .expect("DeveloperConsole_GetLastCommand query should succeed");
    assert!(
        history_lookup_works,
        "DeveloperConsole_GetLastCommand should return the tail of \
         Blizzard_Console_SavedVars.commandHistory ('last_cmd' for a 3-entry list), nil for an \
         empty list, and nil when the saved-var sub-table is missing entirely"
    );
}
}
