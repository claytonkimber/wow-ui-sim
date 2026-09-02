//! Focused retail 12.0.0 spell-diminish category enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_spell_diminish_category_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Root = 0,
                    Taunt = 1,
                    Stun = 2,
                    AoEKnockback = 3,
                    Incapacitate = 4,
                    Disorient = 5,
                    Silence = 6,
                    Disarm = 7,
                }
                local category = Enum.SpellDiminishCategory
                if type(category) ~= "table" then
                    return "SpellDiminishCategory: expected table"
                end

                local count = 0
                for name, value in pairs(category) do
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
                    if category[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.SpellDiminishCategoryMeta
                if type(meta) ~= "table" then
                    return "SpellDiminishCategoryMeta: expected table"
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
        "retail 12.0.0 spell-diminish category enum mismatch: {result}"
    );
}
