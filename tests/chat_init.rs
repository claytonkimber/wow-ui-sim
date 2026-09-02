//! Integration tests for `src/lua_api/chat_init.rs`.
//!
//! These tests construct a bare WowLuaEnv (no full addon loading) and call the
//! chat_init helpers directly, asserting their observable side-effects via
//! `WowLuaEnv::eval`.

use wow_ui_sim::lua_api::WowLuaEnv;

fn env() -> WowLuaEnv {
    WowLuaEnv::new().expect("WowLuaEnv init")
}

// ── init_chat_type_colors ────────────────────────────────────────────────────

/// When ChatTypeInfo is missing, the function returns early without error.
#[test]
fn init_chat_type_colors_no_op_when_table_absent() {
    let env = env();
    env.exec("ChatTypeInfo = nil").unwrap();
    // Must not panic or error.
    wow_ui_sim::lua_api::chat_init::init_chat_type_colors(&env);
}

/// Entries that already have r/g/b set are not overwritten.
#[test]
fn init_chat_type_colors_preserves_existing_rgb() {
    let env = env();
    // Bootstrap seeds SYSTEM with r=1,g=1,b=0 — verify it is untouched.
    wow_ui_sim::lua_api::chat_init::init_chat_type_colors(&env);
    let (r, g, b): (f64, f64, f64) = env
        .eval("return ChatTypeInfo.SYSTEM.r, ChatTypeInfo.SYSTEM.g, ChatTypeInfo.SYSTEM.b")
        .unwrap();
    assert_eq!((r, g, b), (1.0, 1.0, 0.0));
}

/// Entries without r/g/b receive defaults from the table.
#[test]
fn init_chat_type_colors_fills_missing_entries() {
    let env = env();
    // Insert an entry without rgb.
    env.exec("ChatTypeInfo.SAY = { id = 2 }").unwrap();
    wow_ui_sim::lua_api::chat_init::init_chat_type_colors(&env);
    let (r, g, b): (f64, f64, f64) = env
        .eval("return ChatTypeInfo.SAY.r, ChatTypeInfo.SAY.g, ChatTypeInfo.SAY.b")
        .unwrap();
    // SAY default is {1,1,1}.
    assert_eq!((r, g, b), (1.0, 1.0, 1.0));
}

#[test]
fn init_chat_type_colors_ignores_inherited_rgb_defaults() {
    let env = env();
    env.exec(
        r#"
        local inheritedWhite = { r = 1, g = 1, b = 1 }
        ChatTypeInfo.GUILD = setmetatable({ id = 3 }, { __index = inheritedWhite })
        "#,
    )
    .unwrap();
    wow_ui_sim::lua_api::chat_init::init_chat_type_colors(&env);
    let (r, g, b): (f64, f64, f64) = env
        .eval("return ChatTypeInfo.GUILD.r, ChatTypeInfo.GUILD.g, ChatTypeInfo.GUILD.b")
        .unwrap();
    assert_eq!((r, g, b), (0.25, 1.0, 0.25));
}

#[test]
fn init_chat_type_colors_updates_proxy_entries() {
    let env = env();
    env.exec(
        r#"
        local proxy = { GUILD = { id = 3 } }
        ChatTypeInfo = setmetatable({}, { __index = proxy })
        "#,
    )
    .unwrap();
    wow_ui_sim::lua_api::chat_init::init_chat_type_colors(&env);
    let (r, g, b): (f64, f64, f64) = env
        .eval("return ChatTypeInfo.GUILD.r, ChatTypeInfo.GUILD.g, ChatTypeInfo.GUILD.b")
        .unwrap();
    assert_eq!((r, g, b), (0.25, 1.0, 0.25));
}

#[test]
fn init_chat_type_colors_replaces_placeholder_guild_white() {
    let env = env();
    env.exec(
        r#"
        local proxy = { GUILD = { id = 3, r = 1, g = 1, b = 1 } }
        ChatTypeInfo = setmetatable({}, { __index = proxy })
        "#,
    )
    .unwrap();
    wow_ui_sim::lua_api::chat_init::init_chat_type_colors(&env);
    let (r, g, b): (f64, f64, f64) = env
        .eval("return ChatTypeInfo.GUILD.r, ChatTypeInfo.GUILD.g, ChatTypeInfo.GUILD.b")
        .unwrap();
    assert_eq!((r, g, b), (0.25, 1.0, 0.25));
}

