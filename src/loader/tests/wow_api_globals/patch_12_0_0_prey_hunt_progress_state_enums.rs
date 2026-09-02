//! Focused retail 12.0.0 prey-hunt progress-state enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_prey_hunt_progress_state_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Cold = 0,
                    Warm = 1,
                    Hot = 2,
                    Final = 3,
                }
                local state = Enum.PreyHuntProgressState
                if type(state) ~= "table" then
                    return "PreyHuntProgressState: expected table"
                end

                local count = 0
                for name, value in pairs(state) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 4 then
                    return "member count: expected 4, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if state[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.PreyHuntProgressStateMeta
                if type(meta) ~= "table" then
                    return "PreyHuntProgressStateMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 3
                    or meta.NumValues ~= 4 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 prey-hunt progress state enum mismatch: {result}"
    );
}
