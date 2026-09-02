//! Focused retail 12.0.0 Edit Mode system enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_edit_mode_system_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local system = Enum.EditModeSystem
                if type(system) ~= "table" then
                    return "EditModeSystem: expected table"
                end

                local expected = {
                    PersonalResourceDisplay = 21,
                    EncounterEvents = 22,
                    DamageMeter = 23,
                }
                for name, expected_value in pairs(expected) do
                    local value = system[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end
                if system.TotemActionBar ~= nil then
                    return "TotemActionBar: expected nil, got " .. tostring(system.TotemActionBar)
                end

                local meta = Enum.EditModeSystemMeta
                if type(meta) ~= "table" then
                    return "EditModeSystemMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 23 or meta.NumValues ~= 24 then
                    return "metadata mismatch"
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 Edit Mode system enum mismatch: {result}"
    );
}