/// Unknown keys fall back to the white {1,1,1} default.
#[test]
fn init_chat_type_colors_unknown_key_falls_back_to_white() {
    let env = env();
    env.exec("ChatTypeInfo.UNKNOWN_CHAT_TYPE = { id = 99 }")
        .unwrap();
    wow_ui_sim::lua_api::chat_init::init_chat_type_colors(&env);
    let (r, g, b): (f64, f64, f64) = env
        .eval(
            "return ChatTypeInfo.UNKNOWN_CHAT_TYPE.r, \
                    ChatTypeInfo.UNKNOWN_CHAT_TYPE.g, \
                    ChatTypeInfo.UNKNOWN_CHAT_TYPE.b",
        )
        .unwrap();
    assert_eq!((r, g, b), (1.0, 1.0, 1.0));
}

// ── show_chat_frame ──────────────────────────────────────────────────────────

/// When ChatFrame1 is absent, show_chat_frame is a silent no-op.
#[test]
fn show_chat_frame_no_op_when_chat_frame1_absent() {
    let env = env();
    env.exec("ChatFrame1 = nil").unwrap();
    // Ensure ChatFrame1 is nil before calling the helper.
    let is_nil: bool = env.eval("return ChatFrame1 == nil").unwrap();
    assert!(is_nil, "test setup should clear ChatFrame1");
    // Must not panic or error.
    wow_ui_sim::lua_api::chat_init::show_chat_frame(&env);
}

/// When ChatFrame1 exists, show_chat_frame sets DEFAULT_CHAT_FRAME and
/// initialises _FakeChat with the four channel tables.
#[test]
fn show_chat_frame_sets_default_chat_frame_and_fake_chat() {
    let env = env();
    // Provide a minimal ChatFrame1 stub with the WoW frame methods we use.
    env.exec(
        r#"
        ChatFrame1 = CreateFrame("Frame", "ChatFrame1", UIParent)
        function ChatFrame1:_AddMessageSilent() end
        "#,
    )
    .unwrap();

    let edit_box_is_absent: bool = env.eval("return ChatFrame1EditBox == nil").unwrap();
    assert!(edit_box_is_absent, "test requires no ChatFrame1EditBox");

    wow_ui_sim::lua_api::chat_init::show_chat_frame(&env);

    // DEFAULT_CHAT_FRAME should now point to ChatFrame1.
    let same: bool = env.eval("return DEFAULT_CHAT_FRAME == ChatFrame1").unwrap();
    assert!(same, "DEFAULT_CHAT_FRAME should be ChatFrame1");

    // _FakeChat must be populated with all four channels.
    let general_count: i64 = env.eval("return #_FakeChat.msgs.general").unwrap();
    assert!(general_count > 0, "general messages should be non-empty");

    let trade_count: i64 = env.eval("return #_FakeChat.msgs.trade").unwrap();
    assert!(trade_count > 0, "trade messages should be non-empty");

    let say_count: i64 = env.eval("return #_FakeChat.msgs.say").unwrap();
    assert!(say_count > 0, "say messages should be non-empty");

    let guild_count: i64 = env.eval("return #_FakeChat.msgs.guild").unwrap();
    assert!(guild_count > 0, "guild messages should be non-empty");
}

/// _FakeChat:pick rotates through messages deterministically.
#[test]
fn fake_chat_pick_cycles_messages() {
    let env = env();
    env.exec(
        r#"
        ChatFrame1 = CreateFrame("Frame", "ChatFrame1", UIParent)
        function ChatFrame1:_AddMessageSilent() end
        "#,
    )
    .unwrap();
    wow_ui_sim::lua_api::chat_init::show_chat_frame(&env);

    // Pick the same channel twice — should return different messages on the
    // second call (assuming at least 2 messages exist, which the data guarantees).
    let msg1: String = env.eval("return (_FakeChat:pick('general'))").unwrap();
    let msg2: String = env.eval("return (_FakeChat:pick('general'))").unwrap();
    assert_ne!(
        msg1, msg2,
        "successive picks should cycle to the next message"
    );
}
