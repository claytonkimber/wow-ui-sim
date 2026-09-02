//! Focused retail 12.0.0 encounter-event cast-state enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_encounter_event_cast_state_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Casting = 1,
                    NotCasting = 2,
                    Expired = 3,
                }
                local state = Enum.EncounterEventCastState
                if type(state) ~= "table" then
                    return "EncounterEventCastState: expected table"
                end

                local count = 0
                for name, value in pairs(state) do
                    count = count + 1
                    local expected_value = expected[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 3 then
                    return "member count: expected 3, got " .. tostring(count)
                end

                local meta = Enum.EncounterEventCastStateMeta
                if type(meta) ~= "table" then
                    return "EncounterEventCastStateMeta: expected table"
                end
                if meta.MinValue ~= 1 or meta.MaxValue ~= 3 or meta.NumValues ~= 3 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 encounter-event cast-state enum mismatch: {result}"
    );
}
