//! Focused retail 12.0.0 advanced-flight CVar default tests.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_adv_fly_cvar_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    advFlyKeyboardMaxPitchFactor = "5.000000",
                    advFlyKeyboardMaxTurnFactor = "8.000000",
                    advFlyKeyboardMinPitchFactor = "2.500000",
                    advFlyKeyboardMinTurnFactor = "5.000000",
                    advFlyPitchControlCameraChase = "20.000000",
                }
                for name, expected_value in pairs(expected) do
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
        "retail 12.0.0 advanced-flight CVar mismatch: {result}"
    );
}
