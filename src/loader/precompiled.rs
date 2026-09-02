//! Precompiled rilua loader helpers.

use crate::lua_api::methods::{call_function_state, registry_get, registry_set};
use rilua::vm::state::LuaState;
use rilua::{LuaApiMut, LuaResult, Val, runtime_error};

const FIRE_ONLOAD_KEY: &str = "__precompiled_fire_onload";
const FIRE_ONSHOW_KEY: &str = "__precompiled_fire_onshow";
const REPORT_SCRIPT_ERROR_KEY: &str = "__report_script_error";

const FIRE_ONLOAD_SOURCE: &str = r#"
    local __report, frame = ...
    if not frame then return end
    local oldSelf = rawget(_G, "self")
    rawset(_G, "self", frame)
    local intrinsic = __wow_bind_xml_method(frame, "OnLoad_Intrinsic")
    if type(intrinsic) == "function" then
        local ok, err = pcall(intrinsic, frame)
        if not ok then
            __report("[OnLoad_Intrinsic] " .. tostring(err))
        end
    end
    for _, bindingType in ipairs({0, 1, 2}) do
        local handler = frame:GetScript("OnLoad", bindingType)
        if handler then
            local ok, err = pcall(handler, frame)
            if not ok then
                local name = frame.GetName and frame:GetName() or "?"
                local stack = debugstack and debugstack() or ""
                __report("[OnLoad] " .. name .. ": " .. tostring(err) .. (stack ~= "" and ("\n" .. stack) or ""))
            end
        end
    end
    rawset(_G, "self", oldSelf)
"#;

const FIRE_ONSHOW_SOURCE: &str = r#"
    local __report, frame = ...
    if not frame or not frame:IsVisible() then return end
    local oldSelf = rawget(_G, "self")
    rawset(_G, "self", frame)
    for _, bindingType in ipairs({0, 1, 2}) do
        local handler = frame:GetScript("OnShow", bindingType)
        if handler then
            local ok, err = pcall(handler, frame)
            if not ok then
                local name = frame.GetName and frame:GetName() or "?"
                __report("[OnShow] " .. name .. ": " .. tostring(err))
            end
        end
    end
    if type(frame.OnShow_Intrinsic) == "function" then
        local ok, err = pcall(frame.OnShow_Intrinsic, frame)
        if not ok then
            __report("[OnShow_Intrinsic] " .. tostring(err))
        end
    end
    rawset(_G, "self", oldSelf)
"#;

pub fn init(lua: &mut rilua::Lua) -> crate::Result<()> {
    let state = lua.state_mut();
    store_precompiled(state, FIRE_ONLOAD_KEY, FIRE_ONLOAD_SOURCE)?;
    store_precompiled(state, FIRE_ONSHOW_KEY, FIRE_ONSHOW_SOURCE)?;
    Ok(())
}

pub fn fire_onload(state: &mut LuaState, frame: Val) -> LuaResult<()> {
    call_precompiled(state, FIRE_ONLOAD_KEY, frame)
}

pub fn fire_onshow(state: &mut LuaState, frame: Val) -> LuaResult<()> {
    call_precompiled(state, FIRE_ONSHOW_KEY, frame)
}

fn store_precompiled(state: &mut LuaState, key: &'static str, source: &str) -> LuaResult<()> {
    let func = LuaApiMut::load(state, source)?;
    registry_set(state, key, Val::Function(func.gc_ref()));
    Ok(())
}

fn call_precompiled(state: &mut LuaState, key: &'static str, frame: Val) -> LuaResult<()> {
    let func = registry_get(state, key);
    if !matches!(func, Val::Function(_)) {
        return Err(runtime_error(format!("missing precompiled helper {key}")));
    }
    let reporter = registry_get(state, REPORT_SCRIPT_ERROR_KEY);
    if !matches!(reporter, Val::Function(_)) {
        return Err(runtime_error(format!(
            "missing precompiled helper reporter {REPORT_SCRIPT_ERROR_KEY}"
        )));
    }
    call_function_state(state, func, &[reporter, frame]).map(|_| ())
}
