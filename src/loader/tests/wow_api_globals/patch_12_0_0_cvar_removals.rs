use crate::lua_api::WowLuaEnv;

#[test]
fn test_patch_12_0_0_force_allow_aero_cvar_removed() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                if GetCVar("ForceAllowAero") ~= nil then
                    return "value=" .. tostring(GetCVar("ForceAllowAero"))
                end
                if GetCVarDefault("ForceAllowAero") ~= nil then
                    return "default=" .. tostring(GetCVarDefault("ForceAllowAero"))
                end
                return "ok"
            "#,
        )
        .unwrap();

    assert_eq!(
        result, "ok",
        "retail 12.0.0 removed ForceAllowAero CVar should have no value or default"
    );
}

#[test]
fn test_patch_12_0_0_removed_nameplate_cvars() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local names = {
                    "NamePlateClassificationScale",
                    "NamePlateHorizontalScale",
                    "NamePlateMaximumClassificationScale",
                    "NamePlateVerticalScale",
                    "NameplatePersonalClickThrough",
                    "NameplatePersonalHideDelayAlpha",
                    "NameplatePersonalHideDelaySeconds",
                    "NameplatePersonalShowAlways",
                    "NameplatePersonalShowInCombat",
                    "NameplatePersonalShowWithTarget",
                    "ShowClassColorInFriendlyNameplate",
                    "ShowClassColorInNameplate",
                    "ShowNamePlateLoseAggroFlash",
                    "TerrainBlendBakeEnable",
                    "TerrainUnlitShaderEnable",
                    "WorldTextCritScreenY",
                    "WorldTextGravity",
                    "WorldTextMinAlpha",
                    "WorldTextNonRandomZ",
                    "WorldTextRampDuration",
                    "WorldTextRampPow",
                    "WorldTextRampPowCrit",
                    "WorldTextRandomXY",
                    "WorldTextRandomZMax",
                    "WorldTextRandomZMin",
                    "WorldTextScale",
                    "WorldTextScreenY",
                    "WorldTextStartPosRandomness",
                    "activeCUFProfile",
                    "currencyTokensBackpack2",
                    "enablePetBattleFloatingCombatText",
                    "floatingCombatTextAllSpellMechanics",
                    "floatingCombatTextAuraFade",
                    "floatingCombatTextAuras",
                    "floatingCombatTextCombatDamage",
                    "floatingCombatTextCombatDamageAllAutos",
                    "floatingCombatTextCombatDamageDirectionalOffset",
                    "floatingCombatTextCombatDamageDirectionalScale",
                    "floatingCombatTextCombatDamageStyle",
                    "floatingCombatTextCombatHealing",
                    "floatingCombatTextCombatHealingAbsorbSelf",
                    "floatingCombatTextCombatHealingAbsorbTarget",
                    "floatingCombatTextCombatLogPeriodicSpells",
                    "floatingCombatTextCombatState",
                    "floatingCombatTextComboPoints",
                    "floatingCombatTextDamageReduction",
                    "floatingCombatTextDodgeParryMiss",
                    "floatingCombatTextEnergyGains",
                    "floatingCombatTextFloatMode",
                    "floatingCombatTextFriendlyHealers",
                    "floatingCombatTextHonorGains",
                    "floatingCombatTextLowManaHealth",
                    "floatingCombatTextPeriodicEnergyGains",
                    "floatingCombatTextPetMeleeDamage",
                    "floatingCombatTextPetSpellDamage",
                    "floatingCombatTextReactives",
                    "floatingCombatTextRepChanges",
                    "floatingCombatTextSpellMechanics",
                    "floatingCombatTextSpellMechanicsOther",
                    "advancedWatchFrame",
                    "currencyTokensBackpack1",
                    "currencyTokensUnused1",
                    "currencyTokensUnused2",
                    "displayedRAFFriendInfo",
                    "friendsSmallView",
                    "friendsViewButtons",
                    "guildRosterView",
                    "lastRenownForMajorFaction2503",
                    "lastRenownForMajorFaction2507",
                    "lastRenownForMajorFaction2510",
                    "lastRenownForMajorFaction2511",
                    "lastRenownForMajorFaction2564",
                    "lastRenownForMajorFaction2570",
                    "lastRenownForMajorFaction2574",
                    "lastRenownForMajorFaction2590",
                    "lastRenownForMajorFaction2593",
                    "lastRenownForMajorFaction2594",
                    "lastRenownForMajorFaction2600",
                    "lastRenownForMajorFaction2653",
                    "lastRenownForMajorFaction2658",
                    "lastRenownForMajorFaction2685",
                    "lastRenownForMajorFaction2688",
                    "lastRenownForMajorFaction2736",
                    "lastTransmogOutfitIDSpec1",
                    "lastTransmogOutfitIDSpec2",
                    "lastTransmogOutfitIDSpec3",
                    "lastTransmogOutfitIDSpec4",
                    "lastVoidStorageTutorial",
                    "lfGuildComment",
                    "lfGuildSettings",
                    "lfgAutoFill",
                    "lfgAutoJoin",
                    "mapAnimDuration",
                    "mapAnimMinAlpha",
                    "mapAnimStartDelay",
                    "minimapAltitudeHintMode",
                    "minimapShowArchBlobs",
                    "minimapShowQuestBlobs",
                    "nameplateClassResourceTopInset",
                    "nameplateGlobalScale",
                    "nameplateHideHealthAndPower",
                    "nameplateLargeBottomInset",
                    "nameplateLargeTopInset",
                    "nameplateMotion",
                    "nameplateMotionSpeed",
                    "nameplateOtherBottomInset",
                    "nameplateOtherTopInset",
                    "nameplateResourceOnTarget",
                    "nameplateSelfBottomInset",
                    "nameplateSelfScale",
                    "nameplateSelfTopInset",
                    "nameplateShowFriendlyBuffs",
                    "nameplateShowFriendlyGuardians",
                    "nameplateShowFriendlyMinions",
                    "nameplateShowFriendlyPets",
                    "nameplateShowFriendlyTotems",
                    "nameplateShowFriends",
                    "nameplateShowOnlyNames",
                    "nameplateShowPersonalCooldowns",
                    "playerStatLeftDropdown",
                    "playerStatRightDropdown",
                    "removeChatDelay",
                    "showQuestObjectivesOnMap",
                    "showTokenFrameHonor",
                    "splashScreenBoost",
                    "splashScreenNormal",
                    "splashScreenSeason",
                    "trackQuestSorting",
                    "unlockedExpansionLandingPages",
                    "watchFrameBaseAlpha",
                    "watchFrameIgnoreCursor",
                    "watchFrameState",
                }
                for _, name in ipairs(names) do
                    local value = GetCVar(name)
                    if value ~= nil then
                        return name .. ":GetCVar=" .. tostring(value)
                    end
                    local default = GetCVarDefault(name)
                    if default ~= nil then
                        return name .. ":GetCVarDefault=" .. tostring(default)
                    end
                end
                return "ok"
            "#,
        )
        .unwrap();

    assert_eq!(
        result, "ok",
        "retail 12.0.0 removed nameplate CVars should have no value or default: {result}"
    );
}
