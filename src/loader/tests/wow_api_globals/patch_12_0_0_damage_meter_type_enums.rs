//! Focused retail 12.0.0 damage-meter type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_damage_meter_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    DamageDone = 0,
                    Dps = 1,
                    HealingDone = 2,
                    Hps = 3,
                    Absorbs = 4,
                    Interrupts = 5,
                    Dispels = 6,
                    DamageTaken = 7,
                    AvoidableDamageTaken = 8,
                }
                local meter_type = Enum.DamageMeterType
                if type(meter_type) ~= "table" then
                    return "DamageMeterType: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = meter_type[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end
                for _, name in ipairs({"Deaths", "EnemyDamageTaken"}) do
                    if meter_type[name] ~= nil then
                        return name .. ": expected nil"
                    end
                end

                local meta = Enum.DamageMeterTypeMeta
                if type(meta) ~= "table" then
                    return "DamageMeterTypeMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 8 or meta.NumValues ~= 9 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 damage-meter type enum mismatch: {result}"
    );
}
