//! Focused retail 12.0.0 nameplate cast-bar display enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_nameplate_cast_bar_display_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    None = 0,
                    SpellName = 1,
                    SpellIcon = 2,
                    SpellTarget = 3,
                    HighlightImportantCasts = 4,
                    HighlightWhenCastTarget = 5,
                }
                local display = Enum.NamePlateCastBarDisplay
                if type(display) ~= "table" then
                    return "NamePlateCastBarDisplay: expected table"
                end

                local count = 0
                for name, value in pairs(display) do
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
                    if display[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.NamePlateCastBarDisplayMeta
                if type(meta) ~= "table" then
                    return "NamePlateCastBarDisplayMeta: expected table"
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
        "retail 12.0.0 nameplate cast-bar display enum mismatch: {result}"
    );
}
