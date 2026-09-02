//! Focused retail 12.0.0 Edit Mode damage-meter setting enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_edit_mode_damage_meter_settings_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Visibility = 0,
                    Style = 1,
                    Numbers = 2,
                    FrameWidth = 3,
                    FrameHeight = 4,
                    Padding = 5,
                    Transparency = 6,
                    ObsoleteReuse1 = 7,
                    ShowSpecIcon = 8,
                    ShowClassColor = 9,
                    BarHeight = 10,
                    TextSize = 11,
                    BackgroundTransparency = 12,
                }
                local setting = Enum.EditModeDamageMeterSetting
                if type(setting) ~= "table" then
                    return "EditModeDamageMeterSetting: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = setting[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end
                local meta = Enum.EditModeDamageMeterSettingMeta
                if type(meta) ~= "table" then
                    return "EditModeDamageMeterSettingMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 12 or meta.NumValues ~= 13 then
                    return "metadata mismatch"
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 Edit Mode damage-meter setting mismatch: {result}"
    );
}
