use std::path::PathBuf;

use wow_ui_sim::loader::load_addon;
use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path()
        .expect("Blizzard UI cache should be available")
}

fn script_errors_frame_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_ScriptErrorsFrame")
}

fn script_errors_frame_toc() -> PathBuf {
    script_errors_frame_dir().join("Blizzard_ScriptErrorsFrame.toc")
}

const TOC_FILES: &[&str] = &[
    "Blizzard_ScriptErrorsFrame.lua",
    "Blizzard_ScriptErrorsFrame.xml",
];

const SCRIPT_ERRORS_FRAME_MIXIN_METHODS: &[&str] = &[
    "OnLoad",
    "OnShow",
    "Warn",
    "DisplayMessageInternal",
    "GetEditBox",
    "Update",
    "UpdateButtons",
    "GetCount",
    "ChangeDisplayedIndex",
    "ShowPrevious",
    "ShowNext",
];

const MODULE_LOCAL_NAMES: &[&str] = &[
    "ERROR_FORMAT",
    "WARNING_FORMAT",
    "ExcessiveMessageLimit",
    "ErrorMessageType",
    "WarningMessageType",
    "shouldHideErrorFramePredicate",
    "ShouldHideErrorFrame",
    "GetNavigationButtonEnabledStates",
];

const FRAME_CHILD_KEYS: &[&str] = &[
    "IndexLabel",
    "DragArea",
    "ScrollFrame",
    "Reload",
    "PreviousError",
    "NextError",
    "Close",
];

#[test]
fn escape_decimal_non_printables_preserves_printable_text_and_valid_utf8() {
    let env = WowLuaEnv::new().expect("Lua environment initializes");
    let escaped: String = env
        .eval(
            r#"
            return C_StringUtil.EscapeDecimalNonPrintables(
                "ASCII punctuation: !@#$%^&*() café 日本語"
            )
            "#,
        )
        .expect("EscapeDecimalNonPrintables accepts valid UTF-8");

    assert_eq!(escaped, "ASCII punctuation: !@#$%^&*() café 日本語");
}

#[test]
fn escape_decimal_non_printables_escapes_ascii_controls_except_tab_newline_and_return() {
    let env = WowLuaEnv::new().expect("Lua environment initializes");
    let matches_expected: bool = env
        .eval(
            r#"
            local slash = string.char(92)
            local input = {}
            local expected = {}
            for byte = 0, 31 do
                table.insert(input, string.char(byte))
                if byte == 9 or byte == 10 or byte == 13 then
                    table.insert(expected, string.char(byte))
                else
                    table.insert(expected, slash .. string.format("%03d", byte))
                end
            end
            table.insert(input, string.char(32, 126, 127))
            table.insert(expected, " ~" .. slash .. "127")
            return C_StringUtil.EscapeDecimalNonPrintables(table.concat(input))
                == table.concat(expected)
            "#,
        )
        .expect("EscapeDecimalNonPrintables accepts ASCII bytes");

    assert!(matches_expected);
}

