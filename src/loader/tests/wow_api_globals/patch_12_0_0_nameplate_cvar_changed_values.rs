//! Focused retail 12.0.0 changed nameplate and party CVar tests.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_nameplate_cvar_changed_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    {"nameplateLargerScale", "1.200000"},
                    {"nameplateMaxAlpha", "1.000000"},
                    {"nameplateMaxAlphaDistance", "40.000000"},
                    {"nameplateMaxDistance", "60.000000"},
                    {"nameplateMaxScale", "1.000000"},
                    {"nameplateMaxScaleDistance", "10.000000"},
                    {"nameplateMinAlpha", "0.600000"},
                    {"nameplateMinAlphaDistance", "10.000000"},
                    {"nameplateMinScale", "0.800000"},
                    {"nameplateMinScaleDistance", "10.000000"},
                    {"nameplateOccludedAlphaMult", "0.400000"},
                    {"nameplateOverlapH", "0.800000"},
                    {"nameplateOverlapV", "1.100000"},
                    {"nameplatePlayerLargerScale", "1.800000"},
                    {"nameplatePlayerMaxDistance", "60.000000"},
                    {"nameplateSelectedAlpha", "1.000000"},
                    {"nameplateSelectedScale", "1.200000"},
                    {"nameplateSelfAlpha", "0.750000"},
                    {"nameplateShowSelf", "0"},
                    {"nameplateTargetBehindMaxDistance", "0.100000"},
                    {"partyBackgroundOpacity", "0.500000"},
                }
                for index, entry in ipairs(expected) do
                    local name = entry[1]
                    local expected_value = entry[2]
                    local value = GetCVar(name)
                    local default_value = GetCVarDefault(name)
                    if value ~= expected_value or default_value ~= expected_value then
                        return index .. ":" .. name .. ": expected current/default " .. expected_value
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
        "retail 12.0.0 nameplate/party CVar mismatch: {result}"
    );
}
