//! Focused retail 12.0.0 damage-meter storage-type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_damage_meter_storage_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Damage = 0,
                    HealingAndAbsorbs = 1,
                    Absorbs = 2,
                    Interrupts = 3,
                    Dispels = 4,
                    DamageTaken = 5,
                    AvoidableDamageTaken = 6,
                }
                local storage_type = Enum.DamageMeterStorageType
                if type(storage_type) ~= "table" then
                    return "DamageMeterStorageType: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = storage_type[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end
                for _, name in ipairs({"Deaths", "EnemyDamageTaken"}) do
                    if storage_type[name] ~= nil then
                        return name .. ": expected nil"
                    end
                end

                local meta = Enum.DamageMeterStorageTypeMeta
                if type(meta) ~= "table" then
                    return "DamageMeterStorageTypeMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 6 or meta.NumValues ~= 7 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 damage-meter storage-type enum mismatch: {result}"
    );
}
