//! Focused retail 12.0.0 enemy-NPC nameplate aura-display enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_nameplate_enemy_npc_aura_display_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    None = 0,
                    Buffs = 1,
                    Debuffs = 2,
                    CrowdControl = 3,
                }
                local display = Enum.NamePlateEnemyNpcAuraDisplay
                if type(display) ~= "table" then
                    return "NamePlateEnemyNpcAuraDisplay: expected table"
                end

                local count = 0
                for name, value in pairs(display) do
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
                    if display[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.NamePlateEnemyNpcAuraDisplayMeta
                if type(meta) ~= "table" then
                    return "NamePlateEnemyNpcAuraDisplayMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 3 or meta.NumValues ~= 4 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 enemy-NPC nameplate aura-display enum mismatch: {result}"
    );
}
