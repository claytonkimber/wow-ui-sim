//! Focused retail 12.0.0 combat-audio throttle enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_combat_audio_throttle_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Sample = 0,
                    PlayerHealth = 1,
                    TargetHealth = 2,
                    PlayerCast = 3,
                    TargetCast = 4,
                    PlayerResource1 = 5,
                    PlayerResource2 = 6,
                }
                local throttle = Enum.CombatAudioAlertThrottle
                if type(throttle) ~= "table" then
                    return "CombatAudioAlertThrottle: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = throttle[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local absent = {
                    "PlayerHealthSamePercent",
                    "TargetHealthSamePercent",
                    "PlayerResource1SamePercent",
                    "PlayerResource2SamePercent",
                }
                for index = 1, #absent do
                    local name = absent[index]
                    if throttle[name] ~= nil then
                        return name .. ": expected nil"
                    end
                end

                local meta = Enum.CombatAudioAlertThrottleMeta
                if type(meta) ~= "table" then
                    return "CombatAudioAlertThrottleMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 6 or meta.NumValues ~= 7 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 combat-audio throttle enum mismatch: {result}"
    );
}
