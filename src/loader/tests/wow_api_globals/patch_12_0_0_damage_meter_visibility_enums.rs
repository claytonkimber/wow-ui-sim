//! Focused retail 12.0.0 damage-meter visibility enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_damage_meter_visibility_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Always = 0,
                    InCombat = 1,
                    Hidden = 2,
                }
                local visibility = Enum.DamageMeterVisibility
                if type(visibility) ~= "table" then
                    return "DamageMeterVisibility: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = visibility[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local meta = Enum.DamageMeterVisibilityMeta
                if type(meta) ~= "table" then
                    return "DamageMeterVisibilityMeta: expected table"
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
        "retail 12.0.0 damage-meter visibility enum mismatch: {result}"
    );
}
