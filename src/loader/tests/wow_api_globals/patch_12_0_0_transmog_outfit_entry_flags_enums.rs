//! Focused retail 12.0.0 transmog outfit entry flag enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_transmog_outfit_entry_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    AutomaticallyAwardedOnLogin = 1,
                    UseOverrideName = 2,
                    OnlyAvailableDuringEvent = 4,
                    SortedToTopOfList = 8,
                    UseOverrideCostModifier = 16,
                }
                local flags = Enum.TransmogOutfitEntryFlags
                if type(flags) ~= "table" then
                    return "TransmogOutfitEntryFlags: expected table"
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
                if count ~= 5 then
                    return "member count: expected 5, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if flags[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.TransmogOutfitEntryFlagsMeta
                if type(meta) ~= "table" then
                    return "TransmogOutfitEntryFlagsMeta: expected table"
                end
                if meta.MinValue ~= 1 or meta.MaxValue ~= 16 or meta.NumValues ~= 5 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 transmog outfit entry flags enum mismatch: {result}"
    );
}
