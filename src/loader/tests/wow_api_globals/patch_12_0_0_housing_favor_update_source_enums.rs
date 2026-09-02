//! Focused retail 12.0.0 housing favor update source enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_housing_favor_update_source_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Unknown = 0,
                    DecorCollection = 1,
                    DeferredRewards = 2,
                    RetroactiveDecor = 3,
                    NewHouseDecorFavor = 4,
                    InitiativeTask = 5,
                    InitiativeChest = 6,
                    Quest = 7,
                }
                local source = Enum.HousingFavorUpdateSource
                if type(source) ~= "table" then
                    return "HousingFavorUpdateSource: expected table"
                end

                local count = 0
                for name, value in pairs(source) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 8 then
                    return "member count: expected 8, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if source[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.HousingFavorUpdateSourceMeta
                if type(meta) ~= "table" then
                    return "HousingFavorUpdateSourceMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 7
                    or meta.NumValues ~= 8 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 housing favor update source enum mismatch: {result}"
    );
}
