//! Focused retail 12.0.0 chat-lockdown and combat-audio enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_chat_and_combat_audio_enum_values() {
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

                local checks = {
                    ChatMessagingLockdownReason = {
                        ActiveEncounter = 0,
                        ActiveMythicKeystoneOrChallengeMode = 1,
                        ActivePvPMatch = 2,
                    },
                    ChatMessagingLockdownReasonMeta = {
                        MinValue = 0,
                        MaxValue = 2,
                        NumValues = 3,
                    },
                    CombatAudioAlertCastState = {
                        Off = 0,
                        OnCastStart = 1,
                        OnCastEnd = 2,
                    },
                    CombatAudioAlertCastStateMeta = {
                        MinValue = 0,
                        MaxValue = 2,
                        NumValues = 3,
                    },
                }

                for namespace_name, expected_values in pairs(checks) do
                    local error_message = check_values(namespace_name, expected_values)
                    if error_message then
                        return error_message
                    end
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 chat/combat-audio enum mismatch: {result}"
    );
}
