use std::path::PathBuf;

use wow_ui_sim::loader::{discover_all_blizzard_addons, discover_blizzard_addons_for_screen};
use wow_ui_sim::loader::{find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::paths::default_blizzard_ui_addons_path().expect("Blizzard UI cache should be available")
}

fn print_handler_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_PrintHandler")
}

fn print_handler_toc() -> PathBuf {
    print_handler_dir().join("Blizzard_PrintHandler.toc")
}

const PRINT_HANDLER_TOC_FILES: &[&str] = &["Blizzard_PrintHandler.lua"];

const PUBLIC_GLOBAL_FUNCTIONS: &[&str] = &["tostringall", "setprinthandler", "getprinthandler"];

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
fn blizzard_print_handler_find_toc_resolves_bare_variant() {
    let resolved = find_toc_file(&print_handler_dir()).expect("Blizzard_PrintHandler TOC resolves");
    assert_eq!(
        resolved,
        print_handler_toc(),
        "Blizzard_PrintHandler ships exactly one bare TOC — no `_Mainline.toc` variant. The \
         print handler is a foundational global infrastructure addon (overrides Lua's \
         built-in `print()` to route through the WoW chat frame) so a single bare TOC \
         drives every flavor"
    );

    let mainline = print_handler_dir().join("Blizzard_PrintHandler_Mainline.toc");
    assert!(
        !mainline.exists(),
        "There must be NO `_Mainline.toc` at {} — the bare TOC is the canonical entry point",
        mainline.display()
    );
}

#[test]
fn blizzard_print_handler_toc_declares_eager_on_every_screen() {
    let toc = TocFile::from_file(&print_handler_toc()).expect("Blizzard_PrintHandler TOC parses");

    assert!(
        !toc.is_load_on_demand(),
        "TOC must NOT declare `## LoadOnDemand:` — eager-load on every screen so `print()` \
         is hooked before any addon Lua chunk runs"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());

    assert!(
        !toc.is_game_type_restricted(),
        "TOC must NOT declare `## AllowLoadGameType:` — print() is universal infrastructure"
    );

    for screen in [
        ScreenKind::Game,
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        assert!(
            toc.allows_screen(screen),
            "`## AllowLoad: Both` must enable {screen:?} — `allows_screen` at \
             src/toc.rs:306-313 returns true for every ScreenKind when AllowLoad value is \
             `both` (case-insensitive). Required because print() must work identically on \
             glue (login error reporting) and in-world (addon debug output)"
        );
    }

    assert!(
        toc.dependencies().is_empty(),
        "Zero `## Dependencies:` — pure builtin-Lua surface. The single Lua file consumes \
         only Lua-builtin globals (type / error / tostring / next / unpack / pairs / \
         ipairs / select / wipe / pcall / string.join), the engine-level forceinsecure / \
         securecall / geterrorhandler / PrintToDebugWindow stubs, and DEFAULT_CHAT_FRAME \
         (which it nil-checks defensively at line 69 because the chat frame may not yet \
         exist on the earliest glue screens)"
    );
    assert!(
        toc.optional_deps().is_empty(),
        "Zero `## OptionalDeps:` — no soft sibling addons"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Zero saved variables — pure stateless code; the active print handler is a \
         file-local upvalue (LOCAL_PrintHandler at line 66) reset on every reload"
    );
}

