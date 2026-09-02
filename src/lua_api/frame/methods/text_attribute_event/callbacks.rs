//! CallbackRegistryMixin equivalents: RegisterCallback, UnregisterCallback, TriggerEvent.

use crate::lua_api::methods::{
    call_function_state, create_table, extract_frame_id, frame_id_from_stack, frame_ref,
    get_or_create_frame_fields, table_get, table_set, val_to_string,
};
use crate::lua_api::script_helpers::call_error_handler_state;
use crate::lua_bridge::stack_val;
use rilua::vm::gc::arena::GcRef;
use rilua::vm::state::LuaState;
use rilua::vm::table::Table;
use rilua::{LuaResult, Val, runtime_error};

pub(super) const FRAME_CALLBACKS_KEY: &str = "__callbacks";

/// Look up (and optionally create) the per-frame/per-event callback table.
pub(super) fn callback_event_table(
    state: &mut LuaState,
    frame_id: u64,
    event: &str,
    create: bool,
) -> LuaResult<Option<GcRef<Table>>> {
    let frame = frame_ref(state, frame_id)?;
    let Val::Table(frame_ref) = frame else {
        return Ok(None);
    };
    let callbacks = get_or_create_callbacks_table(state, frame_ref, create)?;
    let Some(callbacks) = callbacks else {
        return Ok(None);
    };
    Ok(get_or_create_event_table(state, callbacks, event, create))
}

fn get_or_create_callbacks_table(
    state: &mut LuaState,
    frame_ref: GcRef<Table>,
    create: bool,
) -> LuaResult<Option<GcRef<Table>>> {
    let callbacks_key = state
        .gc
        .intern_string_static(FRAME_CALLBACKS_KEY.as_bytes());
    match state
        .gc
        .tables
        .get(frame_ref)
        .map(|t| t.get_str(callbacks_key, &state.gc.string_arena))
        .unwrap_or(Val::Nil)
    {
        Val::Table(t) => Ok(Some(t)),
        _ if create => {
            let table_ref = state.gc.alloc_table(Table::new());
            if let Some(ft) = state.gc.tables.get_mut(frame_ref) {
                let _ = ft.raw_set(
                    Val::Str(callbacks_key),
                    Val::Table(table_ref),
                    &state.gc.string_arena,
                );
            }
            state.gc.barrier_back(frame_ref);
            Ok(Some(table_ref))
        }
        _ => Ok(None),
    }
}

fn get_or_create_event_table(
    state: &mut LuaState,
    callbacks: GcRef<Table>,
    event: &str,
    create: bool,
) -> Option<GcRef<Table>> {
    let event_key = state.gc.intern_string(event.as_bytes());
    match state
        .gc
        .tables
        .get(callbacks)
        .map(|t| t.get_str(event_key, &state.gc.string_arena))
        .unwrap_or(Val::Nil)
    {
        Val::Table(t) => Some(t),
        _ if create => {
            let table_ref = state.gc.alloc_table(Table::new());
            if let Some(ct) = state.gc.tables.get_mut(callbacks) {
                let _ = ct.raw_set(
                    Val::Str(event_key),
                    Val::Table(table_ref),
                    &state.gc.string_arena,
                );
            }
            state.gc.barrier_back(callbacks);
            Some(table_ref)
        }
        _ => None,
    }
}

pub(super) fn callback_entries(state: &LuaState, event_table: GcRef<Table>) -> Vec<Val> {
    state
        .gc
        .tables
        .get(event_table)
        .map(|table| table.array_slice().to_vec())
        .unwrap_or_default()
}

pub(super) fn callback_entry_fields(state: &mut LuaState, entry: Val) -> Option<(Val, Val)> {
    let Val::Table(entry_ref) = entry else {
        return None;
    };
    let owner_key = state.gc.intern_string_static(b"owner");
    let func_key = state.gc.intern_string_static(b"func");
    let table = state.gc.tables.get(entry_ref)?;
    Some((
        table.get_str(owner_key, &state.gc.string_arena),
        table.get_str(func_key, &state.gc.string_arena),
    ))
}

