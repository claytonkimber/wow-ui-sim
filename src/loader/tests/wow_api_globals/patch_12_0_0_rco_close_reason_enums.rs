//! Focused retail 12.0.0 RcoCloseReason rename and alias removal.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_rco_close_reason_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Fulfill = 0,
                    Expire = 1,
                    Cancel = 2,
                    Reject = 3,
                    GmCancel = 4,
                    CrafterFulfill = 5,
                    Invalid = 6,
                }
                local removed = {
                    RcoCloseFulfill = true,
                    RcoCloseExpire = true,
                    RcoCloseCancel = true,
                    RcoCloseReject = true,
                    RcoCloseGmCancel = true,
                    RcoCloseCrafterFulfill = true,
                    RcoCloseInvalid = true,
                }
                local reason = Enum.RcoCloseReason
                if type(reason) ~= "table" then
                    return "RcoCloseReason: expected table"
                end

                local count = 0
                for name, value in pairs(reason) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 7 then
                    return "member count: expected 7, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if reason[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end
                for name in pairs(removed) do
                    if reason[name] ~= nil then
                        return name .. ": removed alias is still present"
                    end
                end

                local meta = Enum.RcoCloseReasonMeta
                if type(meta) ~= "table" then
                    return "RcoCloseReasonMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 6
                    or meta.NumValues ~= 7 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 RcoCloseReason enum mismatch: {result}"
    );
}
