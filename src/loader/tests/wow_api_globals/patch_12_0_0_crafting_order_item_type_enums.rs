//! Focused retail 12.0.0 crafting-order item-type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_crafting_order_item_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Item = 0,
                    Recraft = 1,
                    CraftedResult = 2,
                    RemoveReagent = 3,
                    Deprecated = 4,
                    Currency = 5,
                }
                local item_type = Enum.CraftingOrderItemType
                if type(item_type) ~= "table" then
                    return "CraftingOrderItemType: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = item_type[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local absent = {"NpcProvided", "Reagent"}
                for index = 1, #absent do
                    local name = absent[index]
                    if item_type[name] ~= nil then
                        return name .. ": expected nil"
                    end
                end

                local meta = Enum.CraftingOrderItemTypeMeta
                if type(meta) ~= "table" then
                    return "CraftingOrderItemTypeMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 5 or meta.NumValues ~= 6 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 crafting-order item-type enum mismatch: {result}"
    );
}
