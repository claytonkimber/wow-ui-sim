//! Focused retail 12.0.0 combat-log message-order enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_combat_log_message_order_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local function check_values(namespace_name, expected_values)
                    local namespace = Enum[namespace_name]
                    if type(namespace) ~= "table" then
                        return namespace_name .. ": expected table, got " .. type(namespace)
                    end

                    for value_name, expected_value in pairs(expected_values) do
                        local value = namespace[value_name]
                        if type(value) ~= "number" then
                            return namespace_name .. "." .. value_name
                                .. ": expected number, got " .. type(value)
                        end
                        if value ~= expected_value then
                            return namespace_name .. "." .. value_name
                                .. ": expected " .. tostring(expected_value)
                                .. ", got " .. tostring(value)
                        end
                    end
                end

                local error_message = check_values("CombatLogMessageOrder", {
                    Newest = 0,
                    Oldest = 1,
                })
                if error_message then
                    return error_message
                end

                error_message = check_values("CombatLogMessageOrderMeta", {
                    MinValue = 0,
                    MaxValue = 1,
                    NumValues = 2,
                })
                if error_message then
                    return error_message
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 combat-log message-order enum mismatch: {result}"
    );
}
