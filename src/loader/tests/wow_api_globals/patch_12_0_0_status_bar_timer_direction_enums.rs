//! Focused retail 12.0.0 status-bar timer direction enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_status_bar_timer_direction_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    ElapsedTime = 0,
                    RemainingTime = 1,
                }
                local direction = Enum.StatusBarTimerDirection
                if type(direction) ~= "table" then
                    return "StatusBarTimerDirection: expected table"
                end

                local count = 0
                for name, value in pairs(direction) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 2 then
                    return "member count: expected 2, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if direction[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.StatusBarTimerDirectionMeta
                if type(meta) ~= "table" then
                    return "StatusBarTimerDirectionMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 1 or meta.NumValues ~= 2 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 status-bar timer direction enum mismatch: {result}"
    );
}