#[test]
fn blizzard_print_handler_toc_declares_metadata_in_raw_bytes() {
    let raw = std::fs::read_to_string(print_handler_toc())
        .expect("Blizzard_PrintHandler TOC reads utf-8");
    assert!(
        raw.contains("## Title: Blizzard_PrintHandler"),
        "TOC must declare `## Title: Blizzard_PrintHandler` exactly — underscored-camelcase \
         form mirroring the addon directory name (NOT space-and-prose); library-flavored \
         title for a developer-facing infrastructure addon"
    );
    assert!(
        raw.contains("## Author: Blizzard Entertainment"),
        "TOC must declare `## Author: Blizzard Entertainment` exactly"
    );
    assert!(
        raw.contains("## AllowLoad: Both"),
        "TOC must declare `## AllowLoad: Both` exactly — universal screen-gate; the print \
         handler must hook `print()` on every screen"
    );
    assert!(
        !raw.contains("## DefaultState"),
        "TOC must NOT declare `## DefaultState:` — DefaultState defaults to enabled when \
         omitted; the print handler is non-optional anyway (disabling would break \
         debugging across the entire UI)"
    );
    assert!(
        !raw.contains("## LoadOnDemand"),
        "TOC must NOT declare `## LoadOnDemand:` — eager-load on every screen"
    );
    assert!(
        !raw.contains("## AllowLoadGameType"),
        "TOC must NOT declare `## AllowLoadGameType:` — universal infrastructure"
    );
    assert!(
        !raw.contains("## Dependencies"),
        "TOC must NOT declare `## Dependencies:` — leaf addon, pure Lua-builtin surface"
    );
    assert!(
        !raw.contains("## RequiredDep"),
        "TOC must NOT declare `## RequiredDep:` or `## RequiredDeps:`"
    );
    assert!(
        !raw.contains("## SavedVariables"),
        "TOC must NOT declare any `## SavedVariables*` keys — pure stateless code"
    );
    assert!(
        !raw.contains("## OptionalDeps"),
        "TOC must NOT declare any `## OptionalDeps:` — zero soft siblings"
    );
    assert!(
        !raw.contains("## UseSecureEnvironment"),
        "TOC must NOT declare `## UseSecureEnvironment:` — `print()` calls forceinsecure() \
         at line 85 and securecall(pcall, print_inner, ...) at line 94, deliberately \
         insecure-by-design so addon code can debug-print without tainting the secure \
         dispatch path"
    );
    assert!(
        !raw.contains("## Version"),
        "TOC must NOT declare `## Version:` — unversioned"
    );
}

#[test]
fn blizzard_print_handler_toc_lists_one_file() {
    let toc = TocFile::from_file(&print_handler_toc()).expect("Blizzard_PrintHandler TOC parses");
    let listed: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        listed, PRINT_HANDLER_TOC_FILES,
        "TOC body must list exactly 1 file: Blizzard_PrintHandler.lua — no XML, no \
         secondary helpers; the entire addon is a single 96-line Lua chunk"
    );
}

#[test]
fn blizzard_print_handler_appears_in_eager_discovery_for_every_screen() {
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
            .any(|(name, _)| name == "Blizzard_PrintHandler");
        assert!(
            found,
            "Blizzard_PrintHandler must appear in eager discovery for {screen:?} — \
             `## AllowLoad: Both` opens the addon to all 4 ScreenKind values, and there is \
             no LoadOnDemand / AllowLoadGameType filter to exclude it"
        );
    }
}

#[test]
fn blizzard_print_handler_appears_in_full_addon_inventory() {
    let inventory = discover_all_blizzard_addons(&blizzard_ui_dir());
    let found = inventory
        .iter()
        .any(|(name, _)| name == "Blizzard_PrintHandler");
    assert!(
        found,
        "Blizzard_PrintHandler must appear in `discover_all_blizzard_addons` — the \
         unfiltered inventory at src/loader/mod.rs:309-343 lists every parseable \
         Blizzard_* TOC"
    );
}

prefork_full_ui_case! {
fn blizzard_print_handler_is_addon_loaded_after_game_screen_boot(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_PrintHandler')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_PrintHandler') must return true after the eager \
         Game-screen sweep"
    );
}
}

