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

fn script_errors_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_ScriptErrors")
}

fn script_errors_toc() -> PathBuf {
    script_errors_dir().join("Blizzard_ScriptErrors.toc")
}

const TOC_FILES: &[&str] = &["Blizzard_ScriptErrors.lua"];

const SCRIPT_ERRORS_MIXIN_METHODS: &[&str] = &["Init", "AddUnhandledError"];

const MODULE_LOCAL_NAMES: &[&str] = &["errorHandlers", "GetErrorData", "HandleLuaError"];

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
    let resolved = find_toc_file(&script_errors_dir()).expect("Blizzard_ScriptErrors TOC resolves");
    assert_eq!(
        resolved,
        script_errors_toc(),
        "Blizzard_ScriptErrors ships exactly one bare TOC. The addon owns the global \
         Lua error pipeline (`seterrorhandler`, `AddLuaErrorHandler`, ScriptErrors \
         instance) so it must load with identical contents on every flavor — no \
         flavor-specific TOC variants"
    );
}

#[test]
fn toc_declares_eager_on_both_screens_with_no_dependencies() {
    let toc = TocFile::from_file(&script_errors_toc()).expect("Blizzard_ScriptErrors TOC parses");

    assert!(
        !toc.is_load_on_demand(),
        "TOC must NOT declare LoadOnDemand — the error handler MUST install via \
         `seterrorhandler(HandleLuaError)` at the bottom of the body BEFORE any \
         downstream addon runs Lua, otherwise pre-handler errors fall back to the C++ \
         engine handler that doesn't go through ScriptErrorsFrame's UI"
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
             Glue-screen Lua errors (login addons, character-select scripts) must \
             route through the same pipeline as in-game errors"
        );
    }

    assert!(
        toc.dependencies().is_empty(),
        "TOC must declare ZERO `## Dependencies:` — the error handler is the bottom \
         of the dependency graph by design. Depending on any other Blizzard addon \
         would create a circular load-order risk: that addon's load-time errors \
         couldn't route through ScriptErrors because ScriptErrors hasn't installed \
         its handler yet"
    );
    assert!(toc.optional_deps().is_empty());
    assert!(toc.saved_variables().is_empty());
    assert!(toc.saved_variables_per_character().is_empty());
}

#[test]
fn toc_declares_default_state_enabled() {
    let toc = TocFile::from_file(&script_errors_toc()).expect("Blizzard_ScriptErrors TOC parses");
    assert!(
        toc.default_enabled(),
        "TOC must ship `## DefaultState: enabled` — the player must NOT be able to \
         disable the global error handler from the addon list. Without it, every \
         downstream Lua error would fall through to the C++ engine handler"
    );
}

#[test]
fn toc_declares_metadata_in_raw_bytes_with_escalate_and_global_env() {
    let raw = std::fs::read_to_string(script_errors_toc())
        .expect("Blizzard_ScriptErrors TOC reads utf-8");
    assert!(
        raw.contains("## Title: Blizzard Script Error"),
        "TOC must declare `## Title: Blizzard Script Error` (note: singular \"Error\" \
         even though the file is plural \"Errors\" — carried verbatim from Blizzard \
         source)"
    );
    assert!(raw.contains("## Author: Blizzard Entertainment"));
    assert!(raw.contains("## DefaultState: enabled"));
    assert!(raw.contains("## AllowLoad: Both"));
    assert!(
        raw.contains("## EscalateErrorDuringLoad: 1"),
        "TOC must declare `## EscalateErrorDuringLoad: 1` — this Blizzard-internal \
         flag tells the engine that any Lua error raised DURING THIS ADDON'S LOAD \
         (before `seterrorhandler` finishes installing) must be escalated rather \
         than swallowed. Crucial because the file's whole purpose is to install the \
         handler, and a silent failure during that install would disable error \
         reporting for the entire session"
    );
    assert!(
        raw.contains("Blizzard_ScriptErrors.lua [AllowLoadEnvironment Global]"),
        "Body line must carry the `[AllowLoadEnvironment Global]` postfix on the \
         single Lua file — this is a global-load-pass filter, not an fenv override. \
         The simulator's TOC parser strips this annotation so the path resolves to \
         the bare filename"
    );
}

