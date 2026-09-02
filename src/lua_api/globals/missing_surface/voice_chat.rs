//! `C_VoiceChat` probe surface backed by `SimState.voice_chat`.
//!
//! Migrates 17 entries off the namespace stub tables:
//!
//! - `GetActiveChannelID()` — returns the active channel ID or nil.
//! - `GetActiveChannelType()` — returns the active channel type or nil.
//! - `GetChannel(channelID)` — returns a VoiceChatChannel table or nothing.
//! - `GetChannels()` — returns array of VoiceChatChannel tables.
//! - `GetCurrentVoiceChatConnectionStatusCode()` — returns status code or nil.
//! - `GetMasterVolumeScale()` — returns master volume scale.
//! - `GetMicrophoneVolume()` — returns microphone volume or nil.
//! - `GetOutputVolume()` — returns output volume or nil.
//! - `GetTtsVoices()` — returns an empty list until TTS voice catalogs exist.
//! - `GetNumActiveChannels()` — returns count of seeded channels.
//! - `GetNumMembers(channelID)` — returns member count for a channel.
//! - `IsDeafened()` — returns deafened bool or nil.
//! - `IsEnabled()` — returns enabled bool.
//! - `IsMuted()` — returns muted bool or nil.
//! - `IsParentalDisabled()` — returns parental-disabled bool.
//! - `IsTalking()` — returns talking bool or nil.
//! - `IsTranscriptionAllowed()` — returns false until transcription policy exists.

use super::ensure_namespace;
use crate::lua_api::methods::{borrow_state, create_table, table_set};
use crate::lua_bridge::{FromStack, table_set_rust_fn_static};
use rilua::vm::state::LuaState;
use rilua::{LuaResult, Val};

pub(super) fn register_voice_chat_surface(state: &mut LuaState) -> LuaResult<()> {
    let ns = ensure_namespace(state, "C_VoiceChat")?;
    table_set_rust_fn_static(state, ns, "GetActiveChannelID", get_active_channel_id)?;
    table_set_rust_fn_static(state, ns, "GetActiveChannelType", get_active_channel_type)?;
    table_set_rust_fn_static(state, ns, "GetChannel", get_channel)?;
    table_set_rust_fn_static(state, ns, "GetChannels", get_channels)?;
    table_set_rust_fn_static(
        state,
        ns,
        "GetCurrentVoiceChatConnectionStatusCode",
        get_current_voice_chat_connection_status_code,
    )?;
    table_set_rust_fn_static(state, ns, "GetMasterVolumeScale", get_master_volume_scale)?;
    table_set_rust_fn_static(state, ns, "GetMicrophoneVolume", get_microphone_volume)?;
    table_set_rust_fn_static(state, ns, "GetOutputVolume", get_output_volume)?;
    table_set_rust_fn_static(state, ns, "GetTtsVoices", get_tts_voices)?;
    table_set_rust_fn_static(state, ns, "GetNumActiveChannels", get_num_active_channels)?;
    table_set_rust_fn_static(state, ns, "GetNumMembers", get_num_members)?;
    table_set_rust_fn_static(state, ns, "IsDeafened", is_deafened)?;
    table_set_rust_fn_static(state, ns, "IsEnabled", is_enabled)?;
    table_set_rust_fn_static(state, ns, "IsMuted", is_muted)?;
    table_set_rust_fn_static(state, ns, "IsParentalDisabled", is_parental_disabled)?;
    table_set_rust_fn_static(state, ns, "IsTalking", is_talking)?;
    table_set_rust_fn_static(
        state,
        ns,
        "IsTranscriptionAllowed",
        is_transcription_allowed,
    )?;
    Ok(())
}

fn build_channel_table(state: &mut LuaState, channel_id: i32) -> Option<Val> {
    let vc = borrow_state(state).ok()?.voice_chat.clone();
    let ch = vc.channels.iter().find(|c| c.channel_id == channel_id)?;
    let tbl = create_table(state);
    table_set(state, tbl, "channelID", Val::Num(ch.channel_id as f64));
    table_set(state, tbl, "channelType", Val::Num(ch.channel_type as f64));
    let name_val = {
        let s = state.gc.intern_string(ch.name.as_bytes());
        Val::Str(s)
    };
    table_set(state, tbl, "channelName", name_val);
    table_set(state, tbl, "numMembers", Val::Num(ch.members.len() as f64));
    Some(tbl)
}

fn get_active_channel_id(state: &mut LuaState) -> LuaResult<u32> {
    let id = borrow_state(state)?.voice_chat.active_channel_id;
    match id {
        Some(n) => state.push(Val::Num(n as f64)),
        None => state.push(Val::Nil),
    }
    Ok(1)
}

