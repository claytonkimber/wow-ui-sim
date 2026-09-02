//! Focused retail 12.0.0 simple order status enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_simple_order_status_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Invalid = 0,
                    Creating = 1,
                    InProgress = 2,
                    Success = 3,
                    Failed = 4,
                }
                local status = Enum.SimpleOrderStatus
                if type(status) ~= "table" then
                    return "SimpleOrderStatus: expected table"
                end

                local count = 0
                for name, value in pairs(status) do
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
                    if status[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.SimpleOrderStatusMeta
                if type(meta) ~= "table" then
                    return "SimpleOrderStatusMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 4
                    or meta.NumValues ~= 5 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 simple order status enum mismatch: {result}"
    );
}