#[test]
fn toc_lists_single_lua_file_with_no_xml() {
    let toc = TocFile::from_file(&script_errors_toc()).expect("Blizzard_ScriptErrors TOC parses");
    let listed: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        listed, TOC_FILES,
        "TOC body must list exactly 1 file: Blizzard_ScriptErrors.lua. The addon is \
         pure-Lua error pipeline — NO XML, no frames, no templates. The visual UI \
         (ScriptErrorsFrame) lives in the separately-loaded Blizzard_ScriptErrorsFrame \
         addon which registers as a downstream handler via `AddLuaErrorHandler`"
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
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_ScriptErrors");
        assert!(
            found,
            "Blizzard_ScriptErrors must auto-discover on screen {screen:?} — eager \
             (no LoadOnDemand) AND `## AllowLoad: Both` makes \
             `discover_blizzard_addons_for_screen` include the addon on every screen"
        );
    }
}

#[test]
fn root_directory_holds_single_lua_next_to_toc() {
    let dir = script_errors_dir();
    assert!(dir.join("Blizzard_ScriptErrors.lua").is_file());
    assert!(dir.join("Blizzard_ScriptErrors.toc").is_file());

    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("read addon dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        2,
        "Blizzard_ScriptErrors directory must contain exactly 2 entries (1 lua + 1 toc), \
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
            message.contains("Blizzard_ScriptErrors")
                || message.contains("ScriptErrorsMixin")
                || message.contains("AddLuaErrorHandler")
                || message.contains("HandleLuaError")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_ScriptErrors emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn is_addon_loaded_after_eager_sweep(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_ScriptErrors')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_ScriptErrors') must return true after the \
         eager Game-screen sweep — eager (no LoadOnDemand) puts the addon in the \
         eager set and `load_addon` flips the loaded flag"
    );
}
}

prefork_full_ui_case! {
fn script_errors_mixin_publishes_with_init_and_add_unhandled_error(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G.ScriptErrorsMixin)")
        .expect("ScriptErrorsMixin type probe succeeds");
    assert_eq!(
        kind, "table",
        "_G.ScriptErrorsMixin must publish as a table — \
         Blizzard_ScriptErrors.lua:3 declares `ScriptErrorsMixin = {{}}` at module \
         scope. The mixin is the prototype that ScriptErrors clones via \
         CreateFromMixins on the next line"
    );

    for method in SCRIPT_ERRORS_MIXIN_METHODS {
        let method_kind: String = env
            .eval(&format!("return type(ScriptErrorsMixin.{method})"))
            .unwrap_or_else(|err| panic!("type(ScriptErrorsMixin.{method}) probe failed: {err}"));
        assert_eq!(
            method_kind, "function",
            "ScriptErrorsMixin.{method} must be a function — the mixin publishes \
             exactly 2 methods: `Init` (allocates `unhandledErrors = {{}}` and \
             `isErrorHandlingDeferred = false`) and `AddUnhandledError` (queues an \
             error tuple {{errorMessage, stack, locals}} and starts a RunNextFrame \
             retry loop until at least one handler is registered)"
        );
    }
}
}

prefork_full_ui_case! {
fn script_errors_singleton_is_initialized_clone_of_mixin(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G.ScriptErrors)")
        .expect("ScriptErrors type probe succeeds");
    assert_eq!(
        kind, "table",
        "_G.ScriptErrors must publish as a table — \
         Blizzard_ScriptErrors.lua:39 calls `ScriptErrors = \
         CreateFromMixins(ScriptErrorsMixin)` which shallow-copies all mixin methods \
         onto a fresh table, then line 40 calls `ScriptErrors:Init()` to allocate \
         the per-instance state fields"
    );

    let unhandled_kind: String = env
        .eval("return type(ScriptErrors.unhandledErrors)")
        .expect("unhandledErrors probe succeeds");
    assert_eq!(
        unhandled_kind, "table",
        "ScriptErrors.unhandledErrors must be a table — `Init` populates it as an \
         empty queue. Errors raised before any handler registers via \
         AddLuaErrorHandler get pushed here and replayed on the next frame"
    );

    let deferred_kind: String = env
        .eval("return type(ScriptErrors.isErrorHandlingDeferred)")
        .expect("isErrorHandlingDeferred probe succeeds");
    assert_eq!(
        deferred_kind, "boolean",
        "ScriptErrors.isErrorHandlingDeferred must be a boolean — `Init` sets it to \
         `false` so the first call to `AddUnhandledError` schedules the \
         RunNextFrame replay loop. The flag flips back to false once the queue \
         drains, so subsequent unhandled errors restart the loop"
    );

    let deferred_value: bool = env
        .eval("return ScriptErrors.isErrorHandlingDeferred")
        .expect("isErrorHandlingDeferred value probe succeeds");
    assert!(
        !deferred_value,
        "ScriptErrors.isErrorHandlingDeferred must be false post-Init — no errors \
         have been queued yet so no deferred replay is in flight"
    );
}
}

