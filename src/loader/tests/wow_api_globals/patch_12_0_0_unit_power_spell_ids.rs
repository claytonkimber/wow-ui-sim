//! Startup publication and values for 12.0.0 UnitPowerSpellIDs constants.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_unit_power_spell_ids_publish_exact_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                if type(Constants) ~= "table" then return "Constants:type" end
                if type(Constants.UnitPowerSpellIDs) ~= "table" then
                    return "UnitPowerSpellIDs:type"
                end

                local expected = {
                    COLLAPSING_STAR_PASSIVE_SPELL_ID = 1221167,
                    COLLAPSING_STAR_SPELL_ID = 1221150,
                    DARK_HEART_SPELL_ID = 1225789,
                    SILENCE_THE_WHISPERS_SPELL_ID = 1227702,
                    VOID_METAMORPHOSIS_SPELL_ID = 1217607,
                }
                for name, expected_value in pairs(expected) do
                    local actual = Constants.UnitPowerSpellIDs[name]
                    if type(actual) ~= "number" then
                        return name .. ":type=" .. type(actual)
                    end
                    if actual ~= expected_value then
                        return name .. ":value=" .. tostring(actual)
                    end
                end
                return "ok"
            "#,
        )
        .unwrap();

    assert_eq!(result, "ok");
}
