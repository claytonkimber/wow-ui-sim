//! Focused retail 12.0.0 damage-meter session-type enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_damage_meter_session_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Overall = 0,
                    Current = 1,
                    Expired = 2,
                }
                local session_type = Enum.DamageMeterSessionType
                if type(session_type) ~= "table" then
                    return "DamageMeterSessionType: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = session_type[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local meta = Enum.DamageMeterSessionTypeMeta
                if type(meta) ~= "table" then
                    return "DamageMeterSessionTypeMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 2 or meta.NumValues ~= 3 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 damage-meter session-type enum mismatch: {result}"
    );
}
