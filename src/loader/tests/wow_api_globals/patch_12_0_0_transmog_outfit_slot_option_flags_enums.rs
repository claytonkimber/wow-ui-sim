//! Focused retail 12.0.0 transmog outfit slot-option flag enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_transmog_outfit_slot_option_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    IllusionNotAllowed = 1,
                    DynamicOptionName = 2,
                    DisablesOffhandSlot = 4,
                }
                local flags = Enum.TransmogOutfitSlotOptionFlags
                if type(flags) ~= "table" then
                    return "TransmogOutfitSlotOptionFlags: expected table"
                end

                local count = 0
                for name, value in pairs(flags) do
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
                    if flags[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.TransmogOutfitSlotOptionFlagsMeta
                if type(meta) ~= "table" then
                    return "TransmogOutfitSlotOptionFlagsMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 1
                    or meta.MaxValue ~= 4
                    or meta.NumValues ~= 3 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 transmog outfit slot-option flags enum mismatch: {result}"
    );
}
