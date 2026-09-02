//! Focused retail 12.0.0 changed startup constant and CVar value tests.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_changed_startup_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected_globals = {
                    LE_EXPANSION_LEVEL_CURRENT = 11,
                    LE_EXPANSION_LEVEL_PREVIOUS = 10,
                    NUM_LE_EXPANSION_LEVELS = 11,
                    NUM_LE_FRAME_TUTORIALS = 163,
                    NUM_LE_PET_JOURNAL_FILTERS = 4,
                }
                for name, expected_value in pairs(expected_globals) do
                    local value = _G[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected numeric " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local expected_cvars = {
                    EmitterCombatRange = "900.000000",
                    NonEmitterCombatRange = "6400.000000",
                }
                for name, expected_value in pairs(expected_cvars) do
                    local value = GetCVar(name)
                    local default_value = GetCVarDefault(name)
                    if value ~= expected_value or default_value ~= expected_value then
                        return name .. ": expected current/default " .. expected_value
                            .. ", got " .. tostring(value)
                            .. "/" .. tostring(default_value)
                    end
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 changed startup value mismatch: {result}"
    );
}
