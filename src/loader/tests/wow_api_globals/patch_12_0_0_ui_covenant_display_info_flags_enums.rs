//! Focused retail 12.0.0 UICovenantDisplayInfoFlags enum values.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_ui_covenant_display_info_flags_enum_values() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    DisplayCovenantAsJourney = 1,
                    UseJourneyRewardTrack = 2,
                }
                local flags = Enum.UICovenantDisplayInfoFlags
                if type(flags) ~= "table" then
                    return "UICovenantDisplayInfoFlags: expected table"
                end

                local count = 0
                for name, value in pairs(flags) do
                    count = count + 1
                    local expected_value = expected[name]
                    if expected_value == nil then
                        return name .. ": unexpected member"
                    end
                    if type(value) ~= "number" or value ~= expected_value then
                        return name .. ": unexpected value " .. tostring(value)
                    end
                end
                if count ~= 2 then
                    return "member count: expected 2, got " .. tostring(count)
                end
                for name, expected_value in pairs(expected) do
                    if flags[name] ~= expected_value then
                        return name .. ": missing member"
                    end
                end

                local meta = Enum.UICovenantDisplayInfoFlagsMeta
                if type(meta) ~= "table"
                    or meta.MinValue ~= 1
                    or meta.MaxValue ~= 2
                    or meta.NumValues ~= 2 then
                    return "metadata mismatch"
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 UICovenantDisplayInfoFlags enum mismatch: {result}"
    );
}
