//! Decimal escaping for non-printable and invalid UTF-8 bytes.

use crate::c_api::ensure_namespace;
use crate::lua_api::methods::create_string_bytes;
use crate::lua_bridge::{stack_val, table_set_rust_fn_static};
use rilua::vm::state::LuaState;
use rilua::{LuaResult, Val};

pub(crate) fn register_escape_decimal_non_printables(state: &mut LuaState) -> LuaResult<()> {
    let namespace = ensure_namespace(state, "C_StringUtil")?;
    table_set_rust_fn_static(
        state,
        namespace,
        "EscapeDecimalNonPrintables",
        escape_decimal_non_printables,
    )
}

fn escape_decimal_non_printables(state: &mut LuaState) -> LuaResult<u32> {
    let Val::Str(input_ref) = stack_val(state, 1) else {
        state.push(Val::Nil);
        return Ok(1);
    };
    let Some(input) = state
        .gc
        .string_arena
        .get(input_ref)
        .map(|input| input.data().to_vec())
    else {
        state.push(Val::Nil);
        return Ok(1);
    };

    let escaped = escape_bytes(&input);
    let escaped_value = create_string_bytes(state, &escaped);
    state.push(escaped_value);
    Ok(1)
}

fn escape_bytes(input: &[u8]) -> Vec<u8> {
    let mut escaped = Vec::with_capacity(input.len());
    let mut remaining = input;

    while !remaining.is_empty() {
        match std::str::from_utf8(remaining) {
            Ok(valid) => {
                append_valid_utf8(&mut escaped, valid.as_bytes());
                break;
            }
            Err(error) => {
                let valid_prefix_len = error.valid_up_to();
                append_valid_utf8(&mut escaped, &remaining[..valid_prefix_len]);
                remaining = &remaining[valid_prefix_len..];

                let invalid_byte_count = error.error_len().unwrap_or(remaining.len());
                for &byte in &remaining[..invalid_byte_count] {
                    append_decimal_escape(&mut escaped, byte);
                }
                remaining = &remaining[invalid_byte_count..];
            }
        }
    }

    escaped
}

fn append_valid_utf8(output: &mut Vec<u8>, input: &[u8]) {
    for &byte in input {
        if byte.is_ascii_control() && !matches!(byte, b'\t' | b'\n' | b'\r') {
            append_decimal_escape(output, byte);
        } else {
            output.push(byte);
        }
    }
}

fn append_decimal_escape(output: &mut Vec<u8>, byte: u8) {
    output.push(b'\\');
    output.push(b'0' + byte / 100);
    output.push(b'0' + (byte / 10) % 10);
    output.push(b'0' + byte % 10);
}
