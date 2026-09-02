//! Focused retail 12.0.0 neighborhood initiative neighborhood-type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_neighborhood_initiative_neighborhood_types_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    NiNeighborhoodTypeSingleton = 0,
                    NiNeighborhoodTypePool = 1,
                }
                local neighborhood_types = Enum.NeighborhoodInitiativeNeighborhoodTypes
                if type(neighborhood_types) ~= "table" then
                    return "NeighborhoodInitiativeNeighborhoodTypes: expected table"
                end

                local count = 0
                for name, value in pairs(neighborhood_types) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 2 then
                    return "member count: expected 2, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if neighborhood_types[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.NeighborhoodInitiativeNeighborhoodTypesMeta
                if type(meta) ~= "table" then
                    return "NeighborhoodInitiativeNeighborhoodTypesMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 1 or meta.NumValues ~= 2 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 neighborhood initiative neighborhood-types enum mismatch: {result}"
    );
}
