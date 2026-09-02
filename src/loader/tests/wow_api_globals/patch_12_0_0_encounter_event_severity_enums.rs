//! Focused retail 12.0.0 encounter-event severity enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_encounter_event_severity_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Low = 0,
                    Medium = 1,
                    High = 2,
                }
                local severity = Enum.EncounterEventSeverity
                if type(severity) ~= "table" then
                    return "EncounterEventSeverity: expected table"
                end

                local count = 0
                for name, value in pairs(severity) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 3 then
                    return "member count: expected 3, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if severity[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.EncounterEventSeverityMeta
                if type(meta) ~= "table" then
                    return "EncounterEventSeverityMeta: expected table"
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
        "retail 12.0.0 encounter-event severity enum mismatch: {result}"
    );
}
