//! Focused retail 12.0.0 housing and recent-ally error constant tests.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_housing_recent_ally_error_constants() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    LE_GAME_ERR_HOUSING_RESULT_MISSING_EXPANSION_ACCESS = 1218,
                    LE_GAME_ERR_HOUSING_RESULT_PERMISSION_DENIED = 1219,
                    LE_GAME_ERR_RECENT_ALLY_PIN_SERVER_ERROR = 1233,
                }
                for name, expected_value in pairs(expected) do
                    local value = _G[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected numeric " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 housing/recent-ally error constant mismatch: {result}"
    );
}
