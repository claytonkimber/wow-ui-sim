//! Focused retail 12.0.0 ExpansionLandingPageType removals.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_expansion_landing_page_type_members_are_removed() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local values = rawget(Enum, "ExpansionLandingPageType")
                if values then
                    for _, name in ipairs({"None", "Dragonflight", "WarWithin"}) do
                        if values[name] ~= nil then
                            return "ExpansionLandingPageType." .. name .. ": expected nil"
                        end
                    end
                end

                local meta = rawget(Enum, "ExpansionLandingPageTypeMeta")
                if meta then
                    for _, name in ipairs({"MinValue", "MaxValue", "NumValues"}) do
                        if meta[name] ~= nil then
                            return "ExpansionLandingPageTypeMeta." .. name .. ": expected nil"
                        end
                    end
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 expansion landing-page enum removal mismatch: {result}"
    );
}
