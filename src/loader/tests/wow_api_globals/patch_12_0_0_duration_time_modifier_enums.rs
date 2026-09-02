//! Focused retail 12.0.0 duration-time modifier enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_duration_time_modifier_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    RealTime = 0,
                    BaseTime = 1,
                }
                local modifier = Enum.DurationTimeModifier
                if type(modifier) ~= "table" then
                    return "DurationTimeModifier: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = modifier[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local meta = Enum.DurationTimeModifierMeta
                if type(meta) ~= "table" then
                    return "DurationTimeModifierMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 1 or meta.NumValues ~= 2 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 duration-time modifier enum mismatch: {result}"
    );
}