pub(super) fn rewrite_callback_entries(
    state: &mut LuaState,
    event_table: GcRef<Table>,
    entries: &[Val],
) {
    let old_len = state
        .gc
        .tables
        .get(event_table)
        .map(|t| t.array_slice().len())
        .unwrap_or(0);
    let new_len = entries.len();
    let clear_to = old_len.max(new_len);
    if let Some(table) = state.gc.tables.get_mut(event_table) {
        for (index, entry) in entries.iter().copied().enumerate() {
            let _ = table.raw_set(Val::Num((index + 1) as f64), entry, &state.gc.string_arena);
        }
        for index in new_len..clear_to {
            let _ = table.raw_set(
                Val::Num((index + 1) as f64),
                Val::Nil,
                &state.gc.string_arena,
            );
        }
    }
    state.gc.barrier_back(event_table);
}

pub(super) fn register_callback(state: &mut LuaState) -> LuaResult<u32> {
    let frame_id = frame_id_from_stack(state, 1)?;
    let event = val_to_string(state, stack_val(state, 2)).ok_or_else(|| {
        runtime_error("CallbackRegistryMixin:RegisterCallback 'event' requires string type.")
    })?;
    let func = stack_val(state, 3);
    if !matches!(func, Val::Function(_)) {
        return Err(runtime_error(
            "CallbackRegistryMixin:RegisterCallback 'func' requires function type.",
        ));
    }
    let owner = match stack_val(state, 4) {
        Val::Nil => func,
        owner => owner,
    };
    if let Some(event_table) = callback_event_table(state, frame_id, &event, true)? {
        let entries = dedup_entries_by_owner(state, event_table, owner);
        let entry_ref = build_callback_entry(state, owner, func, None);
        let mut new_entries = entries;
        new_entries.push(Val::Table(entry_ref));
        rewrite_callback_entries(state, event_table, &new_entries);
    }
    state.push(owner);
    Ok(1)
}

fn dedup_entries_by_owner(state: &mut LuaState, event_table: GcRef<Table>, owner: Val) -> Vec<Val> {
    callback_entries(state, event_table)
        .into_iter()
        .filter(|entry| {
            callback_entry_fields(state, *entry)
                .map(|(entry_owner, _)| entry_owner != owner)
                .unwrap_or(false)
        })
        .collect()
}

fn build_callback_entry(
    state: &mut LuaState,
    owner: Val,
    func: Val,
    unit_filter: Option<&str>,
) -> GcRef<Table> {
    let entry_ref = state.gc.alloc_table(Table::new());
    let owner_key = state.gc.intern_string_static(b"owner");
    let func_key = state.gc.intern_string_static(b"func");
    let unit_key = state.gc.intern_string_static(b"unit");
    let unit_val =
        unit_filter.map(|unit_filter| Val::Str(state.gc.intern_string(unit_filter.as_bytes())));
    if let Some(entry_table) = state.gc.tables.get_mut(entry_ref) {
        let _ = entry_table.raw_set(Val::Str(owner_key), owner, &state.gc.string_arena);
        let _ = entry_table.raw_set(Val::Str(func_key), func, &state.gc.string_arena);
        if let Some(unit_val) = unit_val {
            let _ = entry_table.raw_set(Val::Str(unit_key), unit_val, &state.gc.string_arena);
        }
    }
    state.gc.barrier_back(entry_ref);
    entry_ref
}

pub(crate) fn register_frame_event_callback(
    state: &mut LuaState,
    frame_id: u64,
    event: &str,
    func: Val,
) -> LuaResult<()> {
    let owner = frame_ref(state, frame_id)?;
    if let Some(event_table) = callback_event_table(state, frame_id, event, true)? {
        let entries = dedup_entries_by_owner(state, event_table, owner);
        let entry_ref = build_callback_entry(state, owner, func, None);
        let mut new_entries = entries;
        new_entries.push(Val::Table(entry_ref));
        rewrite_callback_entries(state, event_table, &new_entries);
    }
    Ok(())
}

