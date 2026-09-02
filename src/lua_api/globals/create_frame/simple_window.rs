//! Frame-backed SimpleWindow objects created by the global `CreateWindow` API.

use crate::lua_api::methods::{
    borrow_state, borrow_state_mut, frame_id_from_stack, frame_ref, get_or_create_frame_fields,
    table_get, table_set_static,
};
use crate::lua_bridge::{FromStack, stack_val, table_set_rust_fn_static};
use crate::widget::WidgetType;
use rilua::vm::state::LuaState;
use rilua::{LuaResult, Val};

const TOPMOST_FIELD: &str = "__simpleWindowTopmost";
const MIN_WIDTH_FIELD: &str = "__simpleWindowMinWidth";
const MIN_HEIGHT_FIELD: &str = "__simpleWindowMinHeight";

pub(super) fn create_window(state: &mut LuaState) -> LuaResult<u32> {
    let topmost = bool_arg_or_default(state, 2, false)?;
    let window_id = super::create_frame_instance(
        state,
        WidgetType::Frame,
        "SimpleWindow",
        None,
        None,
        false,
        None,
    )?;

    initialize_window_frame(state, window_id)?;
    install_window_fields(state, window_id, topmost)?;
    let window = frame_ref(state, window_id)?;
    state.push(window);
    Ok(1)
}

fn bool_arg_or_default(state: &LuaState, stack_index: i32, default: bool) -> LuaResult<bool> {
    if stack_val(state, stack_index) == Val::Nil {
        return Ok(default);
    }
    bool::from_stack(state, stack_index)
}

fn initialize_window_frame(state: &mut LuaState, window_id: u64) -> LuaResult<()> {
    let mut sim = borrow_state_mut(state)?;
    if let Some(window) = sim.widgets.get_mut_visual(window_id) {
        window.movable = true;
        window.resizable = true;
    }
    Ok(())
}

fn install_window_fields(state: &mut LuaState, window_id: u64, topmost: bool) -> LuaResult<()> {
    let fields = get_or_create_frame_fields(state, window_id);
    table_set_static(state, fields, TOPMOST_FIELD, Val::Bool(topmost));
    table_set_static(state, fields, MIN_WIDTH_FIELD, Val::Num(0.0));
    table_set_static(state, fields, MIN_HEIGHT_FIELD, Val::Num(0.0));

    let Val::Table(fields_ref) = fields else {
        return Ok(());
    };
    table_set_rust_fn_static(state, fields_ref, "SetTitle", no_result)?;
    table_set_rust_fn_static(state, fields_ref, "SetWindowSize", set_window_size)?;
    table_set_rust_fn_static(state, fields_ref, "SetMinSize", set_min_size)?;
    table_set_rust_fn_static(state, fields_ref, "SetFocus", no_result)?;
    table_set_rust_fn_static(state, fields_ref, "IsTopmost", is_topmost)?;
    table_set_rust_fn_static(state, fields_ref, "SetTopmost", set_topmost)?;
    table_set_rust_fn_static(state, fields_ref, "Close", close)?;
    Ok(())
}

fn no_result(_state: &mut LuaState) -> LuaResult<u32> {
    Ok(0)
}

fn set_window_size(state: &mut LuaState) -> LuaResult<u32> {
    let window_id = frame_id_from_stack(state, 1)?;
    let requested_width = f64::from_stack(state, 2)? as f32;
    let requested_height = f64::from_stack(state, 3)? as f32;
    let fields = get_or_create_frame_fields(state, window_id);
    let width = requested_width.max(number_field(state, fields, MIN_WIDTH_FIELD));
    let height = requested_height.max(number_field(state, fields, MIN_HEIGHT_FIELD));
    update_window_size(state, window_id, width, height)?;
    Ok(0)
}

fn set_min_size(state: &mut LuaState) -> LuaResult<u32> {
    let window_id = frame_id_from_stack(state, 1)?;
    let min_width = f64::from_stack(state, 2)? as f32;
    let min_height = f64::from_stack(state, 3)? as f32;
    let fields = get_or_create_frame_fields(state, window_id);
    table_set_static(state, fields, MIN_WIDTH_FIELD, Val::Num(min_width as f64));
    table_set_static(state, fields, MIN_HEIGHT_FIELD, Val::Num(min_height as f64));

    let (width, height) = current_window_size(state, window_id)?;
    update_window_size(
        state,
        window_id,
        width.max(min_width),
        height.max(min_height),
    )?;
    Ok(0)
}

fn current_window_size(state: &mut LuaState, window_id: u64) -> LuaResult<(f32, f32)> {
    let sim = borrow_state(state)?;
    Ok(sim
        .widgets
        .get(window_id)
        .map(|window| (window.width, window.height))
        .unwrap_or_default())
}

fn update_window_size(
    state: &mut LuaState,
    window_id: u64,
    width: f32,
    height: f32,
) -> LuaResult<()> {
    let mut sim = borrow_state_mut(state)?;
    if let Some(window) = sim.widgets.get_mut_visual(window_id) {
        window.set_size(width, height);
        window.width_is_text_auto = false;
        window.height_is_text_auto = false;
    }
    sim.widgets.mark_rect_dirty(window_id);
    Ok(())
}

fn is_topmost(state: &mut LuaState) -> LuaResult<u32> {
    let window_id = frame_id_from_stack(state, 1)?;
    let fields = get_or_create_frame_fields(state, window_id);
    let topmost = bool_field(state, fields, TOPMOST_FIELD);
    state.push(Val::Bool(topmost));
    Ok(1)
}

fn set_topmost(state: &mut LuaState) -> LuaResult<u32> {
    set_bool_field(state, TOPMOST_FIELD)
}

fn set_bool_field(state: &mut LuaState, field: &'static str) -> LuaResult<u32> {
    let window_id = frame_id_from_stack(state, 1)?;
    let value = bool::from_stack(state, 2)?;
    let fields = get_or_create_frame_fields(state, window_id);
    table_set_static(state, fields, field, Val::Bool(value));
    Ok(0)
}

fn close(state: &mut LuaState) -> LuaResult<u32> {
    crate::lua_api::frame::methods::core_state::hide(state)
}

fn number_field(state: &mut LuaState, fields: Val, field: &'static str) -> f32 {
    match table_get(state, fields, field) {
        Val::Num(value) => value as f32,
        _ => 0.0,
    }
}

fn bool_field(state: &mut LuaState, fields: Val, field: &'static str) -> bool {
    matches!(table_get(state, fields, field), Val::Bool(true))
}
