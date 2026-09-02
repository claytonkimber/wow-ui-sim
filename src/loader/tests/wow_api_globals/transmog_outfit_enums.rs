//! Startup publication and values for the 12.0.0 transmog outfit enums.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

const TRANSMOG_OUTFIT_SLOT_VALUES: &[(&str, i64)] = &[
    ("Back", 3),
    ("Body", 6),
    ("Chest", 4),
    ("Feet", 11),
    ("Hand", 8),
    ("Head", 0),
    ("Legs", 10),
    ("ShoulderLeft", 2),
    ("ShoulderRight", 1),
    ("Tabard", 5),
    ("Waist", 9),
    ("WeaponMainHand", 12),
    ("WeaponOffHand", 13),
    ("WeaponRanged", 14),
    ("Wrist", 7),
];

const TRANSMOG_OUTFIT_SLOT_ERROR_VALUES: &[(&str, i64)] = &[
    ("CannotUseItem", 10),
    ("IncompatibleWithMainHand", 14),
    ("InvalidDestination", 5),
    ("InvalidItemType", 4),
    ("InvalidSlotForForm", 13),
    ("InvalidSlotForRace", 11),
    ("InvalidSource", 8),
    ("InvalidSourceQuality", 9),
    ("Legendary", 3),
    ("Mismatch", 6),
    ("NoIllusion", 12),
    ("NoItem", 1),
    ("NotSoulbound", 2),
    ("Ok", 0),
    ("SameItem", 7),
];

const TRANSMOG_OUTFIT_SLOT_OPTION_VALUES: &[(&str, i64)] = &[
    ("ArtifactSpecFour", 11),
    ("ArtifactSpecOne", 8),
    ("ArtifactSpecThree", 10),
    ("ArtifactSpecTwo", 9),
    ("DeprecatedReuseMe", 6),
    ("FuryTwoHandedWeapon", 7),
    ("None", 0),
    ("OffHand", 4),
    ("OneHandedWeapon", 1),
    ("RangedWeapon", 3),
    ("Shield", 5),
    ("TwoHandedWeapon", 2),
];

const TRANSMOG_OUTFIT_TRANSACTION_FLAGS_VALUES: &[(&str, i64)] = &[
    ("AddNewOutfitMask", 20),
    ("AddOutfitAndUpdateSlots", 28),
    ("CreateAndUpdateOutfitInfoMask", 6),
    ("CreateOutfitInfo", 4),
    ("FullOutfitUpdateMask", 27),
    ("UpdateMetadata", 1),
    ("UpdateOutfitInfo", 2),
    ("UpdateSituations", 16),
    ("UpdateSituationsMask", 18),
    ("UpdateSlots", 8),
];

fn assert_numeric_namespace(env: &WowLuaEnv, namespace_name: &str, expected: &[(&str, i64)]) {
    let expected_lua = expected
        .iter()
        .map(|(name, value)| format!("[{name:?}] = {value}"))
        .collect::<Vec<_>>()
        .join(",\n                ");
    let script = format!(
        r#"
            local namespace = {namespace_name}
            if type(namespace) ~= "table" then
                return "namespace:" .. type(namespace)
            end
            local expected = {{
                {expected_lua}
            }}
            for name, value in pairs(expected) do
                local actual = namespace[name]
                if type(actual) ~= "number" then
                    return name .. ":type=" .. type(actual)
                end
                if actual ~= value then
                    return name .. ":value=" .. tostring(actual)
                end
            end
            return "ok"
        "#,
        namespace_name = namespace_name,
        expected_lua = expected_lua,
    );
    let result: String = env.eval(&script).unwrap();
    assert_eq!(
        result, "ok",
        "{namespace_name} did not match the 12.0.0 source register"
    );
}

#[test]
fn test_patch_12_0_0_transmog_outfit_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    assert_numeric_namespace(&env, "Enum.TransmogOutfitSlot", TRANSMOG_OUTFIT_SLOT_VALUES);
    assert_numeric_namespace(
        &env,
        "Enum.TransmogOutfitSlotError",
        TRANSMOG_OUTFIT_SLOT_ERROR_VALUES,
    );
    assert_numeric_namespace(
        &env,
        "Enum.TransmogOutfitSlotOption",
        TRANSMOG_OUTFIT_SLOT_OPTION_VALUES,
    );
    assert_numeric_namespace(
        &env,
        "Enum.TransmogOutfitTransactionFlags",
        TRANSMOG_OUTFIT_TRANSACTION_FLAGS_VALUES,
    );
}
