#![cfg(feature = "retail-12-0-0")]

use crate::lua_api::WowLuaEnv;

#[test]
fn test_patch_12_0_0_removed_global_constants() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local names = {
                    "LE_FRAME_TUTORIAL_LINK_TRANSMOG_OUTFIT",
                    "LE_FRAME_TUTORIAL_TRANSMOG_OUTFIT_DROPDOWN",
                    "LE_WORLD_ELAPSED_TIMER_TYPE_CHALLENGE_MODE",
                    "LE_WORLD_ELAPSED_TIMER_TYPE_NONE",
                    "LE_WORLD_ELAPSED_TIMER_TYPE_PROVING_GROUND",
                }
                for _, name in ipairs(names) do
                    if rawget(_G, name) ~= nil then
                        return name .. "=" .. tostring(rawget(_G, name))
                    end
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 removed globals should not be published: {result}"
    );
}
