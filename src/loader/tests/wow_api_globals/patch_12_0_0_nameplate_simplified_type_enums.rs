//! Focused retail 12.0.0 nameplate simplified-type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_nameplate_simplified_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    None = 0,
                    Minion = 1,
                    MinusMob = 2,
                    FriendlyPlayer = 3,
                    FriendlyNpc = 4,
                }
                local simplified_type = Enum.NamePlateSimplifiedType
                if type(simplified_type) ~= "table" then
                    return "NamePlateSimplifiedType: expected table"
                end

                local count = 0
                for name, value in pairs(simplified_type) do
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
                    if simplified_type[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.NamePlateSimplifiedTypeMeta
                if type(meta) ~= "table" then
                    return "NamePlateSimplifiedTypeMeta: expected table"
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
        "retail 12.0.0 nameplate simplified-type enum mismatch: {result}"
    );
}
