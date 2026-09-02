//! Focused retail 12.0.0 transmog outfit entry source enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_transmog_outfit_entry_source_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    StampedSource = 0,
                    AutomaticallyAwarded = 1,
                    PlayerPurchased = 2,
                }
                local source = Enum.TransmogOutfitEntrySource
                if type(source) ~= "table" then
                    return "TransmogOutfitEntrySource: expected table"
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
                if count ~= 3 then
                    return "member count: expected 3, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if source[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.TransmogOutfitEntrySourceMeta
                if type(meta) ~= "table" then
                    return "TransmogOutfitEntrySourceMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 2
                    or meta.NumValues ~= 3 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 transmog outfit entry source enum mismatch: {result}"
    );
}
