//! Focused retail 12.0.0 NPC crafting-order set flag enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_npc_crafting_order_set_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    AllowMultiple = 1,
                    AllowDuplicate = 2,
                }
                local flags = Enum.NpcCraftingOrderSetFlags
                if type(flags) ~= "table" then
                    return "NpcCraftingOrderSetFlags: expected table"
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
                if count ~= 2 then
                    return "member count: expected 2, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if flags[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local removed_aliases = {
                    "CraftingOrderFlagAllowMultiple",
                    "CraftingOrderFlagAllowDuplicate",
                }
                for _, name in ipairs(removed_aliases) do
                    if flags[name] ~= nil then
                        return name .. ": removed alias still present"
                    end
                end

                local meta = Enum.NpcCraftingOrderSetFlagsMeta
                if type(meta) ~= "table" then
                    return "NpcCraftingOrderSetFlagsMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 1
                    or meta.MaxValue ~= 2
                    or meta.NumValues ~= 2 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 NPC crafting-order set flags enum mismatch: {result}"
    );
}
