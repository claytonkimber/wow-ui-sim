//! Focused retail 12.0.0 Edit Mode account/aura setting enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_edit_mode_account_aura_settings_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    ["Enum.EditModeAccountSetting"] = {
                        ShowPersonalResourceDisplay = 29,
                        ShowEncounterEvents = 30,
                        ShowDamageMeter = 31,
                        ShowExternalDefensives = 32,
                    },
                    ["Enum.EditModeAuraFrameSetting"] = {
                        VisibleSetting = 8,
                        Opacity = 9,
                        ShowDispelType = 10,
                    },
                }
                for enum_name, members in pairs(expected) do
                    local enum = Enum[enum_name:match("Enum%.(.+)")]
                    if type(enum) ~= "table" then
                        return enum_name .. ": expected table"
                    end
                    for name, expected_value in pairs(members) do
                        local value = enum[name]
                        if type(value) ~= "number" or value ~= expected_value then
                            return enum_name .. "." .. name .. ": expected "
                                .. tostring(expected_value) .. ", got " .. tostring(value)
                        end
                    end
                end

                if Enum.EditModeAccountSetting.ShowTotemActionBar ~= nil then
                    return "Enum.EditModeAccountSetting.ShowTotemActionBar: expected nil"
                end
                local account_meta = Enum.EditModeAccountSettingMeta
                if type(account_meta) ~= "table" then
                    return "Enum.EditModeAccountSettingMeta: expected table"
                end
                if account_meta.MinValue ~= 0
                    or account_meta.MaxValue ~= 32
                    or account_meta.NumValues ~= 33
                then
                    return "Enum.EditModeAccountSettingMeta: metadata mismatch"
                end

                local aura_meta = Enum.EditModeAuraFrameSettingMeta
                if type(aura_meta) ~= "table" then
                    return "Enum.EditModeAuraFrameSettingMeta: expected table"
                end
                if aura_meta.MinValue ~= 0
                    or aura_meta.MaxValue ~= 10
                    or aura_meta.NumValues ~= 11
                then
                    return "Enum.EditModeAuraFrameSettingMeta: metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 Edit Mode account/aura enum mismatch: {result}"
    );
}
