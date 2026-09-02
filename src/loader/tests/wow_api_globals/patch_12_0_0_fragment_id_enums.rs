//! Focused retail 12.0.0 FragmentID additions.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_fragment_id_additions() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local fragment = Enum.FragmentID
                if type(fragment) ~= "table" then
                    return "FragmentID: expected table"
                end

                local expected = {
                    FPlayerInitiativeInfo = 37,
                    FNeighborhoodStateData = 38,
                    FUnitAIGroupLink = 39,
                }
                for name, expected_value in pairs(expected) do
                    local value = fragment[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end

                local meta = Enum.FragmentIDMeta
                if type(meta) ~= "table" then
                    return "FragmentIDMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 255
                    or meta.NumValues ~= 72 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 FragmentID additions mismatch: {result}"
    );
}