fn get_active_channel_type(state: &mut LuaState) -> LuaResult<u32> {
    let channel_type = {
        let sim = borrow_state(state)?;
        let active_channel_id = sim.voice_chat.active_channel_id;
        active_channel_id.and_then(|channel_id| {
            sim.voice_chat
                .channels
                .iter()
                .find(|channel| channel.channel_id == channel_id)
                .map(|channel| channel.channel_type)
        })
    };

    match channel_type {
        Some(channel_type) => state.push(Val::Num(channel_type as f64)),
        None => state.push(Val::Nil),
    }
    Ok(1)
}

fn get_channel(state: &mut LuaState) -> LuaResult<u32> {
    let channel_id = i32::from_stack(state, 1)?;
    match build_channel_table(state, channel_id) {
        Some(tbl) => {
            state.push(tbl);
            Ok(1)
        }
        None => Ok(0),
    }
}

fn get_channels(state: &mut LuaState) -> LuaResult<u32> {
    let ids: Vec<i32> = borrow_state(state)?
        .voice_chat
        .channels
        .iter()
        .map(|c| c.channel_id)
        .collect();
    let array = create_table(state);
    for (i, id) in ids.into_iter().enumerate() {
        if let Some(tbl) = build_channel_table(state, id) {
            if let Val::Table(arr_ref) = array {
                if let Some(t) = state.gc.tables.get_mut(arr_ref) {
                    let _ = t.raw_set(Val::Num(i as f64 + 1.0), tbl, &state.gc.string_arena);
                }
                state.gc.barrier_back(arr_ref);
            }
        }
    }
    state.push(array);
    Ok(1)
}

fn get_current_voice_chat_connection_status_code(state: &mut LuaState) -> LuaResult<u32> {
    let code = borrow_state(state)?.voice_chat.connection_status;
    state.push(Val::Num(code as f64));
    Ok(1)
}

fn get_master_volume_scale(state: &mut LuaState) -> LuaResult<u32> {
    let scale = borrow_state(state)?.voice_chat.master_volume_scale;
    state.push(Val::Num(scale));
    Ok(1)
}

fn get_microphone_volume(state: &mut LuaState) -> LuaResult<u32> {
    let vol = borrow_state(state)?.voice_chat.microphone_volume;
    state.push(Val::Num(vol as f64));
    Ok(1)
}

fn get_output_volume(state: &mut LuaState) -> LuaResult<u32> {
    let vol = borrow_state(state)?.voice_chat.output_volume;
    state.push(Val::Num(vol as f64));
    Ok(1)
}

fn get_tts_voices(state: &mut LuaState) -> LuaResult<u32> {
    let voices = create_table(state);
    state.push(voices);
    Ok(1)
}

fn get_num_active_channels(state: &mut LuaState) -> LuaResult<u32> {
    let count = borrow_state(state)?.voice_chat.channels.len();
    state.push(Val::Num(count as f64));
    Ok(1)
}

fn get_num_members(state: &mut LuaState) -> LuaResult<u32> {
    let channel_id = i32::from_stack(state, 1)?;
    let count = borrow_state(state)?
        .voice_chat
        .channels
        .iter()
        .find(|c| c.channel_id == channel_id)
        .map(|c| c.members.len())
        .unwrap_or(0);
    state.push(Val::Num(count as f64));
    Ok(1)
}

fn is_deafened(state: &mut LuaState) -> LuaResult<u32> {
    let v = borrow_state(state)?.voice_chat.deafened;
    state.push(Val::Bool(v));
    Ok(1)
}

fn is_enabled(state: &mut LuaState) -> LuaResult<u32> {
    let v = borrow_state(state)?.voice_chat.enabled;
    state.push(Val::Bool(v));
    Ok(1)
}

fn is_muted(state: &mut LuaState) -> LuaResult<u32> {
    let v = borrow_state(state)?.voice_chat.muted;
    state.push(Val::Bool(v));
    Ok(1)
}

fn is_parental_disabled(state: &mut LuaState) -> LuaResult<u32> {
    let v = borrow_state(state)?.voice_chat.is_parental_disabled;
    state.push(Val::Bool(v));
    Ok(1)
}

fn is_talking(state: &mut LuaState) -> LuaResult<u32> {
    let v = borrow_state(state)?.voice_chat.talking;
    state.push(Val::Bool(v));
    Ok(1)
}

fn is_transcription_allowed(state: &mut LuaState) -> LuaResult<u32> {
    state.push(Val::Bool(false));
    Ok(1)
}
