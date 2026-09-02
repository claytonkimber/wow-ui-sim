//! Focused retail 12.0.0 encounter-timeline track enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_encounter_timeline_track_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Queued = 0,
                    Short = 1,
                    Medium = 2,
                    Long = 3,
                    Indeterminate = 4,
                }
                local track = Enum.EncounterTimelineTrack
                if type(track) ~= "table" then
                    return "EncounterTimelineTrack: expected table"
                end

                local count = 0
                for name, value in pairs(track) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 5 then
                    return "member count: expected 5, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if track[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.EncounterTimelineTrackMeta
                if type(meta) ~= "table" then
                    return "EncounterTimelineTrackMeta: expected table"
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
        "retail 12.0.0 encounter-timeline track enum mismatch: {result}"
    );
}
