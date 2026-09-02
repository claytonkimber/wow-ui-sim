//! Focused retail 12.0.0 ItemCreationContext removals.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_item_creation_context_members_are_removed() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local context = rawget(Enum, "ItemCreationContext")
                if context then
                    for _, name in ipairs({"Placeholder_12_0_0", "Timewalker"}) do
                        if context[name] ~= nil then
                            return "ItemCreationContext." .. name .. ": expected nil"
                        end
                    end
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 item-creation-context removal mismatch: {result}"
    );
}
