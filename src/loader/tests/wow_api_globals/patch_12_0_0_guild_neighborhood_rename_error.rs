//! Focused retail 12.0.0 guild-neighborhood rename error constant test.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_le_game_err_guild_neighborhood_rename_s() {
    let env = WowLuaEnv::new().unwrap();
    let result: (String, f64) = env
        .eval(
            r#"
                return type(LE_GAME_ERR_GUILD_NEIGHBORHOOD_RENAME_S),
                    LE_GAME_ERR_GUILD_NEIGHBORHOOD_RENAME_S
            "#,
        )
        .unwrap();

    assert_eq!(result.0, "number");
    assert_eq!(result.1, 1223.0);
}
