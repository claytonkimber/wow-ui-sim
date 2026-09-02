/// Proves simulator compatibility globals preserve their tested behavior.
#[test]
fn simulator_legacy_compat_globals_preserve_tested_behavior() {
    let env = load_full_game_ui_with_all_lod();
    let result: String = env
        .eval(
            r#"
            local required = {
                "FindBaseSpellByID",
                "FindFlyoutSlotBySpellID",
                "FindSpellOverrideByID",
                "GetBattlegroundInfo",
                "PlaySound",
                "SetPortraitToTexture",
                "strtrim",
            }
            for _, name in ipairs(required) do
                if type(rawget(_G, name)) ~= "function" then
                    return "missing=" .. name
                end
            end

            local oldBase = C_SpellBook.FindBaseSpellByID
            local baseID
            C_SpellBook.FindBaseSpellByID = function(value)
                baseID = value
                return "base-sentinel"
            end
            local baseOK, baseResult = pcall(FindBaseSpellByID, 101)
            C_SpellBook.FindBaseSpellByID = oldBase
            if not baseOK or baseID ~= 101 or baseResult ~= "base-sentinel" then
                return "base-wrapper"
            end

            local oldFlyout = C_SpellBook.FindFlyoutSlotBySpellID
            local flyoutID
            C_SpellBook.FindFlyoutSlotBySpellID = function(value)
                flyoutID = value
                return "flyout-sentinel"
            end
            local flyoutOK, flyoutResult = pcall(FindFlyoutSlotBySpellID, 202)
            C_SpellBook.FindFlyoutSlotBySpellID = oldFlyout
            if not flyoutOK or flyoutID ~= 202 or flyoutResult ~= "flyout-sentinel" then
                return "flyout-wrapper"
            end

            local oldOverride = C_SpellBook.FindSpellOverrideByID
            local overrideID
            C_SpellBook.FindSpellOverrideByID = function(value)
                overrideID = value
                return "override-sentinel"
            end
            local overrideOK, overrideResult = pcall(FindSpellOverrideByID, 303)
            C_SpellBook.FindSpellOverrideByID = oldOverride
            if not overrideOK or overrideID ~= 303 or overrideResult ~= "override-sentinel" then
                return "override-wrapper"
            end

            local name, canEnter, isHoliday, isRandom, bgID, description, mapID, maxPlayers = GetBattlegroundInfo(1)
            if name ~= "Wintergrasp" or canEnter ~= true or isRandom ~= false or bgID ~= 571 or mapID ~= 571 or maxPlayers ~= 40 then
                return "battleground-row"
            end
            if GetBattlegroundInfo(999) ~= nil then
                return "battleground-unknown"
            end

            local soundOK = pcall(PlaySound, 861)
            if not soundOK then
                return "play-sound"
            end

            local frame = CreateFrame("Frame", "LegacyCompatPortraitFrame", UIParent)
            local texture = frame:CreateTexture("LegacyCompatPortraitTexture", "BORDER")
            SetPortraitToTexture(texture, "Interface\\Icons\\Ability_Mount_RidingHorse")
            SetPortraitToTexture(texture, "Interface\\Icons\\INV_Misc_QuestionMark")
            if texture:GetNumMaskTextures() ~= 1 then
                return "portrait-mask"
            end

            if strtrim("  hello \t") ~= "hello" or strtrim("--hello--", "-") ~= "hello" then
                return "strtrim"
            end
            return "ok"
            "#,
        )
        .expect("legacy compatibility probe succeeds");

    assert_eq!(result, "ok");
}