#[test]
fn escape_decimal_non_printables_escapes_each_invalid_utf8_byte() {
    let env = WowLuaEnv::new().expect("Lua environment initializes");
    let matches_expected: bool = env
        .eval(
            r#"
            local slash = string.char(92)
            local input = "é" .. string.char(195, 40, 255, 226, 130) .. "日本語"
            local expected = "é" .. slash .. "195(" .. slash .. "255"
                .. slash .. "226" .. slash .. "130" .. "日本語"
            return C_StringUtil.EscapeDecimalNonPrintables(input) == expected
            "#,
        )
        .expect("EscapeDecimalNonPrintables accepts invalid UTF-8 bytes");

    assert!(matches_expected);
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
fn find_toc_file_resolves_bare_toc() {
    let resolved =
        find_toc_file(&script_errors_frame_dir()).expect("Blizzard_ScriptErrorsFrame TOC resolves");
    assert_eq!(
        resolved,
        script_errors_frame_toc(),
        "Blizzard_ScriptErrorsFrame ships exactly one bare TOC. The visual error \
         frame is engine-flavor-agnostic — same Lua + XML on every flavor"
    );
}

#[test]
fn toc_declares_eager_both_with_two_hard_dependencies() {
    let toc = TocFile::from_file(&script_errors_frame_toc())
        .expect("Blizzard_ScriptErrorsFrame TOC parses");

    assert!(
        !toc.is_load_on_demand(),
        "TOC must NOT declare LoadOnDemand — the visual handler must register via \
         `AddLuaErrorHandler` at OnLoad before downstream addons can produce errors. \
         If it loaded lazily, errors raised before user interaction would queue \
         silently in `ScriptErrors.unhandledErrors` until the user navigated to a \
         feature that pulled the frame in"
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
            "`## AllowLoad: Both` must enable {screen:?} — glue-screen errors must \
             show in the same UI as in-game errors so login/character-select addon \
             bugs surface visually"
        );
    }

    let deps = toc.dependencies();
    assert_eq!(
        deps,
        vec![
            "Blizzard_ScriptErrors".to_string(),
            "Blizzard_SharedXML".to_string(),
        ],
        "TOC must declare exactly 2 hard `## Dependencies: Blizzard_ScriptErrors, \
         Blizzard_SharedXML`. Order is load-order — Blizzard_ScriptErrors must be \
         fully loaded so `AddLuaErrorHandler` is published before this frame's \
         OnLoad runs; Blizzard_SharedXML provides UIPanelDialogTemplate / \
         ScrollFrameTemplate / TitleDragAreaTemplate / UIPanelButtonTemplate / \
         ScrollingEdit_OnLoad / Wrap / PrintToDebugWindow"
    );
    assert!(toc.optional_deps().is_empty());
    assert!(toc.saved_variables().is_empty());
    assert!(toc.saved_variables_per_character().is_empty());
}

#[test]
fn toc_declares_default_state_enabled_with_escalate_during_load() {
    let toc = TocFile::from_file(&script_errors_frame_toc())
        .expect("Blizzard_ScriptErrorsFrame TOC parses");
    assert!(
        toc.default_enabled(),
        "TOC must ship `## DefaultState: enabled` — the visual error UI must be \
         available out of the box"
    );

    let raw = std::fs::read_to_string(script_errors_frame_toc())
        .expect("Blizzard_ScriptErrorsFrame TOC reads utf-8");
    assert!(
        raw.contains("## EscalateErrorDuringLoad: 1"),
        "TOC must declare `## EscalateErrorDuringLoad: 1` — same flag as \
         Blizzard_ScriptErrors. If THIS addon errors during load, the engine \
         escalates the failure rather than swallowing it. Without escalation, a \
         silent load failure here would leave the pipeline functional (errors \
         routed via Blizzard_ScriptErrors handler list) but invisible (no UI to \
         display them)"
    );
}

#[test]
fn toc_declares_metadata_in_raw_bytes_with_singular_error_in_title() {
    let raw = std::fs::read_to_string(script_errors_frame_toc())
        .expect("Blizzard_ScriptErrorsFrame TOC reads utf-8");
    assert!(
        raw.contains("## Title: Blizzard Script Error Frame"),
        "TOC must declare `## Title: Blizzard Script Error Frame` (note: singular \
         \"Error\" matching Blizzard_ScriptErrors's `## Title: Blizzard Script \
         Error` typo — the folder/file name is plural \"ScriptErrors\" but the \
         displayed title is singular)"
    );
    assert!(raw.contains("## Author: Blizzard Entertainment"));
    assert!(raw.contains("## DefaultState: enabled"));
    assert!(raw.contains("## AllowLoad: Both"));
    assert!(raw.contains("## Dependencies: Blizzard_ScriptErrors, Blizzard_SharedXML"));
}

