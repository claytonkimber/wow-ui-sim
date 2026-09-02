//! Focused retail 12.0.0 transmog outfit equip action enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_transmog_outfit_equip_action_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Equip = 0,
                    EquipAndLock = 1,
                    Remove = 2,
                    RemoveAndLock = 3,
                    Unlock = 4,
                    Lock = 5,
                }
                local action = Enum.TransmogOutfitEquipAction
                if type(action) ~= "table" then
                    return "TransmogOutfitEquipAction: expected table"
                end

                local count = 0
                for name, value in pairs(action) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 6 then
                    return "member count: expected 6, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if action[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.TransmogOutfitEquipActionMeta
                if type(meta) ~= "table" then
                    return "TransmogOutfitEquipActionMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 5
                    or meta.NumValues ~= 6 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 transmog outfit equip action enum mismatch: {result}"
    );
}