pub(super) fn register_unit_callback(
    state: &mut LuaState,
    frame_id: u64,
    event: &str,
    func: Val,
    unit_filter: Option<&str>,
) -> LuaResult<()> {
    let owner = frame_ref(state, frame_id)?;
    if let Some(event_table) = callback_event_table(state, frame_id, event, true)? {
        let entries = dedup_entries_by_owner(state, event_table, owner);
        let entry_ref = build_callback_entry(state, owner, func, unit_filter);
        let mut new_entries = entries;
        new_entries.push(Val::Table(entry_ref));
        rewrite_callback_entries(state, event_table, &new_entries);
    }
    Ok(())
}

pub(super) fn unregister_callback(state: &mut LuaState) -> LuaResult<u32> {
    let frame_id = frame_id_from_stack(state, 1)?;
    let event = val_to_string(state, stack_val(state, 2)).ok_or_else(|| {
        runtime_error("CallbackRegistryMixin:UnregisterCallback 'event' requires string type.")
    })?;
    let owner = stack_val(state, 3);
    if matches!(owner, Val::Nil) {
        return Err(runtime_error(
            "CallbackRegistryMixin:UnregisterCallback 'owner' is required.",
        ));
    }
    if let Some(event_table) = callback_event_table(state, frame_id, &event, false)? {
        let entries = dedup_entries_by_owner(state, event_table, owner);
        rewrite_callback_entries(state, event_table, &entries);
    }
    Ok(0)
}

pub(super) fn add_static_event_method(state: &mut LuaState) -> LuaResult<u32> {
    let owner = stack_val(state, 1);
    let registry = stack_val(state, 2);
    let event = stack_val(state, 3);
    let handler = stack_val(state, 4);

    let Val::Table(registry_table) = registry else {
        return Ok(0);
    };
    let register_callback = table_get(state, Val::Table(registry_table), "RegisterCallback");
    if !matches!(register_callback, Val::Function(_)) {
        return Ok(0);
    }

    let _ = call_function_state(state, register_callback, &[registry, event, handler, owner])?;
    Ok(0)
}

pub(super) fn trigger_callback_event(state: &mut LuaState) -> LuaResult<u32> {
    let frame_id = frame_id_from_stack(state, 1)?;
    let event = val_to_string(state, stack_val(state, 2)).ok_or_else(|| {
        runtime_error("CallbackRegistryMixin:TriggerEvent 'event' requires string type.")
    })?;
    let args = collect_trigger_args(state);
    let callbacks = callback_event_table(state, frame_id, &event, false)?
        .map(|event_table| callback_entries(state, event_table))
        .unwrap_or_default();
    dispatch_callbacks(state, &callbacks, &args);
    Ok(0)
}

pub(crate) fn trigger_callback_event_for_frame(
    state: &mut LuaState,
    frame_id: u64,
    event: &str,
    args: &[Val],
) {
    let callbacks = callback_event_table(state, frame_id, event, false)
        .ok()
        .flatten()
        .map(|event_table| callback_entries(state, event_table))
        .unwrap_or_default();
    dispatch_callbacks(state, &callbacks, args);
}

pub(super) fn setup_menu(_state: &mut LuaState) -> LuaResult<u32> {
    let state = _state;
    let frame = stack_val(state, 1);
    let Some(id) = extract_frame_id(state, frame) else {
        return Ok(0);
    };
    let fields = get_or_create_frame_fields(state, id);
    let override_fn = table_get(state, fields, "SetupMenu");
    if matches!(override_fn, Val::Function(_)) {
        let arg_count = state.top.saturating_sub(state.base) as i32;
        let args: Vec<Val> = (1..=arg_count)
            .map(|index| stack_val(state, index))
            .collect();
        let _ = call_function_state(state, override_fn, &args)?;
        return Ok(0);
    }
    let generator = stack_val(state, 2);
    table_set(state, fields, "menuGenerator", generator);
    Ok(0)
}

