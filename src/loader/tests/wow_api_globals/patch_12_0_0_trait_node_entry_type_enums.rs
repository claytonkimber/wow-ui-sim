//! Focused retail 12.0.0 TraitNodeEntryType enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_trait_node_entry_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    SpendHex = 0,
                    SpendSquare = 1,
                    SpendCircle = 2,
                    SpendSmallCircle = 3,
                    DeprecatedSelect = 4,
                    DragAndDrop = 5,
                    SpendDiamond = 6,
                    ProfPath = 7,
                    ProfPerk = 8,
                    ProfPathUnlock = 9,
                    RedButton = 10,
                    ArmorSet = 11,
                    SpendInfinite = 12,
                    SpendCapstoneCircle = 13,
                    SpendCapstoneSquare = 14,
                }
                local entry_type = Enum.TraitNodeEntryType
                if type(entry_type) ~= "table" then
                    return "TraitNodeEntryType: expected table"
                end

                local count = 0
                for name, value in pairs(entry_type) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 15 then
                    return "member count: expected 15, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if entry_type[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.TraitNodeEntryTypeMeta
                if type(meta) ~= "table" then
                    return "TraitNodeEntryTypeMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 14
                    or meta.NumValues ~= 15 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 trait node entry type enum mismatch: {result}"
    );
}
