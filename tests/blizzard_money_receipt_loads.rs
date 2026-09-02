#![cfg(feature = "client-retail")]
use std::path::PathBuf;

use wow_ui_sim::loader::{discover_blizzard_addons_for_screen, find_toc_file, load_addon};
use wow_ui_sim::lua_api::WowLuaEnv;
use wow_ui_sim::screen::ScreenKind;
use wow_ui_sim::startup::fire_startup_events_for_screen;
use wow_ui_sim::toc::TocFile;

fn blizzard_ui_dir() -> PathBuf {
    wow_ui_sim::client_profile::blizzard_ui_addons_dir_under(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
}

fn money_receipt_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_MoneyReceipt")
}

fn money_receipt_toc() -> PathBuf {
    money_receipt_dir().join("Blizzard_MoneyReceipt.toc")
}

const MONEY_RECEIPT_TOC_FILES: &[&str] = &["Blizzard_MoneyReceipt.lua"];

const FILE_PRIVATE_LOCALS_THAT_MUST_NOT_LEAK: &[&str] = &["ReceiptMixin", "ReceiptDisplay"];

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

#[test]
fn blizzard_money_receipt_find_toc_resolves_bare_variant() {
    let resolved = find_toc_file(&money_receipt_dir()).expect("Blizzard_MoneyReceipt TOC resolves");
    assert_eq!(
        resolved,
        money_receipt_toc(),
        "Blizzard_MoneyReceipt ships exactly one bare TOC — no `_Mainline.toc` and no \
         `_Classic.toc`. The receipt-display feature is a cross-flavor concern (every WoW \
         client surfaces a `+12g 34s 56c` chat-line summary on merchant / mailbox close), \
         so the addon ships a single bare TOC. `find_toc_file` resolves the bare TOC after \
         the `_Mainline.toc` lookup misses"
    );

    let mainline = money_receipt_dir().join("Blizzard_MoneyReceipt_Mainline.toc");
    assert!(
        !mainline.exists(),
        "There must be NO `_Mainline.toc` at {} — Blizzard_MoneyReceipt is flavor-agnostic \
         at the TOC layer. The `## AllowLoadGameType: standard` directive in the bare TOC \
         carries the cross-flavor signal at parse time",
        mainline.display()
    );
}

#[test]
fn blizzard_money_receipt_toc_declares_minimal_envelope_with_standard_game_type() {
    let toc = TocFile::from_file(&money_receipt_toc()).expect("Blizzard_MoneyReceipt TOC parses");
    assert!(
        !toc.is_load_on_demand(),
        "TOC omits `## LoadOnDemand:` — the receipt-display addon eager-loads. The merchant / \
         mailbox interaction events the addon listens for can fire at any time post-login, so \
         deferred load is not viable; the receipt-tracking handler must be wired before the \
         first PLAYER_INTERACTION_MANAGER_FRAME_SHOW event"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());
    assert!(
        toc.dependencies().is_empty(),
        "Zero `## Dependencies:` — the receipt-display addon is dependency-free. It calls 4 \
         globally-published surfaces (GetMoney, GetMoneyString, ChatFrameUtil.\
         DisplaySystemMessageInPrimary, Mixin / CreateFrame / Enum.PlayerInteractionType) — \
         none of which are owned by sibling addons; they are all foundational globals \
         published before any addon Lua runs"
    );
    assert!(toc.optional_deps().is_empty());
    assert!(
        toc.saved_variables().is_empty(),
        "Zero saved variables — the receipt-display addon has no client-side persisted state. \
         The `startingMoney` field is held in-memory on the file-private ReceiptDisplay frame \
         and is reset to nil after every EndTracking call"
    );
    assert!(
        !toc.is_game_type_restricted(),
        "TOC declares `## AllowLoadGameType: standard` — `is_game_type_restricted()` at \
         src/toc.rs:294-302 treats both `mainline` and `standard` as retail-unrestricted \
         (returns false). The `standard` value is the older spelling for what newer addons \
         spell `mainline`; the simulator normalizes both to the same retail-unrestricted \
         outcome"
    );
}

#[test]
fn blizzard_money_receipt_toc_declares_standard_game_type_in_raw_bytes() {
    let raw = std::fs::read_to_string(money_receipt_toc())
        .expect("Blizzard_MoneyReceipt TOC reads as utf-8");
    assert!(
        raw.contains("## AllowLoadGameType: standard"),
        "TOC must declare `## AllowLoadGameType: standard` exactly. Most modern Blizzard \
         addons use `mainline`; the older `standard` spelling survives in a small set of \
         flavor-agnostic addons. The simulator's `is_game_type_restricted()` matcher tolerates \
         either spelling — both return false (retail-unrestricted) — but the raw token is a \
         visible reminder that the parser must accept both"
    );
    assert!(
        !raw.contains("## DefaultState"),
        "TOC must NOT declare `## DefaultState:` — receipt-display omits the directive. The \
         simulator's discovery layer doesn't inspect DefaultState; an addon enters the eager \
         set when it parses, has no AllowLoadGameType mismatch, allows the active screen, \
         and is non-LOD"
    );
    assert!(
        !raw.contains("## AllowLoad:"),
        "TOC must NOT declare `## AllowLoad:` (the colon variant — distinct from the \
         `## AllowLoadGameType:` directive that is present). When `## AllowLoad:` is omitted, \
         `allows_screen` at src/toc.rs:311 defaults to `screen == ScreenKind::Game` — \
         Game-only auto-discovery, NOT every-screen. The receipt-display addon is Game-only \
         because PLAYER_INTERACTION_MANAGER_FRAME_SHOW / HIDE only fires post-PLAYER_\
         ENTERING_WORLD; the merchant / mailbox interaction surface does not exist on glue \
         screens"
    );
}

