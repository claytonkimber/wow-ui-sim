//! Focused retail 12.0.0 transmog outfit slot error metadata.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_transmog_outfit_slot_error_metadata_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Ok = 0,
                    NoItem = 1,
                    NotSoulbound = 2,
                    Legendary = 3,
                    InvalidItemType = 4,
                    InvalidDestination = 5,
                    Mismatch = 6,
                    SameItem = 7,
                    InvalidSource = 8,
                    InvalidSourceQuality = 9,
                    CannotUseItem = 10,
                    InvalidSlotForRace = 11,
                    NoIllusion = 12,
                    InvalidSlotForForm = 13,
                    IncompatibleWithMainHand = 14,
                }
                local errors = Enum.TransmogOutfitSlotError
                if type(errors) ~= "table" then
                    return "TransmogOutfitSlotError: expected table"
                end

                local count = 0
                for name, value in pairs(errors) do
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
                    if errors[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.TransmogOutfitSlotErrorMeta
                if type(meta) ~= "table" then
                    return "TransmogOutfitSlotErrorMeta: expected table"
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
        "retail 12.0.0 transmog outfit slot error metadata mismatch: {result}"
    );
}