fn frame_fields(state: &mut LuaState) -> LuaResult<(u64, Val)> {
    let frame_id = frame_id_from_stack(state, 1)?;
    Ok((frame_id, get_or_create_frame_fields(state, frame_id)))
}

fn current_selections_or_empty_table(state: &mut LuaState, index: i32) -> Val {
    match stack_val(state, index) {
        Val::Nil => create_table(state),
        value => value,
    }
}

fn set_callback_field(state: &mut LuaState, key: &str, value_index: i32) -> LuaResult<u32> {
    let (_, fields) = frame_fields(state)?;
    table_set(state, fields, key, stack_val(state, value_index));
    Ok(0)
}

fn set_callback_field_or_delegate(
    state: &mut LuaState,
    method_name: &str,
    key: &str,
    value_index: i32,
) -> LuaResult<u32> {
    let (_, fields) = frame_fields(state)?;
    let override_fn = table_get(state, fields, method_name);
    if matches!(override_fn, Val::Function(_)) {
        let arg_count = state.top.saturating_sub(state.base) as i32;
        let args: Vec<Val> = (1..=arg_count)
            .map(|index| stack_val(state, index))
            .collect();
        let _ = call_function_state(state, override_fn, &args)?;
        return Ok(0);
    }
    table_set(state, fields, key, stack_val(state, value_index));
    Ok(0)
}

pub(super) fn set_default_text(state: &mut LuaState) -> LuaResult<u32> {
    set_callback_field_or_delegate(state, "SetDefaultText", "defaultText", 2)
}

pub(super) fn set_selection_translator(state: &mut LuaState) -> LuaResult<u32> {
    set_callback_field_or_delegate(state, "SetSelectionTranslator", "selectionTranslator", 2)
}

pub(super) fn set_selection_text(state: &mut LuaState) -> LuaResult<u32> {
    set_callback_field(state, "selectionFunc", 2)
}

pub(super) fn enable_regenerate_on_response(state: &mut LuaState) -> LuaResult<u32> {
    let (_, fields) = frame_fields(state)?;
    table_set(state, fields, "shouldRegenerateOnResponse", Val::Bool(true));
    Ok(0)
}

pub(super) fn get_selection_text(state: &mut LuaState) -> LuaResult<u32> {
    let (_, fields) = frame_fields(state)?;
    let selection_func = table_get(state, fields, "selectionFunc");
    let default_text = table_get(state, fields, "defaultText");
    let result = if matches!(selection_func, Val::Function(_)) {
        let selections = create_table(state);
        let text = call_function_state(state, selection_func, &[selections])?;
        if matches!(text, Val::Nil) {
            default_text
        } else {
            text
        }
    } else {
        default_text
    };
    state.push(result);
    Ok(1)
}

pub(super) fn update_to_menu_selections(state: &mut LuaState) -> LuaResult<u32> {
    let (frame_id, fields) = frame_fields(state)?;
    let current_selections = current_selections_or_empty_table(state, 3);
    let selection_func = table_get(state, fields, "selectionFunc");
    let default_text = table_get(state, fields, "defaultText");
    let text = if matches!(selection_func, Val::Function(_)) {
        let text = call_function_state(state, selection_func, &[current_selections])?;
        if matches!(text, Val::Nil) {
            default_text
        } else {
            text
        }
    } else {
        default_text
    };
    if !matches!(text, Val::Nil) {
        let self_ref = frame_ref(state, frame_id)?;
        let set_text = table_get(state, self_ref, "SetText");
        if matches!(set_text, Val::Function(_)) {
            let _ = call_function_state(state, set_text, &[self_ref, text])?;
        }
    }
    Ok(0)
}

pub(super) fn set_default_callback(state: &mut LuaState) -> LuaResult<u32> {
    set_callback_field(state, "defaultCallback", 2)
}

