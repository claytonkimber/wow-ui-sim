//! Focused retail 12.0.0 encounter-timeline sort-direction enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_encounter_timeline_event_sort_direction_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Descending = 0,
                    Ascending = 1,
                }
                local direction = Enum.EncounterTimelineEventSortDirection
                if type(direction) ~= "table" then
                    return "EncounterTimelineEventSortDirection: expected table"
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
                if count ~= 2 then
                    return "member count: expected 2, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if direction[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.EncounterTimelineEventSortDirectionMeta
                if type(meta) ~= "table" then
                    return "EncounterTimelineEventSortDirectionMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 1 or meta.NumValues ~= 2 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 encounter-timeline sort-direction enum mismatch: {result}"
    );
}
