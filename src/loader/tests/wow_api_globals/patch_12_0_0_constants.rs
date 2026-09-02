//! Focused retail 12.0.0 startup constants.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_grouped_constant_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local function check_constant(namespace_name, constant_name, expected_value)
                    local namespace = Constants[namespace_name]
                    if type(namespace) ~= "table" then
                        return namespace_name .. ".<namespace>: expected table, got " .. type(namespace)
                    end

                    local value = namespace[constant_name]
                    if type(value) ~= "number" then
                        return namespace_name .. "." .. constant_name
                            .. ": expected number, got " .. type(value)
                    end
                    if value ~= expected_value then
                        return namespace_name .. "." .. constant_name
                            .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end

                local checks = {
                    {"CombatLogObjectTargetMasks", "COMBATLOG_OBJECT_RAID_MASK", -1},
                    {"CombatLogObjectTargetMasks", "COMBATLOG_OBJECT_RAID_TARGET_MASK", 255},
                    {"CurrencyConsts", "CURRENCY_WALLET_TYPE_WOWMONEY", 0},
                    {"ItemConsts_Mainline", "HWM_SQUISH_ERA_PLAYER_DATA_ACCOUNT_ELEMENT_ID", 212},
                    {"ProfessionConsts", "CRAFTING_ORDER_CURRENCY_WALLET_ITEM_ID", 262724},
                    {"TransmogOutfitDataConsts", "EQUIP_TRANSMOG_OUTFIT_MANUAL_SPELL_ID", 1247613},

                    {"EncounterTimelineEventConstants", "ENCOUNTER_TIMELINE_INVALID_EVENT", 0},
                    {"EncounterTimelineEventConstants", "ENCOUNTER_TIMELINE_RESERVED_EVENT_COUNT", 40},
                    {"EncounterTimelineIconMasks", "EncounterTimelineAllIcons", 1023},
                    {"EncounterTimelineIconMasks", "EncounterTimelineDamageAlertIcons", 512},
                    {"EncounterTimelineIconMasks", "EncounterTimelineDeadlyIcons", 1},
                    {"EncounterTimelineIconMasks", "EncounterTimelineDispelIcons", 124},
                    {"EncounterTimelineIconMasks", "EncounterTimelineEnrageIcons", 2},
                    {"EncounterTimelineIconMasks", "EncounterTimelineHealerAlertIcons", 256},
                    {"EncounterTimelineIconMasks", "EncounterTimelineNoIcons", 0},
                    {"EncounterTimelineIconMasks", "EncounterTimelineOtherIcons", 127},
                    {"EncounterTimelineIconMasks", "EncounterTimelineRoleIcons", 896},
                    {"EncounterTimelineIconMasks", "EncounterTimelineTankAlertIcons", 128},

                    {"TTSConstants", "TTSRateDefault", 0},
                    {"TTSConstants", "TTSRateMax", 10},
                    {"TTSConstants", "TTSRateMin", -10},
                    {"TTSConstants", "TTSVolumeDefault", 100},
                    {"TTSConstants", "TTSVolumeMax", 100},
                    {"TTSConstants", "TTSVolumeMin", 0},

                    {"UICharacterClasses", "Warrior", 1},
                    {"UICharacterClasses", "Paladin", 2},
                    {"UICharacterClasses", "Hunter", 3},
                    {"UICharacterClasses", "Rogue", 4},
                    {"UICharacterClasses", "Priest", 5},
                    {"UICharacterClasses", "DeathKnight", 6},
                    {"UICharacterClasses", "Shaman", 7},
                    {"UICharacterClasses", "Mage", 8},
                    {"UICharacterClasses", "Warlock", 9},
                    {"UICharacterClasses", "Monk", 10},
                    {"UICharacterClasses", "Druid", 11},
                    {"UICharacterClasses", "DemonHunter", 12},
                    {"UICharacterClasses", "Evoker", 13},

                    {"UnitEventConstants", "MAX_UNIT_TOKENS_IN_EVENT", 4},
                }

                for index = 1, #checks do
                    local check = checks[index]
                    local error_message = check_constant(check[1], check[2], check[3])
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
        "retail 12.0.0 grouped constant publication/type/value mismatch: {result}"
    );
}
