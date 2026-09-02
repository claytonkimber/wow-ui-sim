//! Focused retail 12.0.0 initiative flag enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_initiative_milestone_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local flags = Enum.InitiativeMilestoneFlags
                if type(flags) ~= "table" then
                    return "InitiativeMilestoneFlags: expected table"
                end

                local count = 0
                for name, value in pairs(flags) do
                    count = count + 1
                    if name ~= "FinalMilestone" then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= 1 then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 1 then
                    return "member count: expected 1, got " .. tostring(count)
                end

                local meta = Enum.InitiativeMilestoneFlagsMeta
                if type(meta) ~= "table" then
                    return "InitiativeMilestoneFlagsMeta: expected table"
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
        "retail 12.0.0 initiative milestone flags enum mismatch: {result}"
    );
}

#[test]
fn test_patch_12_0_0_initiative_reward_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local flags = Enum.InitiativeRewardFlags
                if type(flags) ~= "table" then
                    return "InitiativeRewardFlags: expected table"
                end

                local count = 0
                for name, value in pairs(flags) do
                    count = count + 1
                    if name ~= "PermanentWorldState" then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= 1 then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 1 then
                    return "member count: expected 1, got " .. tostring(count)
                end

                local meta = Enum.InitiativeRewardFlagsMeta
                if type(meta) ~= "table" then
                    return "InitiativeRewardFlagsMeta: expected table"
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
        "retail 12.0.0 initiative reward flags enum mismatch: {result}"
    );
}
