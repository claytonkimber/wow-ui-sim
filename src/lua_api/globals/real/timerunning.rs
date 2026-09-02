//! Legacy timerunning globals backed by simulator season state.

use crate::lua_api::methods::borrow_state;
use crate::lua_bridge::table_set_rust_fn_static;
use rilua::vm::state::LuaState;
use rilua::{LuaApiMut, LuaResult, Val};

fn is_timerunning_enabled(state: &mut LuaState) -> LuaResult<u32> {
    let enabled = borrow_state(state)?.timerunning_season_id.is_some();
    state.push(Val::Bool(enabled));
    Ok(1)
}

fn get_remaining_timerunning_season_seconds(state: &mut LuaState) -> LuaResult<u32> {
    let remaining_seconds = {
        let sim = borrow_state(state)?;
        sim.timerunning_season_id
            .map(|_| sim.timerunning_season_seconds_remaining)
            .unwrap_or_default()
    };
    state.push(Val::Num(remaining_seconds as f64));
    Ok(1)
}

pub fn register_all(lua: &mut rilua::Lua) -> LuaResult<()> {
    let state = lua.state_mut();
    let global = state.global;
    table_set_rust_fn_static(
        state,
        global,
        "IsTimerunningEnabled",
        is_timerunning_enabled,
    )?;
    table_set_rust_fn_static(
        state,
        global,
        "GetRemainingTimerunningSeasonSeconds",
        get_remaining_timerunning_season_seconds,
    )?;
    Ok(())
}