pub(super) fn set_is_default_callback(state: &mut LuaState) -> LuaResult<u32> {
    set_callback_field(state, "isDefaultCallback", 2)
}

pub(super) fn set_update_callback(state: &mut LuaState) -> LuaResult<u32> {
    set_callback_field(state, "notifyUpdateCallback", 2)
}

pub(super) fn notify_update(state: &mut LuaState) -> LuaResult<u32> {
    let (_, fields) = frame_fields(state)?;
    let callback = table_get(state, fields, "notifyUpdateCallback");
    if matches!(callback, Val::Function(_)) {
        let self_ref = stack_val(state, 1);
        let description = stack_val(state, 2);
        let _ = call_function_state(state, callback, &[self_ref, description])?;
    }
    Ok(0)
}

pub(super) fn set_on_click_handler(state: &mut LuaState) -> LuaResult<u32> {
    let (_, fields) = frame_fields(state)?;
    table_set(state, fields, "onClickHandler", stack_val(state, 2));
    table_set(state, fields, "onClickSoundKit", stack_val(state, 3));
    Ok(0)
}

pub(super) fn set_on_enter_handler(state: &mut LuaState) -> LuaResult<u32> {
    set_callback_field(state, "onEnterHandler", 2)
}

fn collect_trigger_args(state: &LuaState) -> Vec<Val> {
    let arg_count = state.top.saturating_sub(state.base) as i32;
    if arg_count >= 3 {
        (3..=arg_count).map(|idx| stack_val(state, idx)).collect()
    } else {
        Vec::new()
    }
}

fn dispatch_callbacks(state: &mut LuaState, callbacks: &[Val], args: &[Val]) {
    for &entry in callbacks {
        let Some((owner, func)) = callback_entry_fields(state, entry) else {
            continue;
        };
        if matches!(func, Val::Nil) {
            continue;
        }
        let mut call_args = Vec::with_capacity(args.len() + 1);
        call_args.push(owner);
        call_args.extend_from_slice(args);
        if let Err(error) = call_function_state(state, func, &call_args) {
            call_error_handler_state(state, &error.to_string());
        }
    }
}

pub(crate) fn dispatch_unit_event_callbacks(
    state: &mut LuaState,
    frame_id: u64,
    event: &str,
    args: &[Val],
) {
    let callbacks = callback_event_table(state, frame_id, event, false)
        .ok()
        .flatten()
        .map(|event_table| callback_entries(state, event_table))
        .unwrap_or_default();
    for entry in callbacks {
        let Some((owner, func)) = callback_entry_fields(state, entry) else {
            continue;
        };
        let unit_filter = callback_entry_unit_filter(state, entry);
        if unit_filter.is_some()
            && !unit_filter_matches(state, unit_filter.as_deref(), args.first().copied())
        {
            continue;
        }
        let mut call_args = Vec::with_capacity(args.len() + 1);
        call_args.push(owner);
        call_args.extend_from_slice(args);
        if let Err(error) = call_function_state(state, func, &call_args) {
            call_error_handler_state(state, &error.to_string());
        }
    }
}

fn callback_entry_unit_filter(state: &mut LuaState, entry: Val) -> Option<String> {
    let Val::Table(entry_ref) = entry else {
        return None;
    };
    let unit_key = state.gc.intern_string_static(b"unit");
    let table = state.gc.tables.get(entry_ref)?;
    match table.get_str(unit_key, &state.gc.string_arena) {
        Val::Str(unit_ref) => state
            .gc
            .string_arena
            .get(unit_ref)
            .and_then(|unit| std::str::from_utf8(unit.data()).ok())
            .map(str::to_owned),
        _ => None,
    }
}

fn unit_filter_matches(state: &mut LuaState, unit_filter: Option<&str>, arg: Option<Val>) -> bool {
    let Some(unit_filter) = unit_filter else {
        return true;
    };
    match arg {
        Some(unit_arg) => val_to_string(state, unit_arg).as_deref() == Some(unit_filter),
        None => false,
    }
}
