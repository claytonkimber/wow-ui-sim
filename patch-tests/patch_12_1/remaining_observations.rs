use super::load_full_game_ui_with_all_lod;

const EXPECTED_ABSENT_ADDITIONS: &[&str] = &[
    "CompactUnitFrameLayoutTemplates_LayoutFrameElement",
    "CompactUnitFrameUtil.ApplyConfig",
    "GameRulesUtil.IsPlayerAtEffectiveMaxLevel",
    "ManageFramePositions",
    "RaidWarningUtil.AddMessage",
    "RaidWarningUtil.ClearBossEmotes",
    "TextureUtil.AnimateTexCoords",
];

const EXPECTED_PRESENT_REMOVALS: &[&str] = &[
    "AddFrameLock",
    "AnimatedShine_OnUpdate",
    "AnimateTexCoords",
    "BattleTagInviteFrame_Show",
    "BetterDate",
    "BFAMissionFrame_EscapePressed",
    "BoostTutorial_AttemptLoad",
    "BuildColoredListString",
    "BuildIconArray",
    "BuildListString",
    "BuildMultilineTooltip",
    "BuildNewLineListString",
    "ButtonPulse_OnUpdate",
    "ClassTrainerFrame_Hide",
    "ClassTrainerFrame_Show",
    "CloseCalendarMenus",
    "CommunitiesFrame_IsEnabled",
    "CompactUnitFrame_GetOptionDisplayOnlyHealerPowerBars",
    "CompactUnitFrame_GetOptionDisplayPowerBar",
    "CompactUnitFrame_GetOptionShowDispelIndicatorOverlay",
    "ConvertRGBtoColorString",
    "DisplayTypeUnassignedSupported",
    "EventUtil.AreVariablesLoaded",
    "ExpansionTrial_CheckLoadUI",
    "FriendsFrame_CloseQuickJoinHelpTip",
    "FriendsFrameAddFriendButton_OnClick",
    "FriendsFriends_InitButton",
    "FriendsFriends_SetSelection",
    "FriendsFriendsButton_SetSelected",
    "FriendsFriendsFrame_Close",
    "GetDungeonNameWithDifficulty",
    "GetSocialColoredName",
    "GetSortedSelfResurrectOptions",
    "HelpPlatesSupported",
    "HousingControlsUtil.CanActivateHousingControls",
    "IsFrameLockActive",
    "IsFrameSmartShown",
    "IsLevelAtEffectiveMaxLevel",
    "IsPlayerAtEffectiveMaxLevel",
    "KeyBindingFrame_LoadUI",
    "LocalizePlayerFrame",
    "LocalizezhCN",
    "LocalizezhTW",
    "MajorFactions_LoadUI",
    "NPE_CheckTutorials",
    "NPETutorial_AttemptToBegin",
    "OpenAchievementFrameToAchievement",
    "OrderHallMissionFrame_EscapePressed",
    "OrderHallTalentFrame_EscapePressed",
    "OutfitterUI_LoadUI",
    "QuickJoin_JoinQueueButtonOnClick",
    "RaidBossEmoteFrame_OnEvent",
    "RaidBossEmoteFrame_OnLoad",
    "RaidBrowser_IsEmpowered",
    "RaidNotice_AddMessage",
    "RaidNotice_ClearSlot",
    "RaidNotice_Clear",
    "RaidNotice_FadeInit",
    "RaidNotice_OnUpdate",
    "RaidNotice_SetSlot",
    "RaidNotice_UpdateSlot",
    "RaidWarningFrame_OnEvent",
    "RaidWarningFrame_OnLoad",
    "RecentTimeDate",
    "RefreshAuras",
    "RegisterNewFrameLock",
    "RemoveFrameLock",
    "ReverseQuestObjective",
    "SetDesaturation",
    "SetFrameLock",
    "ShowResurrectRequest",
    "SmartHide",
    "SmartShow",
    "TalentFrame_LoadUI",
    "TargetFrame_UpdateBuffAnchor",
    "TargetFrame_UpdateDebuffAnchor",
    "ToggleLFGFrame",
    "ToggleRafPanel",
    "ToggleRaidBrowser",
    "ToggleWoWHackCharacterUI",
    "TokenFrame_LoadUI",
    "TrialAccountCapReached_Inform",
    "UIParent_ManageFramePositions",
    "UIParent_OnEvent",
    "UIParent_OnHide",
    "UIParent_OnLoad",
    "UIParent_OnShow",
    "UIParent_Shared_OnEvent",
    "UIParent_Shared_OnLoad",
    "UIParent_UpdateTopFramePositions",
    "UIParentLoadAddOn",
    "UnitHasMana",
    "UpdateFrameLock",
    "UpdateUIElementsForClientScene",
    "WorldFrame_OnLoad",
    "WorldFrame_OnUpdate",
    "WoWHackSpellsUI_LoadUI",
    "getglobal",
    "setglobal",
];

