//! Focused retail 12.0.0 raid-dispel display type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_raid_dispel_display_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Disabled = 0,
                    DispellableByMe = 1,
                    DisplayAll = 2,
                }
                local display = Enum.RaidDispelDisplayType
                if type(display) ~= "table" then
                    return "RaidDispelDisplayType: expected table"
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
                if count ~= 3 then
                    return "member count: expected 3, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if display[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.RaidDispelDisplayTypeMeta
                if type(meta) ~= "table" then
                    return "RaidDispelDisplayTypeMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 2
                    or meta.NumValues ~= 3 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 raid dispel display type enum mismatch: {result}"
    );
}
