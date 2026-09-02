//! Focused retail 12.0.7 Edit Mode enum values used by Blizzard_EditMode.

#![cfg(feature = "retail-12-0-7")]

use super::super::*;

#[test]
fn test_patch_12_0_7_edit_mode_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local encounter = Enum.EditModeEncounterEventsSetting
                if type(encounter) ~= "table" then
                    return "EditModeEncounterEventsSetting: expected table"
                end
                if encounter.TooltipAnchor ~= 8 then
                    return "TooltipAnchor: expected 8, got " .. tostring(encounter.TooltipAnchor)
                end
                if encounter.ShowTooltips ~= 8 then
                    return "ShowTooltips: expected preserved value 8, got " .. tostring(encounter.ShowTooltips)
                end

                local encounterMeta = Enum.EditModeEncounterEventsSettingMeta
                if type(encounterMeta) ~= "table" then
                    return "EditModeEncounterEventsSettingMeta: expected table"
                end
                if encounterMeta.MinValue ~= 0
                    or encounterMeta.MaxValue ~= 13
                    or encounterMeta.NumValues ~= 14
                then
                    return "EditModeEncounterEventsSettingMeta: metadata mismatch"
                end

                local visibility = Enum.DamageMeterVisibility
                if type(visibility) ~= "table" then
                    return "DamageMeterVisibility: expected table"
                end
                if visibility.InGroup ~= 3 then
                    return "InGroup: expected 3, got " .. tostring(visibility.InGroup)
                end

                local visibilityMeta = Enum.DamageMeterVisibilityMeta
                if type(visibilityMeta) ~= "table" then
                    return "DamageMeterVisibilityMeta: expected table"
                end
                if visibilityMeta.MinValue ~= 0
                    or visibilityMeta.MaxValue ~= 3
                    or visibilityMeta.NumValues ~= 4
                then
                    return "DamageMeterVisibilityMeta: metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.7 Edit Mode enum mismatch: {result}"
    );
}