/// Verifies every final 12.1 row against its full-load PTR publication type.
#[test]
fn remaining_symbols_match_observed_publications() {
    let env = load_full_game_ui_with_all_lod();
    let compatibility_mismatches: String = env
        .eval(
            r#"
            local function resolve(path)
                local value = _G
                for part in string.gmatch(path, "[^.]+") do
                    value = value and value[part]
                end
                return value
            end

            local function check_callable(path, ...)
                local value = resolve(path)
                if type(value) ~= "function" then
                    return path .. " expected=function observed=" .. type(value)
                end
                local ok, errorMessage = pcall(value, ...)
                if not ok then
                    return path .. " call failed: " .. tostring(errorMessage)
                end
            end

            local mismatches = {}
            for _, probe in ipairs({
                { "GetInventorySlotInfo", "MainHandSlot" },
                { "C_CVar.GetCVar", "UnitNameFocused" },
                { "C_Housing.IsInsideOwnHouse" },
                { "C_Ping.GetContextualPingTypeForUnit", "player" },
                { "C_RecruitAFriend.IsEnabled" },
                { "C_SuperTrack.GetNextWaypointForMap", 1 },
                { "C_UnitAuras.TriggerPrivateAuraShowDispelType", false },
            }) do
                local path = probe[1]
                local errorMessage = check_callable(path, unpack(probe, 2))
                if errorMessage then
                    table.insert(mismatches, errorMessage)
                end
            end

            local iconSize = resolve("Enum.EditModeUnitFrameSetting.IconSize")
            if type(iconSize) ~= "number" then
                table.insert(mismatches, "Enum.EditModeUnitFrameSetting.IconSize expected=number observed=" .. type(iconSize))
            end
            return table.concat(mismatches, ",")
            "#,
        )
        .expect("pre-removal compatibility observation succeeds");
    assert_eq!(
        compatibility_mismatches, "",
        "pinned Blizzard sources require compatibility symbols before strict removal"
    );

    let mismatches: String = env
        .eval(
            r#"
            local expectedAbsent = {
                "CompactUnitFrameLayoutTemplates_LayoutFrameElement",
                "CompactUnitFrameUtil.ApplyConfig",
                "GameRulesUtil.IsPlayerAtEffectiveMaxLevel",
                "ManageFramePositions",
                "RaidWarningUtil.AddMessage",
                "RaidWarningUtil.ClearBossEmotes",
                "TextureUtil.AnimateTexCoords",
            }
            local expectedFunctions = {
                "AddFrameLock",
                "AnimatedShine_OnUpdate",
                "AnimateTexCoords",
                "BattleTagInviteFrame_Show",
                "BetterDate",
                "BFAMissionFrame_EscapePressed",
                "BoostTutorial_AttemptLoad",
                "BuildColoredListString",
                "BuildIconArray",
                "BuildListString",
                "BuildMultilineTooltip",
                "BuildNewLineListString",
                "ButtonPulse_OnUpdate",
                "ClassTrainerFrame_Hide",
                "ClassTrainerFrame_Show",
                "CloseCalendarMenus",
                "CommunitiesFrame_IsEnabled",
                "CompactUnitFrame_GetOptionDisplayOnlyHealerPowerBars",
                "CompactUnitFrame_GetOptionDisplayPowerBar",
                "CompactUnitFrame_GetOptionShowDispelIndicatorOverlay",
                "ConvertRGBtoColorString",
                "DisplayTypeUnassignedSupported",
                "EventUtil.AreVariablesLoaded",
                "ExpansionTrial_CheckLoadUI",
                "FriendsFrame_CloseQuickJoinHelpTip",
                "FriendsFrameAddFriendButton_OnClick",
                "FriendsFriends_InitButton",
                "FriendsFriends_SetSelection",
                "FriendsFriendsButton_SetSelected",
                "FriendsFriendsFrame_Close",
                "GetDungeonNameWithDifficulty",
                "GetSocialColoredName",
                "GetSortedSelfResurrectOptions",
                "HelpPlatesSupported",
                "HousingControlsUtil.CanActivateHousingControls",
                "IsFrameLockActive",
                "IsFrameSmartShown",
                "IsLevelAtEffectiveMaxLevel",
                "IsPlayerAtEffectiveMaxLevel",
                "KeyBindingFrame_LoadUI",
                "LocalizePlayerFrame",
                "LocalizezhCN",
                "LocalizezhTW",
                "MajorFactions_LoadUI",
                "NPE_CheckTutorials",
                "NPETutorial_AttemptToBegin",
                "OpenAchievementFrameToAchievement",
                "OrderHallMissionFrame_EscapePressed",
                "OrderHallTalentFrame_EscapePressed",
                "OutfitterUI_LoadUI",
                "QuickJoin_JoinQueueButtonOnClick",
                "RaidBossEmoteFrame_OnEvent",
                "RaidBossEmoteFrame_OnLoad",
                "RaidBrowser_IsEmpowered",
                "RaidNotice_AddMessage",
                "RaidNotice_ClearSlot",
                "RaidNotice_Clear",
                "RaidNotice_FadeInit",
                "RaidNotice_OnUpdate",
                "RaidNotice_SetSlot",
                "RaidNotice_UpdateSlot",
                "RaidWarningFrame_OnEvent",
                "RaidWarningFrame_OnLoad",
                "RecentTimeDate",
                "RefreshAuras",
                "RegisterNewFrameLock",
                "RemoveFrameLock",
                "ReverseQuestObjective",
                "SetDesaturation",
                "SetFrameLock",
                "ShowResurrectRequest",
                "SmartHide",
                "SmartShow",
                "TalentFrame_LoadUI",
                "TargetFrame_UpdateBuffAnchor",
                "TargetFrame_UpdateDebuffAnchor",
                "ToggleLFGFrame",
                "ToggleRafPanel",
                "ToggleRaidBrowser",
                "ToggleWoWHackCharacterUI",
                "TokenFrame_LoadUI",
                "TrialAccountCapReached_Inform",
                "UIParent_ManageFramePositions",
                "UIParent_OnEvent",
                "UIParent_OnHide",
                "UIParent_OnLoad",
                "UIParent_OnShow",
                "UIParent_Shared_OnEvent",
                "UIParent_Shared_OnLoad",
                "UIParent_UpdateTopFramePositions",
                "UIParentLoadAddOn",
                "UnitHasMana",
                "UpdateFrameLock",
                "UpdateUIElementsForClientScene",
                "WorldFrame_OnLoad",
                "WorldFrame_OnUpdate",
                "WoWHackSpellsUI_LoadUI",
                "getglobal",
                "setglobal",
            }

            local function Resolve(name)
                local namespaceName, methodName = string.match(name, "^([^.]+)%.(.+)$")
                if namespaceName then
                    local namespace = _G[namespaceName]
                    return namespace and namespace[methodName]
                end
                return _G[name]
            end

            local mismatches = {}
            for _, name in ipairs(expectedAbsent) do
                local value = Resolve(name)
                if value ~= nil then
                    table.insert(mismatches, name .. " expected=nil observed=" .. type(value))
                end
            end
            for _, name in ipairs(expectedFunctions) do
                local value = Resolve(name)
                if type(value) ~= "function" then
                    table.insert(mismatches, name .. " expected=function observed=" .. type(value))
                end
            end
            return table.concat(mismatches, ",")
            "#,
        )
        .expect("remaining symbol observation succeeds");

    assert_eq!(mismatches, "", "PTR publication mismatches");
    assert_eq!(EXPECTED_ABSENT_ADDITIONS.len(), 7);
    assert_eq!(EXPECTED_PRESENT_REMOVALS.len(), 99);
}
