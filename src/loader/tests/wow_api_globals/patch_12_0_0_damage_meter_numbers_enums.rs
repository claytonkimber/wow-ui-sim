//! Focused retail 12.0.0 damage-meter number enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_damage_meter_numbers_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Minimal = 0,
                    Compact = 1,
                    Complete = 2,
                }
                local numbers = Enum.DamageMeterNumbers
                if type(numbers) ~= "table" then
                    return "DamageMeterNumbers: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = numbers[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local meta = Enum.DamageMeterNumbersMeta
                if type(meta) ~= "table" then
                    return "DamageMeterNumbersMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 2 or meta.NumValues ~= 3 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 damage-meter number enum mismatch: {result}"
    );
}