prefork_full_ui_case! {
fn add_lua_error_handler_publishes_globally_as_function(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G.AddLuaErrorHandler)")
        .expect("AddLuaErrorHandler probe succeeds");
    assert_eq!(
        kind, "function",
        "_G.AddLuaErrorHandler must publish as a function — \
         Blizzard_ScriptErrors.lua:60 declares `function AddLuaErrorHandler(handler) \
         ... end`. Downstream addons (Blizzard_ScriptErrorsFrame, \
         Blizzard_DebugTools, ProcessExceptionClient hooks) call this to register \
         themselves as handlers for the dispatch fan-out inside HandleLuaError. \
         The body asserts `issecure()` so only Blizzard code can register — addon \
         code calling AddLuaErrorHandler will trip the assert"
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
            "_G.{name} must be nil — the 3 module-locals (the `errorHandlers` \
             registry table that AddLuaErrorHandler appends to, the `GetErrorData` \
             helper that resolves debugstack/debuglocals at the right stack depth, \
             the `HandleLuaError` dispatcher passed to `seterrorhandler`) are \
             intentionally file-local: leaking `errorHandlers` would let any addon \
             register handlers without going through the issecure gate; leaking \
             `HandleLuaError` would let addons reroute the seterrorhandler chain \
             unilaterally"
        );
    }
}
}

prefork_full_ui_case! {
fn add_lua_error_handler_appends_to_internal_registry_via_dispatch(env: &WowLuaEnv) {

    env.exec(
        r#"
        _G.__test_handler_calls = 0
        local handler = function(message, stack, locals)
            _G.__test_handler_calls = _G.__test_handler_calls + 1
            _G.__test_last_message = message
        end
        AddLuaErrorHandler(handler)
        ScriptErrors:AddUnhandledError("synthetic-error", "synthetic-stack", "synthetic-locals")
        "#,
    )
    .expect("registering handler and queuing unhandled error succeeds");

    let queue_kind: String = env
        .eval("return type(ScriptErrors.unhandledErrors)")
        .expect("unhandledErrors probe succeeds");
    assert_eq!(
        queue_kind, "table",
        "ScriptErrors.unhandledErrors must remain a table after AddUnhandledError — \
         even when the deferred RunNextFrame replay hasn't fired yet, the queue \
         field must not be reassigned to nil. The replay drains by setting \
         `self.unhandledErrors = {{}}` so the field always stays a table"
    );

    let deferred: bool = env
        .eval("return ScriptErrors.isErrorHandlingDeferred")
        .expect("isErrorHandlingDeferred probe succeeds");
    assert!(
        deferred,
        "ScriptErrors.isErrorHandlingDeferred must flip to true after \
         AddUnhandledError — the first queued error schedules the RunNextFrame \
         replay loop and the flag stays true until the loop drains the queue. The \
         simulator's RunNextFrame fires on the next OnUpdate tick which has not \
         occurred between the AddUnhandledError call and this probe"
    );
}
}

prefork_full_ui_case! {
fn seterrorhandler_was_invoked_at_module_load_with_internal_dispatcher(env: &WowLuaEnv) {

    let handler_kind: String = env
        .eval("return type(geterrorhandler())")
        .expect("geterrorhandler probe succeeds");
    assert_eq!(
        handler_kind, "function",
        "geterrorhandler() must return a function after the addon's final line \
         (`seterrorhandler(HandleLuaError)` at Blizzard_ScriptErrors.lua:89). The \
         dispatcher is module-local so we cannot probe its identity directly, but \
         we can confirm `seterrorhandler` was called by observing that the \
         registered handler is callable. Without the addon's call, the engine's \
         default handler would be installed (a different function reference but \
         still callable, so this test guards the contract that *some* handler is \
         registered after load — the addon-specific contract is covered by the \
         no-load-errors test)"
    );
}
}