#[test]
fn toc_lists_one_lua_and_one_xml() {
    let toc = TocFile::from_file(&script_errors_frame_toc())
        .expect("Blizzard_ScriptErrorsFrame TOC parses");
    let listed: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        listed, TOC_FILES,
        "TOC body must list exactly 2 files: Blizzard_ScriptErrorsFrame.lua then \
         Blizzard_ScriptErrorsFrame.xml. Order matters because the XML's \
         `mixin=\"ScriptErrorsFrameMixin\"` and `<OnLoad method=\"OnLoad\"/>` \
         binding resolve the Lua-defined mixin global at parse time — the .lua \
         must load first so ScriptErrorsFrameMixin exists when the .xml is parsed"
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
            .any(|(name, _)| name == "Blizzard_ScriptErrorsFrame");
        assert!(
            found,
            "Blizzard_ScriptErrorsFrame must auto-discover on screen {screen:?} — \
             eager (no LoadOnDemand) AND `## AllowLoad: Both` makes \
             `discover_blizzard_addons_for_screen` include the addon on every screen"
        );
    }
}

#[test]
fn root_directory_holds_lua_xml_and_toc() {
    let dir = script_errors_frame_dir();
    assert!(dir.join("Blizzard_ScriptErrorsFrame.lua").is_file());
    assert!(dir.join("Blizzard_ScriptErrorsFrame.xml").is_file());
    assert!(dir.join("Blizzard_ScriptErrorsFrame.toc").is_file());

    let entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("read addon dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries.len(),
        3,
        "Blizzard_ScriptErrorsFrame directory must contain exactly 3 entries \
         (1 lua + 1 xml + 1 toc), got {entries:?}"
    );
}

#[test]
fn xml_wraps_frame_in_scoped_modifier_with_secure_env_addition() {
    let xml_path = script_errors_frame_dir().join("Blizzard_ScriptErrorsFrame.xml");
    let raw = std::fs::read_to_string(&xml_path).expect("XML reads utf-8");
    assert!(
        raw.contains("<ScopedModifier addToSecureEnv=\"true\">"),
        "XML must wrap the frame in `<ScopedModifier addToSecureEnv=\"true\">` — \
         this directive (parsed by xml_file.rs:82-103) tells the loader to \
         register every immediate child as a member of the secure environment \
         scope. ScriptErrorsFrame must be in the secure env so the frame and its \
         children (especially the `Reload` button which calls `ReloadUI` and the \
         scrollable error text editor) can be referenced and scripted by tainted \
         caller stacks without losing access — the error frame itself MUST function \
         even when invoked from tainted code paths, otherwise tainted-callstack \
         errors couldn't surface their UI"
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
            message.contains("Blizzard_ScriptErrorsFrame")
                || message.contains("ScriptErrorsFrameMixin")
                || message.contains("SetHideErrorFramePredicate")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_ScriptErrorsFrame emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn is_addon_loaded_after_eager_sweep(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_ScriptErrorsFrame')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_ScriptErrorsFrame') must return true \
         after the eager Game-screen sweep"
    );
}
}

