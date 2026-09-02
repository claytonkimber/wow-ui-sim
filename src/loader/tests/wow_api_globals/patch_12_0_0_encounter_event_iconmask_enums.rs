//! Focused retail 12.0.0 encounter-event icon-mask enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_encounter_event_iconmask_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    DeadlyEffect = 1,
                    EnrageEffect = 2,
                    BleedEffect = 4,
                    MagicEffect = 8,
                    DiseaseEffect = 16,
                    CurseEffect = 32,
                    PoisonEffect = 64,
                    TankRole = 128,
                    HealerRole = 256,
                    DpsRole = 512,
                }
                local iconmask = Enum.EncounterEventIconmask
                if type(iconmask) ~= "table" then
                    return "EncounterEventIconmask: expected table"
                end

                local count = 0
                for name, value in pairs(iconmask) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 10 then
                    return "member count: expected 10, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if iconmask[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.EncounterEventIconmaskMeta
                if type(meta) ~= "table" then
                    return "EncounterEventIconmaskMeta: expected table"
                end
                if meta.MinValue ~= 1 or meta.MaxValue ~= 512 or meta.NumValues ~= 10 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 encounter-event iconmask enum mismatch: {result}"
    );
}
