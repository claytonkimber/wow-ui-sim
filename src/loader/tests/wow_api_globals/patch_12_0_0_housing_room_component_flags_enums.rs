//! Focused retail 12.0.0 housing room component flag enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_housing_room_component_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    None = 0,
                    HiddenInLayoutMode = 1,
                }
                local flags = Enum.HousingRoomComponentFlags
                if type(flags) ~= "table" then
                    return "HousingRoomComponentFlags: expected table"
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
                if count ~= 2 then
                    return "member count: expected 2, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if flags[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.HousingRoomComponentFlagsMeta
                if type(meta) ~= "table" then
                    return "HousingRoomComponentFlagsMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 1
                    or meta.NumValues ~= 2 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 housing room component flags enum mismatch: {result}"
    );
}
