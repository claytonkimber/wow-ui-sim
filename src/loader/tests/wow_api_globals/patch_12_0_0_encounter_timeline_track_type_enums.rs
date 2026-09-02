//! Focused retail 12.0.0 encounter-timeline track-type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_encounter_timeline_track_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Hidden = 0,
                    Sorted = 1,
                    Linear = 2,
                }
                local track_type = Enum.EncounterTimelineTrackType
                if type(track_type) ~= "table" then
                    return "EncounterTimelineTrackType: expected table"
                end

                local count = 0
                for name, value in pairs(track_type) do
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
                    if track_type[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.EncounterTimelineTrackTypeMeta
                if type(meta) ~= "table" then
                    return "EncounterTimelineTrackTypeMeta: expected table"
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
        "retail 12.0.0 encounter-timeline track-type enum mismatch: {result}"
    );
}
