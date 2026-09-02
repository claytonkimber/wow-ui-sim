//! Focused retail 12.0.0 neighborhood initiative chest-result enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_neighborhood_initiative_chest_result_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    NiSuccess = 0,
                    NiUnspecifiedFailure = 1,
                    NiNoHouseFound = 2,
                    NiNoRewards = 3,
                    NiThrottled = 4,
                    NiServiceDisabled = 5,
                }
                local result = Enum.NeighborhoodInitiativeChestResult
                if type(result) ~= "table" then
                    return "NeighborhoodInitiativeChestResult: expected table"
                end

                local count = 0
                for name, value in pairs(result) do
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
                    if result[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.NeighborhoodInitiativeChestResultMeta
                if type(meta) ~= "table" then
                    return "NeighborhoodInitiativeChestResultMeta: expected table"
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
        "retail 12.0.0 neighborhood initiative chest-result enum mismatch: {result}"
    );
}