prefork_full_ui_case! {
fn script_errors_frame_mixin_publishes_with_eleven_methods(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G.ScriptErrorsFrameMixin)")
        .expect("ScriptErrorsFrameMixin type probe succeeds");
    assert_eq!(
        kind, "table",
        "_G.ScriptErrorsFrameMixin must publish as a table — \
         Blizzard_ScriptErrorsFrame.lua:33 declares `ScriptErrorsFrameMixin = {{}}` \
         and the XML's `mixin=\"ScriptErrorsFrameMixin\"` attribute on \
         ScriptErrorsFrame applies the mixin's methods to that frame at parse time"
    );

    for method in SCRIPT_ERRORS_FRAME_MIXIN_METHODS {
        let method_kind: String = env
            .eval(&format!("return type(ScriptErrorsFrameMixin.{method})"))
            .unwrap_or_else(|err| {
                panic!("type(ScriptErrorsFrameMixin.{method}) probe failed: {err}")
            });
        assert_eq!(
            method_kind, "function",
            "ScriptErrorsFrameMixin.{method} must be a function — the mixin \
             publishes 11 methods: OnLoad (RegisterForDrag, allocates errorData/seen \
             tables, registers AddLuaErrorHandler closure that calls \
             DisplayMessageInternal with ErrorMessageType, registers OnEvent for \
             LUA_WARNING that calls DisplayMessageInternal with WarningMessageType, \
             wires PreviousError/NextError OnClick, ScrollFrame.Text \
             OnUpdate/OnEditFocusGained); OnShow (calls Update); Warn (entry point \
             for direct warning injection — the comment 'For outlier cases where \
             it's necessary to provide the script error frame with a warning \
             message directly. Please avoid if possible.' documents that this is \
             discouraged); DisplayMessageInternal (the central message dedup + \
             render path — keys errors by `message\\nstack` in the `seen` table, \
             increments `count` on duplicates, calls PrintToDebugWindow on \
             first-occurrence, auto-Shows when not hidden by \
             ShouldHideErrorFrame('scriptErrors'), invokes the messageCounter \
             closure and calls OnExcessiveErrors at >=1000 messages); GetEditBox (returns \
             ScrollFrame.Text); Update (renders ERROR_FORMAT or WARNING_FORMAT, \
             triggers ScrollingEdit re-layout, resets vertical scroll); \
             UpdateButtons (enable/disable PreviousError/NextError, set IndexLabel \
             text); GetCount; ChangeDisplayedIndex (calls Wrap from SharedXML to \
             cycle errorData index); ShowPrevious; ShowNext"
        );
    }
}
}

prefork_full_ui_case! {
fn set_hide_error_frame_predicate_publishes_globally(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G.SetHideErrorFramePredicate)")
        .expect("SetHideErrorFramePredicate probe succeeds");
    assert_eq!(
        kind, "function",
        "_G.SetHideErrorFramePredicate must publish as a function — \
         Blizzard_ScriptErrorsFrame.lua:19. The body asserts `issecure()` AND \
         `type(predicate) == 'function'`. The comment '-- For internal use only.' \
         documents that despite being a global, only Blizzard code should call it. \
         Internal callers (boss-fight cinematics, certain cutscene sequences) \
         install a predicate that suppresses the popup frame during specific \
         gameplay moments where script errors would interrupt the immersive \
         experience"
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
            "_G.{name} must be nil — the 8 module-locals (ERROR_FORMAT/WARNING_FORMAT \
             chat-color-coded format strings, ExcessiveMessageLimit=1000 sentinel, \
             ErrorMessageType=0 / WarningMessageType=1 enum sentinels, \
             shouldHideErrorFramePredicate state holder, ShouldHideErrorFrame helper, \
             GetNavigationButtonEnabledStates helper) are intentionally file-local. \
             Leaking them would let addons override the format strings, lower the \
             excessive-message threshold to suppress legitimate errors, or swap the \
             enum sentinels to misroute messages"
        );
    }
}
}

prefork_full_ui_case! {
fn script_errors_frame_publishes_as_named_global_hidden_by_default(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G.ScriptErrorsFrame)")
        .expect("ScriptErrorsFrame global probe succeeds");
    assert_eq!(
        kind, "table",
        "_G.ScriptErrorsFrame must publish as a table — XML declares the named \
         non-virtual frame at root scope with `name=\"ScriptErrorsFrame\"`"
    );

    let shown: bool = env
        .eval("return ScriptErrorsFrame:IsShown()")
        .expect("IsShown probe succeeds");
    assert!(
        !shown,
        "ScriptErrorsFrame must start hidden — XML declares `hidden=\"true\"`. The \
         frame only auto-shows when DisplayMessageInternal queues an error AND \
         `ShouldHideErrorFrame('scriptErrors')` returns false (i.e., the \
         scriptErrors CVar is enabled and no internal predicate suppresses it)"
    );

    let strata: String = env
        .eval("return ScriptErrorsFrame:GetFrameStrata()")
        .expect("GetFrameStrata probe succeeds");
    assert_eq!(
        strata, "TOOLTIP",
        "ScriptErrorsFrame must use frameStrata=TOOLTIP — XML attribute. The \
         highest standard strata ensures the error popup renders above every other \
         UI element so it cannot be obscured by a misbehaving addon's frame stack"
    );
}
}

