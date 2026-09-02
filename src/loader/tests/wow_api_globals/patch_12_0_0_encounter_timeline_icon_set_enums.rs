//! Focused retail 12.0.0 encounter-timeline icon-set enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_encounter_timeline_icon_set_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    TankAlert = 1,
                    HealerAlert = 2,
                    DamageAlert = 3,
                    Deadly = 4,
                    Dispel = 5,
                    Enrage = 6,
                }
                local icon_set = Enum.EncounterTimelineIconSet
                if type(icon_set) ~= "table" then
                    return "EncounterTimelineIconSet: expected table"
                end

                local count = 0
                for name, value in pairs(icon_set) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 6 then
                    return "member count: expected 6, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if icon_set[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.EncounterTimelineIconSetMeta
                if type(meta) ~= "table" then
                    return "EncounterTimelineIconSetMeta: expected table"
                end
                if meta.MinValue ~= 1 or meta.MaxValue ~= 6 or meta.NumValues ~= 6 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 encounter-timeline icon-set enum mismatch: {result}"
    );
}
