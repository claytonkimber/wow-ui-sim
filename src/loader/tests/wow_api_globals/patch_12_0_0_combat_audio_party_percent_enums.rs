//! Focused retail 12.0.0 combat-audio party-percent enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_combat_audio_party_percent_enum_values() {
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

                local error_message = check_values("CombatAudioAlertPartyPercentValues", {
                    Off = 0,
                    Under100Percent = 1,
                    Under90Percent = 2,
                    Under80Percent = 3,
                    Under70Percent = 4,
                    Under60Percent = 5,
                    Under50Percent = 6,
                    Under40Percent = 7,
                    Under30Percent = 8,
                    Under20Percent = 9,
                    Under10Percent = 10,
                })
                if error_message then
                    return error_message
                end

                error_message = check_values("CombatAudioAlertPartyPercentValuesMeta", {
                    MinValue = 0,
                    MaxValue = 10,
                    NumValues = 11,
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
        "retail 12.0.0 combat-audio party-percent enum mismatch: {result}"
    );
}
