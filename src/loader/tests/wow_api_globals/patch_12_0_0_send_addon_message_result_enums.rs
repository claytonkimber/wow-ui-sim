//! Focused retail 12.0.0 SendAddonMessageResult enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_send_addon_message_result_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Success = 0,
                    InvalidPrefix = 1,
                    InvalidMessage = 2,
                    AddonMessageThrottle = 3,
                    InvalidChatType = 4,
                    NotInGroup = 5,
                    TargetRequired = 6,
                    InvalidChannel = 7,
                    ChannelThrottle = 8,
                    GeneralError = 9,
                    NotInGuild = 10,
                    AddOnMessageLockdown = 11,
                    TargetOffline = 12,
                }
                local result_type = Enum.SendAddonMessageResult
                if type(result_type) ~= "table" then
                    return "SendAddonMessageResult: expected table"
                end

                local count = 0
                for name, value in pairs(result_type) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 13 then
                    return "member count: expected 13, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if result_type[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.SendAddonMessageResultMeta
                if type(meta) ~= "table" then
                    return "SendAddonMessageResultMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 12
                    or meta.NumValues ~= 13 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 SendAddonMessageResult enum mismatch: {result}"
    );
}
