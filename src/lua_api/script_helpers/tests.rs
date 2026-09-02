use super::*;
use crate::lua_api::WowLuaEnv;
use crate::lua_api::methods::call_function_state_multi;
use rilua::{LuaApi, LuaApiMut};

#[test]
fn direct_state_call_restores_call_frame_after_lua_error() {
    let mut lua = rilua::Lua::new().expect("lua should initialize");
    let failing = lua
        .state_mut()
        .load("error('direct-state boom')")
        .expect("failing function should compile");

    let error = call_function_state(lua.state_mut(), Val::Function(failing.gc_ref()), &[])
        .expect_err("failing function should return its Lua error");
    assert!(error.to_string().contains("direct-state boom"));
    assert_eq!(
        lua.state().ci,
        0,
        "failed direct calls must unwind call frames"
    );

    let succeeding = lua
        .state_mut()
        .load("return 42")
        .expect("succeeding function should compile");
    let result = call_function_state(lua.state_mut(), Val::Function(succeeding.gc_ref()), &[])
        .expect("subsequent direct call should not see a stale Lua frame");
    assert_eq!(result, Val::Num(42.0));
}

#[test]
fn direct_void_state_call_does_not_retry_matching_lua_error_text() {
    let mut lua = rilua::Lua::new().expect("lua should initialize");
    lua.exec("__direct_void_error_calls = 0")
        .expect("counter should initialize");
    let loader = lua
        .state_mut()
        .load(
            r#"
                return function()
                    __direct_void_error_calls = __direct_void_error_calls + 1
                    error("expected Lua closure in execute: genuine Lua failure")
                end
            "#,
        )
        .expect("failing function source should compile");
    let failing = call_function_state(lua.state_mut(), Val::Function(loader.gc_ref()), &[])
        .expect("failing function should load");

    let error = call_void_function_state(lua.state_mut(), failing, &[])
        .expect_err("genuine Lua failure should be returned");
    assert!(error.contains("genuine Lua failure"));
    assert_eq!(
        lua.state().ci,
        0,
        "failed direct calls must unwind call frames"
    );

    let counter_loader = lua
        .state_mut()
        .load("return __direct_void_error_calls")
        .expect("counter source should compile");
    let calls = call_function_state(lua.state_mut(), Val::Function(counter_loader.gc_ref()), &[])
        .expect("subsequent direct call should succeed");
    assert_eq!(calls, Val::Num(1.0), "failing function must run once");
}

#[test]
fn direct_void_state_call_runs_successful_function_once() {
    let mut lua = rilua::Lua::new().expect("lua should initialize");
    let loader = lua
        .state_mut()
        .load(
            r#"
                local calls = 0
                return function()
                    calls = calls + 1
                    return calls
                end,
                function()
                    return calls
                end
            "#,
        )
        .expect("function source should compile");
    let functions = call_function_state_multi(lua.state_mut(), Val::Function(loader.gc_ref()), &[])
        .expect("functions should load");

    call_void_function_state(lua.state_mut(), functions[0], &[])
        .expect("successful function should return");
    let calls = call_function_state(lua.state_mut(), functions[1], &[])
        .expect("counter function should return");
    assert_eq!(calls, Val::Num(1.0), "successful function must run once");
}

#[test]
fn protected_lua_pcall_state_caches_wrapper_factory() {
    let mut lua = rilua::Lua::new().expect("lua should initialize");
    let direct = lua
        .state_mut()
        .load("return function(value) return value end")
        .expect("wrapper source should compile");
    let direct_func = call_function_state(lua.state_mut(), Val::Function(direct.gc_ref()), &[])
        .expect("direct function should build");

    let first_results = protected_lua_pcall_state(lua.state_mut(), direct_func, &[Val::Num(7.0)])
        .expect("first protected call should succeed");
    assert_eq!(first_results, vec![Val::Num(7.0)]);

    let first_factory = registry_value(lua.state_mut(), "__protected_lua_pcall_wrapper_factory");
    assert!(
        !matches!(first_factory, Val::Nil),
        "wrapper factory should be cached in the registry"
    );

    let second_results = protected_lua_pcall_state(lua.state_mut(), direct_func, &[Val::Num(9.0)])
        .expect("second protected call should succeed");
    assert_eq!(second_results, vec![Val::Num(9.0)]);

    let second_factory = registry_value(lua.state_mut(), "__protected_lua_pcall_wrapper_factory");
    assert_eq!(
        first_factory, second_factory,
        "cached wrapper factory should be reused"
    );
}

