//! Focused retail 12.0.0 item creation context additions.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_item_creation_context_additions() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local context = Enum.ItemCreationContext
                if type(context) ~= "table" then
                    return "ItemCreationContext: expected table"
                end
                if type(context.TimewalkerLevelUp) ~= "number"
                    or context.TimewalkerLevelUp ~= 22 then
                    return "TimewalkerLevelUp mismatch"
                end
                if type(context.TimewalkerMaxLevel) ~= "number"
                    or context.TimewalkerMaxLevel ~= 186 then
                    return "TimewalkerMaxLevel mismatch"
                end

                local meta = Enum.ItemCreationContextMeta
                if type(meta) ~= "table" then
                    return "ItemCreationContextMeta: expected table"
                end
                if type(meta.MinValue) ~= "number"
                    or type(meta.MaxValue) ~= "number"
                    or type(meta.NumValues) ~= "number"
                    or meta.MinValue ~= 0
                    or meta.MaxValue ~= 186
                    or meta.NumValues ~= 187 then
                    return "metadata mismatch"
                end

                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 item creation context additions mismatch: {result}"
    );
}
