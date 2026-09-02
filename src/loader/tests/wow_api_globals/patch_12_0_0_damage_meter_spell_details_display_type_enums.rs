//! Focused retail 12.0.0 damage-meter spell-details display enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_damage_meter_spell_details_display_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    SpellCasted = 0,
                    UnitSpecificSpellCasted = 1,
                    SpellAffected = 2,
                }
                local display_type = Enum.DamageMeterSpellDetailsDisplayType
                if type(display_type) ~= "table" then
                    return "DamageMeterSpellDetailsDisplayType: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = display_type[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end
                for _, name in ipairs({"Deaths", "EnemyDamageTaken"}) do
                    if display_type[name] ~= nil then
                        return name .. ": expected nil"
                    end
                end

                local meta = Enum.DamageMeterSpellDetailsDisplayTypeMeta
                if type(meta) ~= "table" then
                    return "DamageMeterSpellDetailsDisplayTypeMeta: expected table"
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
        "retail 12.0.0 damage-meter spell-details display enum mismatch: {result}"
    );
}