#[test]
fn blizzard_money_receipt_toc_lists_single_lua_file() {
    let toc = TocFile::from_file(&money_receipt_toc()).expect("Blizzard_MoneyReceipt TOC parses");
    let listed: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        listed, MONEY_RECEIPT_TOC_FILES,
        "TOC body must list exactly 1 file — Blizzard_MoneyReceipt.lua at the addon root. \
         No XML, no flavor subdirectory, no shared / mainline split. The 78-line addon is \
         one of the smallest in the Blizzard tree: a single Lua file defining a file-local \
         ReceiptMixin and a single anonymous frame instance"
    );
}

#[test]
fn blizzard_money_receipt_auto_discovers_on_game_only() {
    let game_addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), ScreenKind::Game);
    let game_found = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_MoneyReceipt");
    assert!(
        game_found,
        "Blizzard_MoneyReceipt must auto-discover on the Game screen — receipt tracking \
         depends on merchant / mailbox interaction events that fire post-PLAYER_ENTERING_WORLD"
    );

    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&blizzard_ui_dir(), screen);
        let found = addons
            .iter()
            .any(|(name, _)| name == "Blizzard_MoneyReceipt");
        assert!(
            !found,
            "Blizzard_MoneyReceipt must NOT auto-discover on glue screens. With \
             `## AllowLoad:` omitted from the TOC, `allows_screen` at src/toc.rs:311 \
             defaults to Game-only (`screen == ScreenKind::Game`). The receipt addon has \
             no role on Login / CharacterSelect / CharacterCreate because the merchant / \
             mailbox interaction events it tracks are server-driven and only fire after \
             world entry. (Screen tested: {screen:?})"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_money_receipt_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_MoneyReceipt")
                || message.contains("ReceiptMixin")
                || message.contains("ReceiptDisplay")
                || message.contains("GENERIC_MONEY_GAINED_RECEIPT")
                || message.contains("CRAFTINGORDERS_DISPLAY_CRAFTER_FULFILLED_MSG")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_MoneyReceipt emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_money_receipt_is_addon_loaded_after_auto_discovery(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_MoneyReceipt')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_MoneyReceipt') must return true after the eager \
         auto-discovery sweep — proves the receipt-display addon registers with the \
         loaded-set during the standard Game-screen boot pipeline, no explicit load_addon \
         call required"
    );
}
}

prefork_full_ui_case! {
fn blizzard_money_receipt_does_not_leak_file_local_mixin_or_frame_globals(env: &WowLuaEnv) {
    for symbol in FILE_PRIVATE_LOCALS_THAT_MUST_NOT_LEAK {
        let kind: String = env
            .eval(&format!("return type(_G.{symbol})"))
            .unwrap_or_else(|err| panic!("type(_G.{symbol}) probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "_G.{symbol} must remain nil — Blizzard_MoneyReceipt.lua line 1 declares \
             `local ReceiptMixin = {{}}` and line 73 declares `local ReceiptDisplay = \
             Mixin(CreateFrame('FRAME'), ReceiptMixin)`. Both are file-private; neither has \
             a `_G.<name> = ...` assignment. The receipt-display surface is fully \
             encapsulated — third-party addons MUST go through PLAYER_INTERACTION_MANAGER_\
             FRAME_SHOW / HIDE event subscription to observe receipt-tracking, never by \
             reaching into the private mixin"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_money_receipt_surface_globals_are_published_post_load(env: &WowLuaEnv) {

    for func in &["Mixin", "CreateFrame", "GetMoney", "GetMoneyString"] {
        let kind: String = env
            .eval(&format!("return type(_G.{func})"))
            .unwrap_or_else(|err| panic!("type({func}) probe failed: {err}"));
        assert_eq!(
            kind, "function",
            "Surface global `{func}` must publish at `_G` as a function before \
             Blizzard_MoneyReceipt.lua runs. The 78-line addon's last 2 lines call \
             `Mixin(CreateFrame('FRAME'), ReceiptMixin):OnLoad()` directly — if any of \
             these is nil, the file fails to load. GetMoney / GetMoneyString are called \
             inside the mixin methods (BeginTracking / Display / OnEvent crafting branch)"
        );
    }

    let chat_kind: String = env
        .eval("return type(ChatFrameUtil.DisplaySystemMessageInPrimary)")
        .expect("ChatFrameUtil.DisplaySystemMessageInPrimary probe succeeds");
    assert_eq!(
        chat_kind, "function",
        "ChatFrameUtil.DisplaySystemMessageInPrimary must publish as a function — the \
         receipt-display addon calls it from both EndTracking (the `+12g 34s 56c` summary) \
         and the CRAFTINGORDERS_DISPLAY_CRAFTER_FULFILLED_MSG branch (the crafting-order \
         tip-received chat line)"
    );

    for variant in &["Merchant", "MailInfo"] {
        let kind: String = env
            .eval(&format!(
                "return type(Enum.PlayerInteractionType.{variant})"
            ))
            .unwrap_or_else(|err| panic!("Enum.PlayerInteractionType.{variant} probe: {err}"));
        assert_eq!(
            kind, "number",
            "Enum.PlayerInteractionType.{variant} must be a numeric enum value. The \
             addon's `relevantInteractionTypes` table at line 17 keys against these two \
             interaction kinds — Merchant for vendor-window receipts, MailInfo for mailbox \
             tracking. If either enum is nil, the relevantInteractionTypes table loses the \
             keying and ReceiptMixin:OnEvent silently no-ops on every interaction"
        );
    }
}
}
