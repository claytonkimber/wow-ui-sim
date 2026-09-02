//! Focused retail 12.0.0 nameplate style enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_nameplate_style_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Modern = 0,
                    Thin = 1,
                    Block = 2,
                    HealthFocus = 3,
                    CastFocus = 4,
                    Legacy = 5,
                }
                local style = Enum.NamePlateStyle
                if type(style) ~= "table" then
                    return "NamePlateStyle: expected table"
                end

                local count = 0
                for name, value in pairs(style) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 6 then
                    return "member count: expected 6, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if style[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.NamePlateStyleMeta
                if type(meta) ~= "table" then
                    return "NamePlateStyleMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 5 or meta.NumValues ~= 6 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 nameplate style enum mismatch: {result}"
    );
}
