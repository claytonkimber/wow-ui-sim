//! Focused retail 12.0.0 transmog outfit data flag enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_transmog_outfit_data_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    IsCachedLocally = 1,
                }
                local flags = Enum.TransmogOutfitDataFlags
                if type(flags) ~= "table" then
                    return "TransmogOutfitDataFlags: expected table"
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
                if count ~= 1 then
                    return "member count: expected 1, got " .. tostring(count)
                end
                if flags.IsCachedLocally ~= 1 then
                    return "IsCachedLocally: missing member"
                end

                local meta = Enum.TransmogOutfitDataFlagsMeta
                if type(meta) ~= "table" then
                    return "TransmogOutfitDataFlagsMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 1
                    or meta.MaxValue ~= 1
                    or meta.NumValues ~= 1 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 transmog outfit data flags enum mismatch: {result}"
    );
}
