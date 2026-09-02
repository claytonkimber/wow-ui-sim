//! Focused retail 12.0.0 crafting-order result enum value-change tests.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_crafting_order_result_enum_changed_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    MissingItem = 31,
                    MissingNpc = 32,
                    MissingOrder = 33,
                    MissingRecraftItem = 34,
                    NoAccountItems = 35,
                    NotClaimed = 36,
                    NotCrafted = 37,
                    NotInGuild = 38,
                    NotYetImplemented = 39,
                    OutOfPublicOrderCapacity = 40,
                    ServerIsNotAvailable = 41,
                    ThrottleViolation = 42,
                    TargetCannotCraft = 43,
                    TargetLocked = 44,
                    Timeout = 45,
                    TooManyItems = 47,
                    WrongVersion = 48,
                }
                local crafting_order_result = Enum.CraftingOrderResult
                if type(crafting_order_result) ~= "table" then
                    return "CraftingOrderResult: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = crafting_order_result[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local metadata = Enum.CraftingOrderResultMeta
                if type(metadata) ~= "table" then
                    return "CraftingOrderResultMeta: expected table"
                end
                if metadata.MinValue ~= 0
                    or metadata.MaxValue ~= 48
                    or metadata.NumValues ~= 49
                then
                    return "CraftingOrderResultMeta: expected 0/48/49, got "
                        .. tostring(metadata.MinValue) .. "/"
                        .. tostring(metadata.MaxValue) .. "/"
                        .. tostring(metadata.NumValues)
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 crafting-order result enum mismatch: {result}"
    );
}
