//! Focused retail 12.0.0 combat-audio spec-setting enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_combat_audio_spec_setting_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Resource1Percent = 0,
                    Resource1Format = 1,
                    Resource2Percent = 2,
                    Resource2Format = 3,
                    SayIfTargeted = 4,
                }
                if type(Enum.CombatAudioAlertSpecSetting) ~= "table" then
                    return "CombatAudioAlertSpecSetting: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = Enum.CombatAudioAlertSpecSetting[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local absent = {
                    "Resource1Voice",
                    "Resource1Volume",
                    "Resource2Voice",
                    "Resource2Volume",
                }
                for index = 1, #absent do
                    local name = absent[index]
                    if Enum.CombatAudioAlertSpecSetting[name] ~= nil then
                        return name .. ": expected nil"
                    end
                end

                local meta = Enum.CombatAudioAlertSpecSettingMeta
                if type(meta) ~= "table" then
                    return "CombatAudioAlertSpecSettingMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 4 or meta.NumValues ~= 5 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 combat-audio spec-setting enum mismatch: {result}"
    );
}
