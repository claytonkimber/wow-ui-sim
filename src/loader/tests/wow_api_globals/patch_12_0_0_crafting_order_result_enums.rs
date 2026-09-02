//! Focused retail 12.0.0 crafting-order result enum additions.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_crafting_order_result_enum_additions() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    MissingCurrency = 30,
                    TooManyCurrencies = 46,
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
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 crafting-order result enum mismatch: {result}"
    );
}
