//! Focused retail 12.0.0 status-bar fill style enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_status_bar_fill_style_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Standard = 0,
                    StandardNoRangeFill = 1,
                    Center = 2,
                    Reverse = 3,
                }
                local fill_style = Enum.StatusBarFillStyle
                if type(fill_style) ~= "table" then
                    return "StatusBarFillStyle: expected table"
                end

                local count = 0
                for name, value in pairs(fill_style) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 4 then
                    return "member count: expected 4, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if fill_style[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.StatusBarFillStyleMeta
                if type(meta) ~= "table" then
                    return "StatusBarFillStyleMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 3
                    or meta.NumValues ~= 4 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 status-bar fill style enum mismatch: {result}"
    );
}
