//! Focused retail 12.0.0 dungeon-encounter trigger-type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_dungeon_encounter_trigger_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Invalid = 0,
                    OnStart = 1,
                    OnComplete = 2,
                    OnEnd = 3,
                    PreviouslyCompleted = 4,
                }
                local trigger_type = Enum.DungeonEncounterTriggerType
                if type(trigger_type) ~= "table" then
                    return "DungeonEncounterTriggerType: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = trigger_type[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local meta = Enum.DungeonEncounterTriggerTypeMeta
                if type(meta) ~= "table" then
                    return "DungeonEncounterTriggerTypeMeta: expected table"
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
        "retail 12.0.0 dungeon-encounter trigger-type enum mismatch: {result}"
    );
}
