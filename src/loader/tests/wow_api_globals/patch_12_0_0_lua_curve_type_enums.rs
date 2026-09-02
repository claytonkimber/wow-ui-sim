//! Focused retail 12.0.0 Lua curve type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_lua_curve_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Linear = 0,
                    Step = 1,
                    Cosine = 2,
                    Cubic = 3,
                }
                local curve_type = Enum.LuaCurveType
                if type(curve_type) ~= "table" then
                    return "LuaCurveType: expected table"
                end

                local count = 0
                for name, value in pairs(curve_type) do
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
                    if curve_type[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.LuaCurveTypeMeta
                if type(meta) ~= "table" then
                    return "LuaCurveTypeMeta: expected table"
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
        "retail 12.0.0 Lua curve type enum mismatch: {result}"
    );
}
