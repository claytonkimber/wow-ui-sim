//! Focused retail 12.0.0 dungeon-encounter creature flag enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_dungeon_encounter_creature_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    BossCreature = 1,
                    DropLootImmediately = 2,
                    DoNotDespawnOnSuccess = 4,
                }
                local flags = Enum.DungeonEncounterXCreatureFlags
                if type(flags) ~= "table" then
                    return "DungeonEncounterXCreatureFlags: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = flags[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local meta = Enum.DungeonEncounterXCreatureFlagsMeta
                if type(meta) ~= "table" then
                    return "DungeonEncounterXCreatureFlagsMeta: expected table"
                end
                if meta.MinValue ~= 1 or meta.MaxValue ~= 4 or meta.NumValues ~= 3 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 dungeon-encounter creature flag enum mismatch: {result}"
    );
}
