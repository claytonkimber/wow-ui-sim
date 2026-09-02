//! Focused retail 12.0.0 enum values from the API audit.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_audit_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local function check_namespace(namespace_name)
                    local namespace = Enum[namespace_name]
                    if type(namespace) ~= "table" then
                        return namespace_name .. ": expected table, got " .. type(namespace)
                    end
                end

                local function check_value(namespace_name, value_name, expected_value)
                    local namespace_error = check_namespace(namespace_name)
                    if namespace_error then
                        return namespace_error
                    end

                    local value = Enum[namespace_name][value_name]
                    if type(value) ~= "number" then
                        return namespace_name .. "." .. value_name
                            .. ": expected number, got " .. type(value)
                    end
                    if value ~= expected_value then
                        return namespace_name .. "." .. value_name
                            .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local function check_meta(namespace_name, expected_min, expected_max, expected_num)
                    local meta_name = namespace_name .. "Meta"
                    local namespace_error = check_namespace(meta_name)
                    if namespace_error then
                        return namespace_error
                    end

                    local meta = Enum[meta_name]
                    local expected = {
                        MinValue = expected_min,
                        MaxValue = expected_max,
                        NumValues = expected_num,
                    }
                    for value_name, expected_value in pairs(expected) do
                        local value = meta[value_name]
                        if type(value) ~= "number" then
                            return meta_name .. "." .. value_name
                                .. ": expected number, got " .. type(value)
                        end
                        if value ~= expected_value then
                            return meta_name .. "." .. value_name
                                .. ": expected " .. tostring(expected_value)
                                .. ", got " .. tostring(value)
                        end
                    end
                end

                local function check_absent(namespace_name, value_name)
                    local namespace_error = check_namespace(namespace_name)
                    if namespace_error then
                        return namespace_error
                    end

                    local value = Enum[namespace_name][value_name]
                    if value ~= nil then
                        return namespace_name .. "." .. value_name
                            .. ": expected absent, got " .. tostring(value)
                    end
                end

                local checks = {
                    {"AccountDataUpdateStatus", "Corrupt", 2},
                    {"AccountDataUpdateStatus", "Failed", 1},
                    {"AccountDataUpdateStatus", "Success", 0},
                    {"AccountDataUpdateStatus", "Toobig", 3},

                    {"AddOnRestrictionState", "Inactive", 0},
                    {"AddOnRestrictionState", "Activating", 1},
                    {"AddOnRestrictionState", "Active", 2},

                    {"AddOnRestrictionType", "Combat", 0},
                    {"AddOnRestrictionType", "Encounter", 1},
                    {"AddOnRestrictionType", "ChallengeMode", 2},
                    {"AddOnRestrictionType", "PvPMatch", 3},
                    {"AddOnRestrictionType", "Map", 4},

                    {"AuraFrameVisibleSetting", "Always", 0},
                    {"AuraFrameVisibleSetting", "InCombat", 1},
                    {"AuraFrameVisibleSetting", "Hidden", 2},

                    {"BulkPurchaseResult", "ResultOk", 0},
                    {"BulkPurchaseResult", "ResultInProgress", 1},
                    {"BulkPurchaseResult", "ResultPartialSuccess", 2},
                    {"BulkPurchaseResult", "ResultFailed", 3},
                    {"BulkPurchaseResult", "ResultTooManyProducts", 4},
                    {"BulkPurchaseResult", "ResultSystemDisabled", 5},
                    {"BulkPurchaseResult", "ResultInsufficientFunds", 6},
                    {"BulkPurchaseResult", "ResultPurchaseTimeout", 7},

                    {"BulkRefundResult", "ResultOk", 0},
                    {"BulkRefundResult", "ResultFailed", 1},
                    {"BulkRefundResult", "ResultInvalidRequest", 2},
                    {"BulkRefundResult", "ResultRefundWindowExpired", 3},
                    {"BulkRefundResult", "ResultSystemDisabled", 4},
                    {"BulkRefundResult", "ResultTimeout", 5},
                }

                local absent_checks = {
                    {"AccountDataUpdateStatus", "AccountDataUpdateSuccess"},
                    {"AccountDataUpdateStatus", "AccountDataUpdateFailed"},
                    {"AccountDataUpdateStatus", "AccountDataUpdateCorrupt"},
                    {"AccountDataUpdateStatus", "AccountDataUpdateToobig"},
                }

                local meta_checks = {
                    {"AccountDataUpdateStatus", 0, 3, 4},
                    {"AddOnRestrictionState", 0, 2, 3},
                    {"AddOnRestrictionType", 0, 4, 5},
                    {"AuraFrameVisibleSetting", 0, 2, 3},
                    {"BulkPurchaseResult", 0, 7, 8},
                    {"BulkRefundResult", 0, 5, 6},
                }

                for index = 1, #checks do
                    local check = checks[index]
                    local error_message = check_value(check[1], check[2], check[3])
                    if error_message then
                        return error_message
                    end
                end

                for index = 1, #meta_checks do
                    local check = meta_checks[index]
                    local error_message = check_meta(check[1], check[2], check[3], check[4])
                    if error_message then
                        return error_message
                    end
                end

                for index = 1, #absent_checks do
                    local check = absent_checks[index]
                    local error_message = check_absent(check[1], check[2])
                    if error_message then
                        return error_message
                    end
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 audit enum table/type/value mismatch: {result}"
    );
}