prefork_full_ui_case! {
fn script_errors_frame_publishes_seven_named_child_keys(env: &WowLuaEnv) {

    for child_key in FRAME_CHILD_KEYS {
        let child_kind: String = env
            .eval(&format!("return type(ScriptErrorsFrame.{child_key})"))
            .unwrap_or_else(|err| {
                panic!("type(ScriptErrorsFrame.{child_key}) probe failed: {err}")
            });
        assert_eq!(
            child_kind, "table",
            "ScriptErrorsFrame.{child_key} must publish as a table (frame/region) — \
             the XML body declares 7 parentKey children: IndexLabel (FontString \
             showing 'N / total' under the scrollframe), DragArea (overlays the \
             title bar so left-button drag moves the panel), ScrollFrame \
             (ScrollFrameTemplate with MinimalScrollBar holding the editable \
             multi-line text), Reload (UIPanelButton calling ReloadUI), \
             PreviousError/NextError (32×32 spellbook-prev/next-page texture \
             buttons), Close (UIPanelButton calling HideParentPanel)"
        );
    }
}
}

prefork_full_ui_case! {
fn registers_lua_warning_event_after_onload(env: &WowLuaEnv) {

    let registered: bool = env
        .eval("return ScriptErrorsFrame:IsEventRegistered('LUA_WARNING')")
        .expect("IsEventRegistered probe succeeds");
    assert!(
        registered,
        "ScriptErrorsFrame:IsEventRegistered('LUA_WARNING') must return true after \
         OnLoad — line 70 calls `self:RegisterEvent('LUA_WARNING')` so the engine \
         dispatches LUA_WARNING events to the OnEvent script wired at line 47, \
         which calls DisplayMessageInternal(warningMessage, WarningMessageType). \
         LUA_WARNING is the soft-error path (e.g., deprecated API usage, \
         non-fatal pattern matches) — distinct from the seterrorhandler path that \
         routes hard errors via AddLuaErrorHandler"
    );
}
}

prefork_full_ui_case! {
fn display_message_internal_dedups_identical_message_stack_pairs(env: &WowLuaEnv) {

    env.exec(
        r#"
        ScriptErrorsFrame.errorData = {}
        ScriptErrorsFrame.seen = {}
        ScriptErrorsFrame.index = 0
        ScriptErrorsFrame:DisplayMessageInternal("synthetic-error", 0, "synthetic-stack", "synthetic-locals")
        ScriptErrorsFrame:DisplayMessageInternal("synthetic-error", 0, "synthetic-stack", "synthetic-locals")
        ScriptErrorsFrame:DisplayMessageInternal("synthetic-error", 0, "synthetic-stack", "synthetic-locals")
        "#,
    )
    .expect("DisplayMessageInternal sequence succeeds");

    let count: f64 = env
        .eval("return ScriptErrorsFrame:GetCount()")
        .expect("GetCount probe succeeds");
    assert_eq!(
        count, 1.0,
        "GetCount must return 1 after 3 identical (message, stack) pairs — the \
         `seen[messageKey]` table dedups by `message\\nstack` and increments the \
         existing entry's `count` field instead of appending. This is what lets the \
         UI show 'Count: 47' for a tight-loop error without flooding the navigation \
         bar"
    );

    let entry_count: f64 = env
        .eval("return ScriptErrorsFrame.errorData[1].count")
        .expect("errorData[1].count probe succeeds");
    assert_eq!(
        entry_count, 3.0,
        "errorData[1].count must equal 3 after 3 identical raises — the dedup \
         path increments the count field instead of appending a new errorData \
         entry"
    );
}
}
