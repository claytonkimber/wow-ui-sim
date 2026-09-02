//! Focused retail 12.0.0 transmog situation enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_transmog_situation_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local function check_family(name, expected, min_value, max_value, num_values)
                    local family = Enum[name]
                    if type(family) ~= "table" then
                        return name .. ": expected table"
                    end
                    local count = 0
                    for member, value in pairs(family) do
                        count = count + 1
                        local expected_value = expected[member]
                        if expected_value == nil then
                            return name .. "." .. member .. ": unexpected member"
                        end
                        if type(value) ~= "number" or value ~= expected_value then
                            return name .. "." .. member .. ": unexpected value " .. tostring(value)
                        end
                    end
                    if count ~= num_values then
                        return name .. ": member count mismatch"
                    end
                    for member, expected_value in pairs(expected) do
                        if family[member] ~= expected_value then
                            return name .. "." .. member .. ": missing member"
                        end
                    end
                    local meta = Enum[name .. "Meta"]
                    if type(meta) ~= "table"
                        or meta.MinValue ~= min_value
                        or meta.MaxValue ~= max_value
                        or meta.NumValues ~= num_values then
                        return name .. ": metadata mismatch"
                    end
                    return nil
                end

                local families = {
                    {
                        name = "TransmogSituationFlags",
                        expected = {
                            IsPlayerFacing = 1,
                            SpecUseTalentLoadout = 2,
                            AllSituation = 4,
                            DefaultsToOn = 8,
                            DynamicallyNamed = 16,
                            NoneSituation = 32,
                            DisabledSituation = 64,
                        },
                        min_value = 1,
                        max_value = 64,
                        num_values = 7,
                    },
                    {
                        name = "TransmogSituationGroupFlags",
                        expected = {DynamicallyCreatesGroups = 1},
                        min_value = 1,
                        max_value = 1,
                        num_values = 1,
                    },
                    {
                        name = "TransmogSituationTrigger",
                        expected = {
                            None = 0,
                            Manual = 1,
                            TransmogUpdate = 2,
                            Location = 3,
                            Movement = 4,
                            Specialization = 5,
                            EquipmentSet = 6,
                            Forms = 7,
                            EventOutfit = 8,
                        },
                        min_value = 0,
                        max_value = 8,
                        num_values = 9,
                    },
                    {
                        name = "TransmogSituationTriggerFlags",
                        expected = {
                            CanLockOutfit = 1,
                            CanChangeLockedOutfit = 2,
                            IsPlayerFacing = 4,
                            SituationsAreExclusive = 8,
                            DisabledTrigger = 16,
                        },
                        min_value = 1,
                        max_value = 16,
                        num_values = 5,
                    },
                    {
                        name = "TransmogSituationTriggerType",
                        expected = {
                            None = 0,
                            Manual = 1,
                            Automatic = 2,
                            TransmogUpdate = 3,
                        },
                        min_value = 0,
                        max_value = 3,
                        num_values = 4,
                    },
                }

                for _, family in ipairs(families) do
                    local error_message = check_family(
                        family.name,
                        family.expected,
                        family.min_value,
                        family.max_value,
                        family.num_values
                    )
                    if error_message then
                        return error_message
                    end
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 transmog situation enum mismatch: {result}"
    );
}
