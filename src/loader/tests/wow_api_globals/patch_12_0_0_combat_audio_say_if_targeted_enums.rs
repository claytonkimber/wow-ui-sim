//! Focused retail 12.0.0 combat-audio targeted-state enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_combat_audio_say_if_targeted_enum_values() {
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

                local error_message = check_values("CombatAudioAlertSayIfTargetedType", {
                    None = 0,
                    Aggro = 1,
                    Targeted = 2,
                    TargetedBy = 3,
                })
                if error_message then
                    return error_message
                end

                error_message = check_values("CombatAudioAlertSayIfTargetedTypeMeta", {
                    MinValue = 0,
                    MaxValue = 3,
                    NumValues = 4,
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
        "retail 12.0.0 combat-audio targeted-state enum mismatch: {result}"
    );
}
