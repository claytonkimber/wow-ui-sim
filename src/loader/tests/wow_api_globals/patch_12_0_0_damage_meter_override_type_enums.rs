//! Focused retail 12.0.0 damage-meter override-type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_damage_meter_override_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Ignore = 0,
                    AllowFriendlyFire = 1,
                    RedirectSourceToOwner = 2,
                    RedirectSourceToAuraCaster = 3,
                    IgnoreForAbsorbSpell = 4,
                }
                local override_type = Enum.DamageMeterOverrideType
                if type(override_type) ~= "table" then
                    return "DamageMeterOverrideType: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = override_type[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local meta = Enum.DamageMeterOverrideTypeMeta
                if type(meta) ~= "table" then
                    return "DamageMeterOverrideTypeMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 4 or meta.NumValues ~= 5 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 damage-meter override-type enum mismatch: {result}"
    );
}
