//! Tests for the `C_WowTokenPublic` SimState-backed surface.
//! Covers the commerce-status / market-price reads, the buy + market
//! refresh event dispatch, and the canonical-token id check.

use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::lua_api::state::TokenAuctionInfo;

const WOW_TOKEN_ITEM_ID: i32 = 122270;
const TOKEN_RESULT_SUCCESS: i32 = 0;

const BUY_LISTENER: &str = r#"
    local buy_count, last_buy_result = 0, -1
    local buy_listener = CreateFrame("Frame")
    buy_listener:RegisterEvent("TOKEN_BUY_RESULT")
    buy_listener:SetScript("OnEvent", function(_, _, result)
        buy_count = buy_count + 1
        last_buy_result = result or -1
    end)
"#;

const PRICE_LISTENER: &str = r#"
    local price_count, last_price_result = 0, -1
    local price_listener = CreateFrame("Frame")
    price_listener:RegisterEvent("TOKEN_MARKET_PRICE_UPDATED")
    price_listener:SetScript("OnEvent", function(_, _, result)
        price_count = price_count + 1
        last_price_result = result or -1
    end)
"#;

#[test]
fn get_commerce_system_status_returns_default_commerce_on_balance_off() {
    let env = WowLuaEnv::new().unwrap();
    let (commerce, poll_seconds, balance): (bool, i32, bool) = env
        .eval("return C_WowTokenPublic.GetCommerceSystemStatus()")
        .unwrap();
    assert!(commerce);
    assert_eq!(poll_seconds, 60);
    assert!(!balance);
}

#[test]
fn get_commerce_system_status_reflects_state_overrides() {
    let env = WowLuaEnv::new().unwrap();
    {
        let mut sim = env.state().borrow_mut();
        sim.wow_token.commerce_enabled = false;
        sim.wow_token.poll_seconds = 30;
        sim.wow_token.balance_enabled = true;
    }
    let (commerce, poll_seconds, balance): (bool, i32, bool) = env
        .eval("return C_WowTokenPublic.GetCommerceSystemStatus()")
        .unwrap();
    assert!(!commerce);
    assert_eq!(poll_seconds, 30);
    assert!(balance);
}

#[test]
fn get_current_market_price_returns_state_value_twice() {
    let env = WowLuaEnv::new().unwrap();
    env.state().borrow_mut().wow_token.current_market_price = 250_000;
    let (first, second): (i64, i64) = env
        .eval("return C_WowTokenPublic.GetCurrentMarketPrice()")
        .unwrap();
    assert_eq!(first, 250_000);
    assert_eq!(second, 250_000);
}

#[test]
fn get_current_market_price_default_is_zero() {
    let env = WowLuaEnv::new().unwrap();
    let (first, second): (i64, i64) = env
        .eval("return C_WowTokenPublic.GetCurrentMarketPrice()")
        .unwrap();
    assert_eq!(first, 0);
    assert_eq!(second, 0);
}

#[test]
fn get_guaranteed_price_returns_state_value() {
    let env = WowLuaEnv::new().unwrap();
    env.state().borrow_mut().wow_token.guaranteed_price = 199_000;
    let price: i64 = env
        .eval("return C_WowTokenPublic.GetGuaranteedPrice()")
        .unwrap();
    assert_eq!(price, 199_000);
}

#[test]
fn buy_token_increments_owned_count_and_fires_success_event() {
    let env = WowLuaEnv::new().unwrap();
    let script = format!(
        r#"
        {listener}
        C_WowTokenPublic.BuyToken()
        return buy_count, last_buy_result
        "#,
        listener = BUY_LISTENER,
    );
    let (count, result): (i32, i32) = env.eval(&script).unwrap();
    assert_eq!(count, 1);
    assert_eq!(result, TOKEN_RESULT_SUCCESS);
    assert_eq!(env.state().borrow().wow_token.owned_token_count, 1);
}

#[test]
fn buy_token_called_twice_increments_count_each_time() {
    let env = WowLuaEnv::new().unwrap();
    env.eval::<()>("C_WowTokenPublic.BuyToken(); C_WowTokenPublic.BuyToken()")
        .unwrap();
    assert_eq!(env.state().borrow().wow_token.owned_token_count, 2);
}

#[test]
fn update_market_price_fires_token_market_price_updated_with_success() {
    let env = WowLuaEnv::new().unwrap();
    let script = format!(
        r#"
        {listener}
        C_WowTokenPublic.UpdateMarketPrice()
        return price_count, last_price_result
        "#,
        listener = PRICE_LISTENER,
    );
    let (count, result): (i32, i32) = env.eval(&script).unwrap();
    assert_eq!(count, 1);
    assert_eq!(result, TOKEN_RESULT_SUCCESS);
}

#[test]
fn update_token_count_is_a_silent_no_op() {
    let env = WowLuaEnv::new().unwrap();
    env.state().borrow_mut().wow_token.owned_token_count = 7;
    env.eval::<()>("C_WowTokenPublic.UpdateTokenCount()")
        .unwrap();
    assert_eq!(env.state().borrow().wow_token.owned_token_count, 7);
}

#[test]
fn update_listed_auctionable_tokens_is_a_silent_no_op() {
    let env = WowLuaEnv::new().unwrap();
    env.eval::<()>("C_WowTokenPublic.UpdateListedAuctionableTokens()")
        .unwrap();
}

#[cfg(feature = "client-mists")]
#[test]
fn get_num_listed_auctionable_tokens_reflects_state() {
    let env = WowLuaEnv::new().unwrap();
    env.state()
        .borrow_mut()
        .wow_token
        .listed_auctionable
        .push(TokenAuctionInfo {
            auction_id: 77,
            price: 250_000,
            owner: "Seller".to_string(),
        });

    let count: i32 = env
        .eval("return C_WowTokenPublic.GetNumListedAuctionableTokens()")
        .unwrap();
    assert_eq!(count, 1);
}

#[cfg(feature = "client-mists")]
#[test]
fn get_listed_auctionable_token_info_returns_row_values() {
    let env = WowLuaEnv::new().unwrap();
    env.state()
        .borrow_mut()
        .wow_token
        .listed_auctionable
        .push(TokenAuctionInfo {
            auction_id: 77,
            price: 250_000,
            owner: "Seller".to_string(),
        });

    let (auction_id, price): (i64, i64) = env
        .eval("return C_WowTokenPublic.GetListedAuctionableTokenInfo(1)")
        .unwrap();
    assert_eq!(auction_id, 77);
    assert_eq!(price, 250_000);
}

#[test]
fn is_auctionable_wow_token_true_for_canonical_id() {
    let env = WowLuaEnv::new().unwrap();
    let script = format!("return C_WowTokenPublic.IsAuctionableWowToken({WOW_TOKEN_ITEM_ID})");
    let ok: bool = env.eval(&script).unwrap();
    assert!(ok);
}

#[test]
fn is_auctionable_wow_token_false_for_other_item_ids() {
    let env = WowLuaEnv::new().unwrap();
    let ok: bool = env
        .eval("return C_WowTokenPublic.IsAuctionableWowToken(12345)")
        .unwrap();
    assert!(!ok);
}
