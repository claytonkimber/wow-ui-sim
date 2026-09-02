//! Focused retail 12.0.0 player-companion info flag enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_player_companion_info_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    IgnoreSeasonInScenarios = 1,
                }
                local flags = Enum.PlayerCompanionInfoFlags
                if type(flags) ~= "table" then
                    return "PlayerCompanionInfoFlags: expected table"
                end

                local count = 0
                for name, value in pairs(flags) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 1 then
                    return "member count: expected 1, got " .. tostring(count)
                end
                if flags.IgnoreSeasonInScenarios ~= 1 then
                    return "IgnoreSeasonInScenarios: missing member"
                end

                local meta = Enum.PlayerCompanionInfoFlagsMeta
                if type(meta) ~= "table" then
                    return "PlayerCompanionInfoFlagsMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 1
                    or meta.MaxValue ~= 1
                    or meta.NumValues ~= 1 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 player companion info flags enum mismatch: {result}"
    );
}
