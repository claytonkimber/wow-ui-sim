//! Focused retail 12.0.0 nameplate threat-display enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_nameplate_threat_display_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    None = 0,
                    Progressive = 1,
                    Flash = 2,
                    HealthBarColor = 3,
                }
                local display = Enum.NamePlateThreatDisplay
                if type(display) ~= "table" then
                    return "NamePlateThreatDisplay: expected table"
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
                if count ~= 4 then
                    return "member count: expected 4, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if display[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.NamePlateThreatDisplayMeta
                if type(meta) ~= "table" then
                    return "NamePlateThreatDisplayMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 3 or meta.NumValues ~= 4 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 nameplate threat-display enum mismatch: {result}"
    );
}
