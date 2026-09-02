//! Generic dispatch payloads for newly registered housing events.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;
use rilua::Val;

#[test]
fn test_patch_12_0_0_housing_event_dispatch_payloads() {
    let env = WowLuaEnv::new().unwrap();
    let setup_result: String = env
        .eval(
            r##"
                local event_names = {
                    "HOUSE_LEVEL_CHANGED",
                    "HOUSING_BASIC_MODE_PLACEMENT_FLAGS_UPDATED",
                    "HOUSING_BASIC_MODE_SELECTED_TARGET_CHANGED",
                    "HOUSING_DECOR_PLACE_SUCCESS",
                }
                local probe = { received = {} }
                local frame = CreateFrame("Frame")

                for _, event_name in ipairs(event_names) do
                    local ok, error_message = pcall(function()
                        frame:RegisterEvent(event_name)
                    end)
                    if not ok then
                        return "register:" .. event_name .. ":" .. tostring(error_message)
                    end
                end

                frame:SetScript("OnEvent", function(_, event_name, ...)
                    probe.received[event_name] = {
                        event_name = event_name,
                        count = select("#", ...),
                        values = { ... },
                    }
                end)
                _G.__housing_event_probe = probe
                return "ok"
            "##,
        )
        .unwrap();
    assert_eq!(setup_result, "ok", "housing event registration failed");

    let house_level_payload: Val = env.eval(r#"return { houseLevel = 7 }"#).unwrap();
    env.fire_event_with_args("HOUSE_LEVEL_CHANGED", &[house_level_payload])
        .unwrap();
    env.fire_event_with_args(
        "HOUSING_BASIC_MODE_PLACEMENT_FLAGS_UPDATED",
        &[Val::Num(3.0), Val::Num(5.0)],
    )
    .unwrap();
    env.fire_event_with_args(
        "HOUSING_BASIC_MODE_SELECTED_TARGET_CHANGED",
        &[Val::Bool(true), Val::Num(42.0), Val::Bool(false)],
    )
    .unwrap();
    env.fire_event_with_args(
        "HOUSING_DECOR_PLACE_SUCCESS",
        &[
            env.lua_string("HousingDecor-123"),
            Val::Num(64.0),
            Val::Bool(true),
            Val::Bool(false),
        ],
    )
    .unwrap();

    let result: String = env
        .eval(
            r##"
                local probe = _G.__housing_event_probe
                local received = function(event_name)
                    local event = probe.received[event_name]
                    if event == nil then
                        return nil, event_name .. ":missing"
                    end
                    if event.event_name ~= event_name then
                        return nil, event_name .. ":name"
                    end
                    return event.values, event.count, nil
                end

                local values, count, error_message = received("HOUSE_LEVEL_CHANGED")
                if error_message then return error_message end
                if count ~= 1 or #values ~= 1 or type(values[1]) ~= "table" or values[1].houseLevel ~= 7 then
                    return "HOUSE_LEVEL_CHANGED:payload"
                end

                values, count, error_message = received("HOUSING_BASIC_MODE_PLACEMENT_FLAGS_UPDATED")
                if error_message then return error_message end
                if count ~= 2 or #values ~= 2 or type(values[1]) ~= "number" or values[1] ~= 3
                    or type(values[2]) ~= "number" or values[2] ~= 5 then
                    return "HOUSING_BASIC_MODE_PLACEMENT_FLAGS_UPDATED:payload"
                end

                values, count, error_message = received("HOUSING_BASIC_MODE_SELECTED_TARGET_CHANGED")
                if error_message then return error_message end
                if count ~= 3 or #values ~= 3 or type(values[1]) ~= "boolean" or values[1] ~= true
                    or type(values[2]) ~= "number" or values[2] ~= 42
                    or type(values[3]) ~= "boolean" or values[3] ~= false then
                    return "HOUSING_BASIC_MODE_SELECTED_TARGET_CHANGED:payload"
                end

                values, count, error_message = received("HOUSING_DECOR_PLACE_SUCCESS")
                if error_message then return error_message end
                if count ~= 4 or #values ~= 4 or type(values[1]) ~= "string" or values[1] ~= "HousingDecor-123"
                    or type(values[2]) ~= "number" or values[2] ~= 64
                    or type(values[3]) ~= "boolean" or values[3] ~= true
                    or type(values[4]) ~= "boolean" or values[4] ~= false then
                    return "HOUSING_DECOR_PLACE_SUCCESS:payload"
                end

                return "ok"
            "##,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "housing event dispatch payload mismatch: {result}"
    );
}

#[test]
fn test_patch_12_0_0_removed_housing_catalog_searcher_event_rejected() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local frame = CreateFrame("Frame")
                local removed_ok = pcall(function()
                    frame:RegisterEvent("HOUSING_CATALOG_SEARCHER_RELEASED")
                end)
                local valid_ok = pcall(function()
                    frame:RegisterEvent("HOUSE_LEVEL_CHANGED")
                end)
                return tostring(removed_ok) .. ":" .. tostring(valid_ok)
            "#,
        )
        .unwrap();

    assert_eq!(
        result, "false:true",
        "removed housing event registration result mismatch: {result}"
    );
}

#[test]
fn test_patch_12_0_0_removed_show_delves_display_event_rejected() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local frame = CreateFrame("Frame")
                local removed_ok = pcall(function()
                    frame:RegisterEvent("SHOW_DELVES_DISPLAY_UI")
                end)
                local valid_ok = pcall(function()
                    frame:RegisterEvent("HOUSE_LEVEL_CHANGED")
                end)
                return tostring(removed_ok) .. ":" .. tostring(valid_ok)
            "#,
        )
        .unwrap();

    assert_eq!(
        result, "false:true",
        "removed SHOW_DELVES_DISPLAY_UI registration result mismatch: {result}"
    );
}
