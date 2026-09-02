#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::discover_blizzard_addons_for_screen;
use wow_ui_sim::loader::load_addon;
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn load_full_game_ui() -> WowLuaEnv {
    let env = WowLuaEnv::new().expect("Failed to create Lua environment");
    env.set_screen_size(1024.0, 768.0);
    env.set_screen_mode(ScreenKind::Game);

    {
        let mut state = env.state().borrow_mut();
        state.addon_base_paths = vec![blizzard_ui_dir()];
    }

    wow_ui_sim::xml::register_intrinsic_templates();

    let ui = blizzard_ui_dir();
    let addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    for (name, toc_path) in &addons {
        load_addon(&env.loader_env(), toc_path)
            .unwrap_or_else(|err| panic!("[load {name}] FAILED: {err}"));
    }

    env.apply_post_load_workarounds();
    fire_startup_events_for_screen(&env, ScreenKind::Game);
    env
}

prefork_full_ui_case! {
fn blizzard_chat_frame_util_exposes_auction_house_notification_text(env: &WowLuaEnv) {

    let helper_present: bool = env
        .eval(
            "return type(ChatFrameUtil) == 'table' \
                and type(ChatFrameUtil.GetAuctionHouseNotificationText) == 'function'",
        )
        .expect("ChatFrameUtil query should succeed");
    assert!(
        helper_present,
        "ChatFrameUtil.GetAuctionHouseNotificationText should be defined after Blizzard_ChatFrameUtil load"
    );
}
}

prefork_full_ui_case! {
fn auction_house_notification_text_returns_strings_for_known_types(env: &WowLuaEnv) {

    let bid_placed_matches_constant: bool = env
        .eval(
            "return ChatFrameUtil.GetAuctionHouseNotificationText( \
                Enum.AuctionHouseNotification.BidPlaced, nil) == ERR_AUCTION_BID_PLACED \
             and type(ERR_AUCTION_BID_PLACED) == 'string'",
        )
        .expect("BidPlaced lookup should succeed");
    assert!(
        bid_placed_matches_constant,
        "BidPlaced should resolve to ERR_AUCTION_BID_PLACED constant"
    );

    let auction_won: String = env
        .eval(
            "return ChatFrameUtil.GetAuctionHouseNotificationText( \
                Enum.AuctionHouseNotification.AuctionWon, 'Linen Cloth')",
        )
        .expect("AuctionWon lookup should succeed");
    assert!(
        auction_won.contains("Linen Cloth"),
        "AuctionWon should format ERR_AUCTION_WON_S with the item name; got: {auction_won:?}"
    );

    let unknown_returns_empty: String = env
        .eval("return ChatFrameUtil.GetAuctionHouseNotificationText(-1, nil)")
        .expect("unknown type lookup should succeed");
    assert_eq!(
        unknown_returns_empty, "",
        "unknown notification type should return empty string"
    );
}
}
