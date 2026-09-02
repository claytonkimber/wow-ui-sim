//! Focused retail 12.0.0 TooltipDataLineType enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_tooltip_data_line_type_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    None = 0,
                    Blank = 1,
                    UnitName = 2,
                    GemSocket = 3,
                    AzeriteEssenceSlot = 4,
                    AzeriteEssencePower = 5,
                    LearnableSpell = 6,
                    UnitThreat = 7,
                    QuestObjective = 8,
                    AzeriteItemPowerDescription = 9,
                    RuneforgeLegendaryPowerDescription = 10,
                    SellPrice = 11,
                    ProfessionCraftingQuality = 12,
                    SpellName = 13,
                    CurrencyTotal = 14,
                    ItemEnchantmentPermanent = 15,
                    UnitOwner = 16,
                    QuestTitle = 17,
                    QuestPlayer = 18,
                    NestedBlock = 19,
                    ItemBinding = 20,
                    RestrictedRaceClass = 21,
                    RestrictedFaction = 22,
                    RestrictedSkill = 23,
                    RestrictedPvPMedal = 24,
                    RestrictedReputation = 25,
                    RestrictedSpellKnown = 26,
                    RestrictedLevel = 27,
                    EquipSlot = 28,
                    ItemName = 29,
                    Separator = 30,
                    ToyName = 31,
                    ToyText = 32,
                    ToyEffect = 33,
                    ToyDuration = 34,
                    RestrictedArena = 35,
                    RestrictedBg = 36,
                    ToyFlavorText = 37,
                    ToyDescription = 38,
                    ToySource = 39,
                    GemSocketEnchantment = 40,
                    ItemLevel = 41,
                    ItemUpgradeLevel = 42,
                    SpellPassive = 43,
                    SpellDescription = 44,
                }
                local line_type = Enum.TooltipDataLineType
                if type(line_type) ~= "table" then
                    return "TooltipDataLineType: expected table"
                end

                local count = 0
                for name, value in pairs(line_type) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 45 then
                    return "member count: expected 45, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if line_type[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                    if type(line_type[name]) ~= "number" then
                        return name .. ": expected number"
                    end
                end

                local meta = Enum.TooltipDataLineTypeMeta
                if type(meta) ~= "table" then
                    return "TooltipDataLineTypeMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 44
                    or meta.NumValues ~= 45 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 TooltipDataLineType enum mismatch: {result}"
    );
}
