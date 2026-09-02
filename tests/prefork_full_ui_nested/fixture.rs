use wow_ui_sim::lua_api::WowLuaEnv;

#[cfg(feature = "client-retail")]
prefork_full_ui_case! {
    fn preloaded_parent_has_normal_game_startup(env: &WowLuaEnv) {
        let (ui_parent_exists, world_frame_exists, width, height): (bool, bool, f64, f64) = env
            .eval(
                "return UIParent ~= nil, WorldFrame ~= nil, GetScreenWidth(), GetScreenHeight()",
            )
            .expect("normal Game startup state should be queryable");

        assert!(ui_parent_exists, "preloaded Game UI should publish UIParent");
        assert!(world_frame_exists, "preloaded Game UI should publish WorldFrame");
        assert_eq!(width, 1024.0);
        assert_eq!(height, 768.0);
    }
}
