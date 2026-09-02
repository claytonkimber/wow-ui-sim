//! Focused retail 12.0.0 LFG entry general playstyle enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_lfg_entry_general_playstyle_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    None = 0,
                    Learning = 1,
                    FunRelaxed = 2,
                    FunSerious = 3,
                    Expert = 4,
                }
                local playstyle = Enum.LFGEntryGeneralPlaystyle
                if type(playstyle) ~= "table" then
                    return "LFGEntryGeneralPlaystyle: expected table"
                end

                local count = 0
                for name, value in pairs(playstyle) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 5 then
                    return "member count: expected 5, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if playstyle[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.LFGEntryGeneralPlaystyleMeta
                if type(meta) ~= "table" then
                    return "LFGEntryGeneralPlaystyleMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 4
                    or meta.NumValues ~= 5 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 LFG entry general playstyle enum mismatch: {result}"
    );
}