#[test]
fn script_handlers_use_numeric_frame_subtables() {
    let mut lua = rilua::Lua::new().expect("lua should initialize");
    let direct = lua
        .state_mut()
        .load("return function() return 'ok' end")
        .expect("handler source should compile");
    let handler = call_function_state(lua.state_mut(), Val::Function(direct.gc_ref()), &[])
        .expect("handler function should build");

    set_script_binding(
        lua.state_mut(),
        42,
        "OnEvent",
        ScriptBinding::Normal,
        handler,
    );
    assert_eq!(
        get_script_binding(lua.state_mut(), 42, "OnEvent", ScriptBinding::Normal),
        Some(handler)
    );

    let scripts = registry_table(lua.state_mut(), SCRIPTS_KEY).expect("scripts table exists");
    let frame_table = match lua
        .state()
        .gc
        .tables
        .get(scripts)
        .expect("scripts table is live")
        .get_int(42)
    {
        Val::Table(table) => table,
        other => panic!("expected numeric frame subtable, got {other:?}"),
    };
    let frame_table_hash_size = lua
        .state()
        .gc
        .tables
        .get(frame_table)
        .expect("frame script table is live")
        .hash_size();
    assert_eq!(
        frame_table_hash_size,
        SCRIPT_HANDLER_TABLE_HASH_SLOTS as u32
    );
    assert!(matches!(
        table_get_str(lua.state_mut(), frame_table, "OnEvent"),
        Val::Function(_)
    ));
    assert_eq!(
        table_get_str(lua.state_mut(), scripts, "42_OnEvent"),
        Val::Nil,
        "script registry should not allocate flat composite keys"
    );

    remove_script_binding(lua.state_mut(), 42, "OnEvent", ScriptBinding::Normal);
    assert_eq!(
        get_script_binding(lua.state_mut(), 42, "OnEvent", ScriptBinding::Normal),
        None
    );
}

#[test]
fn protected_lua_pcall_state_returns_traceback_on_error() {
    let mut lua = rilua::Lua::new().expect("lua should initialize");
    lua.exec(
        r#"
            function __traceback_probe()
                local function inner()
                    error("protected boom")
                end
                inner()
            end
            "#,
    )
    .expect("probe source should load");

    let loader = lua
        .state_mut()
        .load("return __traceback_probe")
        .expect("probe loader should compile");
    let func = call_function_state(lua.state_mut(), Val::Function(loader.gc_ref()), &[])
        .expect("probe function should exist");
    let error = protected_lua_pcall_state(lua.state_mut(), func, &[])
        .expect_err("protected call should fail");

    assert!(
        error.contains("protected boom"),
        "error should include original failure, got: {error}"
    );
    assert!(
        error.contains("stack traceback:"),
        "error should include Lua traceback, got: {error}"
    );
    assert!(
        error.contains("inner"),
        "traceback should include failing call path, got: {error}"
    );
}

#[test]
fn call_error_handler_state_mirrors_first_seen_errors_to_console_output() {
    let env = WowLuaEnv::new().expect("env should initialize");
    {
        let mut lua = env.rilua_mut();
        call_error_handler_state(lua.state_mut(), "boom");
        call_error_handler_state(lua.state_mut(), "boom");
    }

    let state = env.state().borrow();
    let mirrored: Vec<_> = state
        .console_output
        .iter()
        .filter(|line| line.starts_with("Lua error: boom"))
        .collect();
    assert_eq!(
        mirrored.len(),
        1,
        "duplicate Lua errors should be deduped in GUI console output"
    );
}

#[test]
fn collect_lua_error_tracks_records_and_counts_without_console_mirroring() {
    let env = WowLuaEnv::new().expect("env should initialize");
    {
        let lua = env.rilua();
        let _ = collect_lua_error(lua.state(), "boom");
        let _ = collect_lua_error(lua.state(), "boom");
    }

    let state = env.state().borrow();
    assert_eq!(state.lua_errors.len(), 2, "raw errors should be recorded");
    assert_eq!(
        state.lua_error_records.len(),
        2,
        "addon-attributed records should be recorded"
    );
    assert_eq!(
        state.lua_error_counts.get("boom"),
        Some(&2),
        "normalized counts should increment"
    );
    assert!(
        state
            .console_output
            .iter()
            .all(|line| !line.starts_with("Lua error: boom")),
        "collector-only path should not mirror to GUI console"
    );
}

#[test]
fn active_error_handler_callback_failures_use_canonical_sink() {
    let env = WowLuaEnv::new().expect("env should initialize");
    {
        let mut lua = env.rilua_mut();
        lua.exec("seterrorhandler(function(msg) error('handler boom: ' .. msg) end)")
            .expect("seterrorhandler should install callback");
        call_error_handler_state(lua.state_mut(), "root boom");
        call_error_handler_state(lua.state_mut(), "root boom");
    }

    let state = env.state().borrow();
    assert_eq!(
        state.lua_error_counts.get("root boom"),
        Some(&2),
        "original errors should be counted through canonical sink"
    );
    let handler_count = state
        .lua_error_counts
        .iter()
        .find_map(|(key, count)| key.contains("handler boom: root boom").then_some(*count))
        .unwrap_or(0);
    assert_eq!(
        handler_count, 2,
        "error handler callback failures should also be counted via canonical sink"
    );
}
