//! Focused retail 12.0.0 coverage for cooldown viewer and housing enums.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_cooldown_viewer_and_house_exterior_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    {
                        "CooldownViewerAddAlertStatus",
                        {
                            {"Success", 0},
                            {"InvalidAlertType", 1},
                            {"InvalidEventType", 2},
                            {"DuplicateAlert", 3},
                        },
                    },
                    {
                        "CooldownViewerAlertEventType",
                        {
                            {"Available", 1},
                            {"PandemicTime", 2},
                            {"OnCooldown", 3},
                            {"ChargeGained", 4},
                        },
                    },
                    {
                        "HouseExteriorWMODataFlags",
                        {
                            {"None", 0},
                            {"UnlockedByDefault", 1},
                            {"AllowedInHordeNeighborhoods", 2},
                            {"AllowedInAllianceNeighborhoods", 4},
                        },
                    },
                }
                for _, namespace_check in ipairs(expected) do
                    local namespace_name = namespace_check[1]
                    local namespace = Enum[namespace_name]
                    if type(namespace) ~= "table" then
                        return namespace_name .. ":namespace=" .. type(namespace)
                    end
                    for _, value_check in ipairs(namespace_check[2]) do
                        local name = value_check[1]
                        local value = namespace[name]
                        if type(value) ~= "number" then
                            return namespace_name .. "." .. name .. ":type=" .. type(value)
                        end
                        if value ~= value_check[2] then
                            return namespace_name .. "." .. name .. ":value=" .. tostring(value)
                        end
                    end
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 cooldown viewer and housing enum values did not match"
    );
}
