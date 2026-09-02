//! Focused retail 12.0.0 personal-resource display setting enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_personal_resource_display_setting_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    HideHealthAndPower = 0,
                    OnlyShowInCombat = 1,
                }
                local setting = Enum.EditModePersonalResourceDisplaySetting
                if type(setting) ~= "table" then
                    return "EditModePersonalResourceDisplaySetting: expected table"
                end

                local count = 0
                for name, value in pairs(setting) do
                    count = count + 1
                    local expected_value = expected[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 2 then
                    return "member count: expected 2, got " .. tostring(count)
                end

                local meta = Enum.EditModePersonalResourceDisplaySettingMeta
                if type(meta) ~= "table" then
                    return "EditModePersonalResourceDisplaySettingMeta: expected table"
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
        "retail 12.0.0 personal-resource display setting mismatch: {result}"
    );
}
