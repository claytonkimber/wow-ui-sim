//! Focused retail 12.0.0 HousingCatalogEntrySubtype enum removal.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_housing_catalog_entry_subtype_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Invalid = 0,
                    Unowned = 1,
                    OwnedModifiedStack = 2,
                    OwnedUnmodifiedStack = 3,
                }
                local subtype = Enum.HousingCatalogEntrySubtype
                if type(subtype) ~= "table" then
                    return "HousingCatalogEntrySubtype: expected table"
                end

                local count = 0
                for name, value in pairs(subtype) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 4 then
                    return "member count: expected 4, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if subtype[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end
                if rawget(subtype, "MarketItem") ~= nil then
                    return "MarketItem: removed member is present"
                end

                local expected_meta = {
                    MinValue = 0,
                    MaxValue = 3,
                    NumValues = 4,
                }
                local meta = Enum.HousingCatalogEntrySubtypeMeta
                if type(meta) ~= "table" then
                    return "HousingCatalogEntrySubtypeMeta: expected table"
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
        "retail 12.0.0 HousingCatalogEntrySubtype enum mismatch: {result}"
    );
}
