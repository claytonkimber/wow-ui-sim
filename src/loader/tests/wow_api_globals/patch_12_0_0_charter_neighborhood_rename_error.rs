//! Focused retail 12.0.0 charter-neighborhood rename error constant test.

#![cfg(feature = "retail-12-0-0")]

use super::super::*;

#[test]
fn test_patch_12_0_0_le_game_err_charter_neighborhood_rename() {
    let env = WowLuaEnv::new().unwrap();
    let result: (String, f64) = env
        .eval(
            r#"
                return type(LE_GAME_ERR_CHARTER_NEIGHBORHOOD_RENAME),
                    LE_GAME_ERR_CHARTER_NEIGHBORHOOD_RENAME
            "#,
        )
        .unwrap();

    assert_eq!(result.0, "number");
    assert_eq!(result.1, 1224.0);
}
