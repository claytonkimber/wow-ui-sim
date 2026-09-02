//! Focused retail 12.0.0 sleeves geo range enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_sleeves_geo_range_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    None = 0,
                    Default = 1,
                    Flared = 2,
                    Puffy = 3,
                    PandaCollar = 4,
                }
                local range = Enum.SleevesGeoRange
                if type(range) ~= "table" then
                    return "SleevesGeoRange: expected table"
                end

                local count = 0
                for name, value in pairs(range) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 5 then
                    return "member count: expected 5, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if range[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.SleevesGeoRangeMeta
                if type(meta) ~= "table" then
                    return "SleevesGeoRangeMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 4
                    or meta.NumValues ~= 5 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 sleeves geo range enum mismatch: {result}"
    );
}
