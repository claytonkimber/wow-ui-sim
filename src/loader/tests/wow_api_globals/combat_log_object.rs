//! Startup publication and values for the 12.0.0 combat-log object enums.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

const COMBAT_LOG_OBJECT_VALUES: &[(&str, &[(&str, i64)])] = &[
    (
        "CombatLogObject",
        &[
            ("AffiliationMine", 1),
            ("AffiliationOutsider", 8),
            ("AffiliationParty", 2),
            ("AffiliationRaid", 4),
            ("ControlNpc", 512),
            ("ControlPlayer", 256),
            ("Empty", 0),
            ("Focus", 131_072),
            ("Mainassist", 524_288),
            ("Maintank", 262_144),
            ("None", 2_147_483_648),
            ("ReactionFriendly", 16),
            ("ReactionHostile", 64),
            ("ReactionNeutral", 32),
            ("Target", 65_536),
            ("TypeGuardian", 8_192),
            ("TypeNpc", 2_048),
            ("TypeObject", 16_384),
            ("TypePet", 4_096),
            ("TypePlayer", 1_024),
        ],
    ),
    (
        "CombatLogObjectMeta",
        &[
            ("MaxValue", -2_147_483_648),
            ("MinValue", 0),
            ("NumValues", 20),
        ],
    ),
    (
        "CombatLogObjectTarget",
        &[
            ("RaidNone", 2_147_483_648),
            ("Raidtarget1", 1),
            ("Raidtarget2", 2),
            ("Raidtarget3", 4),
            ("Raidtarget4", 8),
            ("Raidtarget5", 16),
            ("Raidtarget6", 32),
            ("Raidtarget7", 64),
            ("Raidtarget8", 128),
        ],
    ),
    (
        "CombatLogObjectTargetMeta",
        &[
            ("MaxValue", -2_147_483_648),
            ("MinValue", 1),
            ("NumValues", 9),
        ],
    ),
];

#[test]
fn test_patch_12_0_0_combat_log_object_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let expected_namespaces = COMBAT_LOG_OBJECT_VALUES
        .iter()
        .map(|(namespace, values)| {
            let expected_values = values
                .iter()
                .map(|(name, value)| format!("[{name:?}] = {value}"))
                .collect::<Vec<_>>()
                .join(",\n                    ");
            format!(
                "[{namespace:?}] = {{\n                    {expected_values}\n                }}"
            )
        })
        .collect::<Vec<_>>()
        .join(",\n                ");
    let script = format!(
        r#"
            local expected = {{
                {expected_namespaces}
            }}
            for namespace_name, expected_values in pairs(expected) do
                local namespace = Enum[namespace_name]
                if type(namespace) ~= "table" then
                    return namespace_name .. ":namespace=" .. type(namespace)
                end
                for name, value in pairs(expected_values) do
                    local actual = namespace[name]
                    if type(actual) ~= "number" then
                        return namespace_name .. "." .. name .. ":type=" .. type(actual)
                    end
                    if actual ~= value then
                        return namespace_name .. "." .. name .. ":value=" .. tostring(actual)
                    end
                end
            end
            return "ok"
        "#,
        expected_namespaces = expected_namespaces,
    );
    let result: String = env.eval(&script).unwrap();
    assert_eq!(
        result, "ok",
        "CombatLogObject enum namespaces did not match the 12.0.0 source register"
    );
}
