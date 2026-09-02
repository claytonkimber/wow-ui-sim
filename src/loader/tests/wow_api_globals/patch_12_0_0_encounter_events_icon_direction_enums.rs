//! Focused retail 12.0.0 encounter-events icon-direction enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_encounter_events_icon_direction_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Top = 0,
                    Bottom = 1,
                    Left = 0,
                    Right = 1,
                }
                local direction = Enum.EncounterEventsIconDirection
                if type(direction) ~= "table" then
                    return "EncounterEventsIconDirection: expected table"
                end

                local count = 0
                for name, value in pairs(direction) do
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
                    if direction[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.EncounterEventsIconDirectionMeta
                if type(meta) ~= "table" then
                    return "EncounterEventsIconDirectionMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 1 or meta.NumValues ~= 4 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 encounter-events icon-direction enum mismatch: {result}"
    );
}
