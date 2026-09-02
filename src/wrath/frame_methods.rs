//! Wrath-only no-op frame method stubs.
//!
//! These methods existed in the Wrath client but were removed or moved in
//! later retail builds. Stubbed here so wrath addon code does not crash on
//! missing method errors.
//!
//! Compiled under `client-wrath`, `client-mists`, `client-era`, and `client-anniversary`.

use crate::lua_bridge::table_set_rust_fn_static;
use rilua::vm::gc::arena::GcRef;
use rilua::vm::state::LuaState;
use rilua::vm::table::Table;
use rilua::{LuaResult, Val};

pub fn register_all(state: &mut LuaState, mt: GcRef<Table>) -> LuaResult<()> {
    table_set_rust_fn_static(state, mt, "IgnoreDepth", ignore_depth)?;
    table_set_rust_fn_static(state, mt, "SetBackdropColor", set_backdrop_color)?;
    table_set_rust_fn_static(
        state,
        mt,
        "SetBackdropBorderColor",
        set_backdrop_border_color,
    )?;
    table_set_rust_fn_static(
        state,
        mt,
        "SetPlayerTextureHeight",
        set_player_texture_height,
    )?;
    table_set_rust_fn_static(state, mt, "SetPlayerTextureWidth", set_player_texture_width)?;
    table_set_rust_fn_static(state, mt, "SetMaxBytes", set_max_bytes)?;
    table_set_rust_fn_static(state, mt, "GetTextHeight", get_text_height)?;
    table_set_rust_fn_static(state, mt, "OpenTicket", browser_navigate)?;
    Ok(())
}

/// Wrath frame method that controlled depth-buffer interactions. No-op stub.
fn ignore_depth(_state: &mut LuaState) -> LuaResult<u32> {
    Ok(0)
}

/// Wrath had `SetBackdropBorderColor` directly on Frame; retail moved it to
/// `BackdropTemplateMixin`. No-op stub — real backdrop rendering is out of scope.
fn set_backdrop_border_color(_state: &mut LuaState) -> LuaResult<u32> {
    Ok(0)
}

/// Wrath had `SetBackdropColor` directly on Frame; same migration as the
/// border-color counterpart. No-op stub.
fn set_backdrop_color(_state: &mut LuaState) -> LuaResult<u32> {
    Ok(0)
}

/// Wrath PlayerModel method. No-op stub.
fn set_player_texture_height(_state: &mut LuaState) -> LuaResult<u32> {
    Ok(0)
}

/// Companion to `SetPlayerTextureHeight` — wrath Minimap calls
/// `Minimap:SetPlayerTextureWidth(40)` during init.
fn set_player_texture_width(_state: &mut LuaState) -> LuaResult<u32> {
    Ok(0)
}

/// Wrath EditBox method. Real implementation would clamp input length to
/// `bytes`; no-op suffices for unblocking load.
fn set_max_bytes(_state: &mut LuaState) -> LuaResult<u32> {
    Ok(0)
}

/// Wrath Button method that returns the rendered FontString height. Used by
/// `GossipResize`, quest title buttons, HelpFrame to size buttons. Returning
/// a fixed 12 (default font line height) matches what wrath text renders to
/// closely enough for layout to settle.
fn get_text_height(state: &mut LuaState) -> LuaResult<u32> {
    state.push(Val::Num(12.0));
    Ok(1)
}

/// Classic Browser frame method. The simulator has no embedded browser
/// engine, so navigation records no external side effect but must be callable
/// by HelpFrame and other Blizzard panels.
fn browser_navigate(_state: &mut LuaState) -> LuaResult<u32> {
    Ok(0)
}
