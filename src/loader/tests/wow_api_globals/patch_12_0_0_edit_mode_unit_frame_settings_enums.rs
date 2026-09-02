//! Focused retail 12.0.0 Edit Mode unit-frame setting enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_edit_mode_unit_frame_setting_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local setting = Enum.EditModeUnitFrameSetting
                if type(setting) ~= "table" then
                    return "EditModeUnitFrameSetting: expected table"
                end

                local expected = {
                    AuraOrganizationType = 18,
                    IconSize = 19,
                    Opacity = 20,
                }
                for name, expected_value in pairs(expected) do
                    local value = setting[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end
                if setting.BigDefensiveIconSize ~= nil then
                    return "BigDefensiveIconSize: expected nil, got "
                        .. tostring(setting.BigDefensiveIconSize)
                end

                local meta = Enum.EditModeUnitFrameSettingMeta
                if type(meta) ~= "table" then
                    return "EditModeUnitFrameSettingMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 20 or meta.NumValues ~= 21 then
                    return "metadata mismatch"
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 Edit Mode unit-frame setting enum mismatch: {result}"
    );
}
