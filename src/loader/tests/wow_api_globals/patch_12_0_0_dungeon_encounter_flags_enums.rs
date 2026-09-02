//! Focused retail 12.0.0 dungeon-encounter flag enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_dungeon_encounter_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    StickyNews = 1,
                    GuildNews = 2,
                    RaidLockPlayers = 4,
                    AutoEnd = 8,
                    Cosmetic = 16,
                    Unused = 32,
                    HideUntilCompleted = 64,
                    NoAutoStart = 128,
                    IgnoreSpawnLimit = 256,
                    DisableEncounterEvents = 512,
                }
                local flags = Enum.DungeonEncounterFlags
                if type(flags) ~= "table" then
                    return "DungeonEncounterFlags: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = flags[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local meta = Enum.DungeonEncounterFlagsMeta
                if type(meta) ~= "table" then
                    return "DungeonEncounterFlagsMeta: expected table"
                end
                if meta.MinValue ~= 1 or meta.MaxValue ~= 512 or meta.NumValues ~= 10 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 dungeon-encounter flag enum mismatch: {result}"
    );
}
