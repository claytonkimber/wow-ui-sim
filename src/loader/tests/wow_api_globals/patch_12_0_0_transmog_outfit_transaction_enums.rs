//! Focused retail 12.0.0 transmog outfit transaction enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_transmog_outfit_transaction_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local flags = Enum.TransmogOutfitTransactionFlags
                if type(flags) ~= "table" then
                    return "TransmogOutfitTransactionFlags: expected table"
                end
                local flag_count = 0
                for _, value in pairs(flags) do
                    flag_count = flag_count + 1
                    if type(value) ~= "number" then
                        return "TransmogOutfitTransactionFlags: expected numeric members"
                    end
                end
                if flag_count ~= 10 then
                    return "flag member count: expected 10, got " .. tostring(flag_count)
                end
                local flag_meta = Enum.TransmogOutfitTransactionFlagsMeta
                if type(flag_meta) ~= "table"
                    or flag_meta.MinValue ~= 1
                    or flag_meta.MaxValue ~= 28
                    or flag_meta.NumValues ~= 10 then
                    return "flag metadata mismatch"
                end

                local expected_type = {
                    UpdateMetadata = 0,
                    UpdateOutfitInfo = 1,
                    CreateOutfitInfo = 2,
                    UpdateSlots = 3,
                    UpdateSituations = 4,
                }
                local transaction_type = Enum.TransmogOutfitTransactionType
                if type(transaction_type) ~= "table" then
                    return "TransmogOutfitTransactionType: expected table"
                end
                local type_count = 0
                for name, value in pairs(transaction_type) do
                    type_count = type_count + 1
                    local expected_value = expected_type[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if type_count ~= 5 then
                    return "type member count: expected 5, got " .. tostring(type_count)
                end
                for name, expected_value in pairs(expected_type) do
                    if transaction_type[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end
                local type_meta = Enum.TransmogOutfitTransactionTypeMeta
                if type(type_meta) ~= "table"
                    or type_meta.MinValue ~= 0
                    or type_meta.MaxValue ~= 4
                    or type_meta.NumValues ~= 5 then
                    return "type metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 transmog outfit transaction enum mismatch: {result}"
    );
}
