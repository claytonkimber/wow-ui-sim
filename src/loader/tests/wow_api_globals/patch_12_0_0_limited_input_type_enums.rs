//! Focused retail 12.0.0 limited input type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_limited_input_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    MouseMove = 0,
                    MouseDown = 1,
                    MouseUp = 2,
                    MouseWheel = 3,
                }
                local input_type = Enum.LimitedInputType
                if type(input_type) ~= "table" then
                    return "LimitedInputType: expected table"
                end

                local count = 0
                for name, value in pairs(input_type) do
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
                    if input_type[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.LimitedInputTypeMeta
                if type(meta) ~= "table" then
                    return "LimitedInputTypeMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 3
                    or meta.NumValues ~= 4 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 limited input type enum mismatch: {result}"
    );
}