prefork_full_ui_case! {
fn blizzard_print_handler_publishes_three_global_functions(env: &WowLuaEnv) {

    for name in PUBLIC_GLOBAL_FUNCTIONS {
        let kind: String = env
            .eval(&format!("return type(_G.{name})"))
            .unwrap_or_else(|err| panic!("type(_G.{name}) probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "_G.{name} must publish as a function — Blizzard_PrintHandler exposes 3 \
             global helpers: tostringall (varargs-tostring fold with fast-paths for n=0/1/2/3 \
             and a needfix optimization that skips conversion when every arg is already a \
             string, using a module-private LOCAL_ToStringAllTemp scratch buffer); \
             setprinthandler (validates the arg is a function, errors `Invalid print \
             handler` otherwise, swaps the file-local LOCAL_PrintHandler upvalue); \
             getprinthandler (returns the current LOCAL_PrintHandler closure). Plus the \
             addon overrides the global `print` function which routes through \
             securecall + pcall + forceinsecure to keep print taint out of the secure \
             dispatch path"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_print_handler_overrides_global_print_function(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G.print)")
        .expect("type(_G.print) probe succeeds");
    assert_eq!(
        kind, "function",
        "_G.print must publish as a function — the addon's last-line override \
         `function print(...)` replaces Lua's built-in print global with a securecall- \
         wrapped variant that delegates to LOCAL_PrintHandler. The luacheck inline-ignore \
         comment at line 93 (`-- luacheck: ignore 121`) acknowledges that this is \
         deliberately overwriting the read-only `print` global"
    );
}
}

prefork_full_ui_case! {
fn blizzard_print_handler_default_handler_routes_through_get_handler(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(getprinthandler())")
        .expect("type(getprinthandler()) probe succeeds");
    assert_eq!(
        kind, "function",
        "getprinthandler() must return a function on fresh load — the addon's file-local \
         LOCAL_PrintHandler at line 66 starts as a default closure that string.join's its \
         varargs with a single space delimiter via tostringall, then nil-checks \
         DEFAULT_CHAT_FRAME and calls `:AddMessage(printMsg)` if present. The default \
         handler is replaceable via setprinthandler(func)"
    );
}
}

prefork_full_ui_case! {
fn blizzard_print_handler_set_print_handler_validates_function_argument(env: &WowLuaEnv) {

    let valid: bool = env
        .eval(
            "local ok = pcall(setprinthandler, function() end); \
             return ok",
        )
        .expect("setprinthandler with function probe succeeds");
    assert!(
        valid,
        "setprinthandler(function() end) must succeed — the validator at line 75 only \
         rejects non-function arguments via `if type(func) ~= 'function' then error(...)`"
    );

    let invalid: bool = env
        .eval(
            "local ok, err = pcall(setprinthandler, 'not a function'); \
             return (not ok) and tostring(err):find('Invalid print handler') ~= nil",
        )
        .expect("setprinthandler with string probe succeeds");
    assert!(
        invalid,
        "setprinthandler('not a function') must error with `Invalid print handler` — \
         the explicit type-check at line 75-77 raises before the swap"
    );
}
}

prefork_full_ui_case! {
fn blizzard_print_handler_tostringall_handles_zero_to_three_arguments(env: &WowLuaEnv) {

    let no_args: i64 = env
        .eval("return select('#', tostringall())")
        .expect("tostringall() probe succeeds");
    assert_eq!(
        no_args, 0,
        "tostringall() with zero args must return zero values — the n==0 fast-path at \
         line 40-42 returns early without entering the conversion loop"
    );

    let one_arg: String = env
        .eval("return tostringall(42)")
        .expect("tostringall(42) probe succeeds");
    assert_eq!(
        one_arg, "42",
        "tostringall(42) must return '42' — the n==1 fast-path at line 32-33 calls \
         tostring(...) directly without entering the LOCAL_ToStringAllTemp scratch path"
    );

    let three_args: String = env
        .eval("local a, b, c = tostringall(1, true, nil); return a..'|'..b..'|'..c")
        .expect("tostringall(1, true, nil) probe succeeds");
    assert_eq!(
        three_args, "1|true|nil",
        "tostringall(1, true, nil) must return '1', 'true', 'nil' — the n==3 fast-path at \
         line 37-39 calls tostring(a), tostring(b), tostring(c) directly"
    );
}
}

prefork_full_ui_case! {
fn blizzard_print_handler_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_PrintHandler")
                || message.contains("PrintHandler")
                || message.contains("setprinthandler")
                || message.contains("getprinthandler")
                || message.contains("tostringall")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_PrintHandler emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}
