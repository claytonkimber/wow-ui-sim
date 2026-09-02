//! Focused retail 12.0.0 guild-neighborhood error constant tests.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_guild_neighborhood_error_constants() {
    let env = WowLuaEnv::new().unwrap();
    let result: String = env
        .eval(
            r#"
                local expected = {
                    {
                        name = "LE_GAME_ERR_GUILD_NEIGHBORHOOD_BUILT_HOUSE_S",
                        value = LE_GAME_ERR_GUILD_NEIGHBORHOOD_BUILT_HOUSE_S,
                        expected = 1220,
                    },
                    {
                        name = "LE_GAME_ERR_GUILD_NEIGHBORHOOD_NEW_SUBDIVISION",
                        value = LE_GAME_ERR_GUILD_NEIGHBORHOOD_NEW_SUBDIVISION,
                        expected = 1222,
                    },
                    {
                        name = "LE_GAME_ERR_GUILD_NEIGHBORHOOD_SOLD_HOUSE_S",
                        value = LE_GAME_ERR_GUILD_NEIGHBORHOOD_SOLD_HOUSE_S,
                        expected = 1221,
                    },
                }
                for _, check in ipairs(expected) do
                    if type(check.value) ~= "number" or check.value ~= check.expected then
                        return check.name .. ": expected numeric " .. tostring(check.expected)
                            .. ", got " .. tostring(check.value)
                    end
                end
                return "ok"
            "#,
        )
        .unwrap();
    assert_eq!(
        result, "ok",
        "retail 12.0.0 guild-neighborhood error constant mismatch: {result}"
    );
}
