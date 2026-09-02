//! Focused retail 12.0.0 housing favor update type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_housing_favor_update_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Add = 0,
                    InitiativeAdd = 1,
                    Set = 2,
                }
                local update_type = Enum.HousingFavorUpdateType
                if type(update_type) ~= "table" then
                    return "HousingFavorUpdateType: expected table"
                end

                local count = 0
                for name, value in pairs(update_type) do
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
                    if update_type[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.HousingFavorUpdateTypeMeta
                if type(meta) ~= "table" then
                    return "HousingFavorUpdateTypeMeta: expected table"
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
        "retail 12.0.0 housing favor update type enum mismatch: {result}"
    );
}
