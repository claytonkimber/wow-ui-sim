//! `C_InstanceEncounter` state-backed encounter probes.

use crate::c_api::ensure_namespace;
use crate::client_profile::{ACTIVE, ClientProfile};
use crate::lua_api::methods::borrow_state;
use crate::lua_bridge::table_set_rust_fn_static;
use rilua::vm::state::LuaState;
use rilua::{LuaResult, Val};

pub(crate) fn register_c_instance_encounter_surface(state: &mut LuaState) -> LuaResult<()> {
    if !matches!(ACTIVE, ClientProfile::Retail | ClientProfile::Ptr) {
        return Ok(());
    }

    let namespace = ensure_namespace(state, "C_InstanceEncounter")?;
    table_set_rust_fn_static(
        state,
        namespace,
        "IsEncounterInProgress",
        is_encounter_in_progress,
    )?;
    Ok(())
}

pub(crate) fn is_encounter_in_progress(state: &mut LuaState) -> LuaResult<u32> {
    let encounter_in_progress = borrow_state(state)?.world.encounter_in_progress;
    state.push(Val::Bool(encounter_in_progress));
    Ok(1)
}
