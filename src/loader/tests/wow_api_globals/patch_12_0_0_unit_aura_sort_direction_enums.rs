//! Focused retail 12.0.0 UnitAuraSortDirection enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_unit_aura_sort_direction_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {Normal = 0, Reverse = 1}
                local direction = Enum.UnitAuraSortDirection
                if type(direction) ~= "table" then
                    return "UnitAuraSortDirection: expected table"
                end
                local count = 0
                for name, value in pairs(direction) do
                    count = count + 1
                    if expected[name] == nil or type(value) ~= "number" or value ~= expected[name] then
                        return name .. ": unexpected member"
                    end
                end
                if count ~= 2 or direction.Normal ~= 0 or direction.Reverse ~= 1 then
                    return "member mismatch"
                end
                local meta = Enum.UnitAuraSortDirectionMeta
                if type(meta) ~= "table"
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
        "retail 12.0.0 UnitAuraSortDirection mismatch: {result}"
    );
}
