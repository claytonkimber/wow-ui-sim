//! Focused retail 12.0.0 renown reward flag enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_renown_rewards_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Milestone = 1,
                    Capstone = 2,
                    Hidden = 4,
                    AccountUnlock = 8,
                }
                local flags = Enum.RenownRewardsFlags
                if type(flags) ~= "table" then
                    return "RenownRewardsFlags: expected table"
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
                if count ~= 4 then
                    return "member count: expected 4, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if flags[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.RenownRewardsFlagsMeta
                if type(meta) ~= "table" then
                    return "RenownRewardsFlagsMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 1
                    or meta.MaxValue ~= 8
                    or meta.NumValues ~= 4 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 renown rewards flags enum mismatch: {result}"
    );
}
