//! Focused retail 12.0.0 Edit Mode encounter-events setting enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_edit_mode_encounter_events_settings_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    Orientation = 0,
                    IconDirection = 1,
                    ShowSpellName = 2,
                    IconSize = 3,
                    OverallSize = 4,
                    BackgroundTransparency = 5,
                    Transparency = 6,
                    Visibility = 7,
                    ShowTooltips = 8,
                    ShowTimer = 9,
                }
                local setting = Enum.EditModeEncounterEventsSetting
                if type(setting) ~= "table" then
                    return "EditModeEncounterEventsSetting: expected table"
                end
                for name, expected_value in pairs(expected) do
                    local value = setting[name]
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": expected " .. tostring(expected_value)
                            .. ", got " .. tostring(value)
                    end
                end
                local absent = {
                    "TooltipAnchor",
                    "ViewType",
                    "FlipHorizontally",
                    "BarWidth",
                    "Padding",
                }
                for _, name in ipairs(absent) do
                    if setting[name] ~= nil then
                        return name .. ": expected nil"
                    end
                end

                local meta = Enum.EditModeEncounterEventsSettingMeta
                if type(meta) ~= "table" then
                    return "EditModeEncounterEventsSettingMeta: expected table"
                end
                if meta.MinValue ~= 0 or meta.MaxValue ~= 9 or meta.NumValues ~= 10 then
                    return "metadata mismatch"
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 Edit Mode encounter-events setting mismatch: {result}"
    );
}
