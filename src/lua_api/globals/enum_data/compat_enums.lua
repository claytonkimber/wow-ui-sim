-- Hand-maintained compatibility enums for Blizzard UI surfaces that are
-- missing from the generated wowless globals snapshot.

if not Enum.SecondsFormatterInterval then
  Enum.SecondsFormatterInterval = {
    Seconds = 0,
    Minutes = 1,
    Hours = 2,
    Days = 3,
  }
end

if not Enum.SecondsFormatterIntervalMeta then
  Enum.SecondsFormatterIntervalMeta = {
    MinValue = 0,
    MaxValue = 3,
    NumValues = 4,
  }
end

if not Enum.SecondsFormatterAbbreviation then
  Enum.SecondsFormatterAbbreviation = {
    None = 0,
    OneLetter = 1,
    TwoLetters = 2,
    Full = 3,
  }
end

if not Enum.SecondsFormatterAbbreviationMeta then
  Enum.SecondsFormatterAbbreviationMeta = {
    MinValue = 0,
    MaxValue = 3,
    NumValues = 4,
  }
end

if not Enum.SecondsFormatterRounding then
  Enum.SecondsFormatterRounding = {
    RoundUp = 0,
    Truncate = 1,
  }
end

if not Enum.SecondsFormatterRoundingMeta then
  Enum.SecondsFormatterRoundingMeta = {
    MinValue = 0,
    MaxValue = 1,
    NumValues = 2,
  }
end

if not Enum.ScriptObjectAccessRestriction then
  Enum.ScriptObjectAccessRestriction = {
    DenyTaintedAccessWhenAurasAreSecret = 1,
  }
end

if not Enum.ScriptObjectAccessRestrictionMeta then
  Enum.ScriptObjectAccessRestrictionMeta = {
    MinValue = 1,
    MaxValue = 1,
    NumValues = 1,
  }
end

if not Enum.CooldownLayoutStatus then
  Enum.CooldownLayoutStatus = {
    Success = 0,
    InvalidLayoutName = 1,
    TooManyLayouts = 2,
    AttemptToModifyDefaultLayoutWouldCreateTooManyLayouts = 3,
    TooManyAlerts = 4,
    InvalidOrderChange = 5,
    NoValidAlerts = 6,
  }
end

if not Enum.CDMLayoutMode then
  Enum.CDMLayoutMode = {
    AccessOnly = false,
    AllowCreate = true,
  }
end

if not Enum.CooldownLayoutAction then
  Enum.CooldownLayoutAction = {
    ChangeOrder = 0,
    ChangeCategory = 1,
    AddLayout = 2,
    AddAlert = 3,
  }
end

if not Enum.CooldownLayoutType then
  Enum.CooldownLayoutType = {
    Character = 1,
    Account = 2,
  }
end

if not Enum.CharacterCreateRaceMode then
  Enum.CharacterCreateRaceMode = {
    Normal = 0,
    Allied = 1,
  }
end

if not Enum.TransmogOutfitSlotOptionSheatheCategory then
  Enum.TransmogOutfitSlotOptionSheatheCategory = {
    Default = 0,
    Back = 1,
    Side = 2,
    Hide = 3,
  }
end

if not Enum.ExpansionLandingPageType then
  Enum.ExpansionLandingPageType = {
    None = 0,
    Dragonflight = 1,
    WarWithin = 2,
  }
end

if not CompactRaidGroupTypeEnum then
  CompactRaidGroupTypeEnum = {
    Party = Enum.EditModeUnitFrameSystemIndices.Party,
    Raid = Enum.EditModeUnitFrameSystemIndices.Raid,
    Arena = Enum.EditModeUnitFrameSystemIndices.Arena,
  }
end
