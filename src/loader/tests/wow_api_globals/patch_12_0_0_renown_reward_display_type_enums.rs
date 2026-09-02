//! Focused retail 12.0.0 renown reward display enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_renown_reward_display_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    None = 0,
                    Item = 1,
                    Spell = 2,
                    Mount = 3,
                    Transmog = 4,
                    TransmogSet = 5,
                    TransmogIllusion = 6,
                    Title = 7,
                    GarrFollower = 8,
                    Currency = 9,
                }
                local display_type = Enum.RenownRewardDisplayType
                if type(display_type) ~= "table" then
                    return "RenownRewardDisplayType: expected table"
                end

                local count = 0
                for name, value in pairs(display_type) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 10 then
                    return "member count: expected 10, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if display_type[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.RenownRewardDisplayTypeMeta
                if type(meta) ~= "table" then
                    return "RenownRewardDisplayTypeMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 9
                    or meta.NumValues ~= 10 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 renown reward display type enum mismatch: {result}"
    );
}
