//! Focused retail 12.0.0 encounter-events visibility enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_encounter_events_visibility_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Always = 0,
                    InEncounter = 1,
                    DeprecatedHidden = 2,
                }
                local visibility = Enum.EncounterEventsVisibility
                if type(visibility) ~= "table" then
                    return "EncounterEventsVisibility: expected table"
                end

                local count = 0
                for name, value in pairs(visibility) do
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
                    if visibility[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.EncounterEventsVisibilityMeta
                if type(meta) ~= "table" then
                    return "EncounterEventsVisibilityMeta: expected table"
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
        "retail 12.0.0 encounter-events visibility enum mismatch: {result}"
    );
}
