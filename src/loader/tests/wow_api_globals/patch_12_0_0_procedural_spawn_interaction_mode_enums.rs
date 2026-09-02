//! Focused retail 12.0.0 procedural-spawn interaction-mode enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_procedural_spawn_interaction_mode_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    None = 0,
                    Paint = 1,
                    Manipulate = 2,
                }
                local mode = Enum.ProceduralSpawnInteractionMode
                if type(mode) ~= "table" then
                    return "ProceduralSpawnInteractionMode: expected table"
                end

                local count = 0
                for name, value in pairs(mode) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 3 then
                    return "member count: expected 3, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if mode[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.ProceduralSpawnInteractionModeMeta
                if type(meta) ~= "table" then
                    return "ProceduralSpawnInteractionModeMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 2
                    or meta.NumValues ~= 3 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 procedural spawn interaction mode enum mismatch: {result}"
    );
}
