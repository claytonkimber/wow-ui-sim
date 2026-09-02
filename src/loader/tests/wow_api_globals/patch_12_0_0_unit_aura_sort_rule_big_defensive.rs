//! Focused retail 12.0.0 UnitAuraSortRule BigDefensive enum value.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_unit_aura_sort_rule_big_defensive_value() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local rule = Enum.UnitAuraSortRule
                if type(rule) ~= "table" then
                    return "UnitAuraSortRule: expected table"
                end
                if type(rule.BigDefensive) ~= "number" or rule.BigDefensive ~= 2 then
                    return "BigDefensive: unexpected value"
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 BigDefensive enum mismatch: {result}"
    );
}
