//! Focused retail 12.0.0 ExpansionLevel enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_expansion_level_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    None = 0,
                    BurningCrusade = 1,
                    Northrend = 2,
                    Cataclysm = 3,
                    MistsOfPandaria = 4,
                    Draenor = 5,
                    Legion = 6,
                    BattleForAzeroth = 7,
                    Shadowlands = 8,
                    Dragonflight = 9,
                    WarWithin = 10,
                    Midnight = 11,
                    LastTitan = 12,
                }
                local expansion = Enum.ExpansionLevel
                if type(expansion) ~= "table" then
                    return "ExpansionLevel: expected table"
                end

                local count = 0
                for name, value in pairs(expansion) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 13 then
                    return "member count: expected 13, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if expansion[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local expected_meta = {
                    MinValue = 0,
                    MaxValue = 12,
                    NumValues = 13,
                }
                local meta = Enum.ExpansionLevelMeta
                if type(meta) ~= "table" then
                    return "ExpansionLevelMeta: expected table"
                end

                local meta_count = 0
                for name, value in pairs(meta) do
                    meta_count = meta_count + 1
                    local expected_value = expected_meta[name]
                    if expected_value == nil then
                        return "metadata." .. name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return "metadata." .. name .. ": unexpected value " .. tostring(value)
                    end
                end
                if meta_count ~= 3 then
                    return "metadata member count: expected 3, got " .. tostring(meta_count)
                end
                for name, expected_value in pairs(expected_meta) do
                    if meta[name] ~= expected_value then
                        return "metadata." .. name .. ": missing member"
                    end
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 ExpansionLevel enum mismatch: {result}"
    );
}
