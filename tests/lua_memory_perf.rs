#![cfg(feature = "gui")]

use crate::common;
use crate::perf_base_game;
use crate::perf_game_ui;
use crate::perf_lua_memory;

use perf_game_ui::load_timed_game_ui;
use perf_lua_memory::measure_settled_lua_memory_kib;

const LUA_MEMORY_BUDGET_KIB: f64 = 110_000.0;

#[test]
fn settled_game_ui_lua_memory_stays_under_budget() {
    perf_test_timeout! {
        let loaded_ui = load_timed_game_ui();
        assert!(
            loaded_ui.startup_elapsed.as_nanos() > 0,
            "shared perf UI startup timing should be captured"
        );

        let lua_memory_kib = measure_settled_lua_memory_kib(&loaded_ui);
        eprintln!(
            "Lua memory baseline after full startup: {:.2} KiB (budget {:.2} KiB)",
            lua_memory_kib,
            LUA_MEMORY_BUDGET_KIB
        );

        assert!(
            lua_memory_kib < LUA_MEMORY_BUDGET_KIB,
            "Lua memory {:.2} KiB exceeds budget {:.2} KiB",
            lua_memory_kib,
            LUA_MEMORY_BUDGET_KIB
        );
    }
}
