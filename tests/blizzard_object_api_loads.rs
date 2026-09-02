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

fn object_api_dir() -> PathBuf {
    blizzard_ui_dir().join("Blizzard_ObjectAPI")
}

fn object_api_mainline_toc() -> PathBuf {
    object_api_dir().join("Blizzard_ObjectAPI_Mainline.toc")
}

const OBJECT_API_TOC_FILES: &[&str] = &[
    "Mainline/ContinuableContainer.lua",
    "Mainline/ItemLocation.lua",
    "Mainline/AsyncCallbackSystem.lua",
    "Mainline/ObjectCache.lua",
    "Mainline/Item.lua",
    "Mainline/Spell.lua",
    "Mainline/PlayerLocation.lua",
    "Mainline/CampaignChapter.lua",
    "Mainline/Campaign.lua",
    "Mainline/Quest.lua",
    "Mainline/UiMapPoint.lua",
    "Mainline/CovenantCalling.lua",
];

const PUBLIC_CONSTRUCTOR_TABLES: &[&str] = &[
    "Item",
    "Spell",
    "PlayerLocation",
    "ItemLocation",
    "UiMapPoint",
    "ContinuableContainer",
];

const PUBLIC_MIXIN_TABLES: &[&str] = &[
    "ItemMixin",
    "SpellMixin",
    "PlayerLocationMixin",
    "ItemLocationMixin",
    "CovenantCallingMixin",
    "AsyncCallbackSystemMixin",
];

const PUBLIC_CACHES: &[&str] = &["QuestCache", "CampaignCache", "CampaignChapterCache"];

const PUBLIC_EVENT_LISTENERS: &[&str] = &[
    "QuestEventListener",
    "ItemEventListener",
    "SpellEventListener",
];

const FILE_PRIVATE_LOCALS: &[&str] = &[
    "QuestMixin",
    "CampaignMixin",
    "CampaignChapterMixin",
    "permittedAPI",
    "CallingsUpdater",
];

const UNLISTED_FILE_GLOBALS: &[&str] = &["SpellBookItemLocation", "SpellBookItemLocationMixin"];

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
fn blizzard_object_api_find_toc_resolves_mainline_variant_only() {
    let resolved = find_toc_file(&object_api_dir()).expect("Blizzard_ObjectAPI TOC resolves");
    assert_eq!(
        resolved,
        object_api_mainline_toc(),
        "Blizzard_ObjectAPI ships exactly one `_Mainline.toc` and NO bare TOC. \
         `find_toc_file` at src/loader/mod.rs:67-70 prefers the `_Mainline.toc` variant first \
         and the bare `.toc` second, so the Mainline-suffixed TOC resolves successfully. The \
         Mainline-only naming reflects that the API is retail-exclusive — every file in the \
         body lives under `Mainline\\` and references retail-only C_* namespaces \
         (C_QuestInfoSystem, C_CampaignInfo, C_QuestLine.GetQuestLineQuests)"
    );

    let bare = object_api_dir().join("Blizzard_ObjectAPI.toc");
    assert!(
        !bare.exists(),
        "There must be NO bare `Blizzard_ObjectAPI.toc` at {} — the addon ships ONLY the \
         `_Mainline.toc` variant. The bare-TOC fallback at src/loader/mod.rs:69 is the second \
         lookup, so the absence is load-bearing: it locks the addon into the mainline-only \
         path",
        bare.display()
    );
}

#[test]
fn blizzard_object_api_toc_declares_mainline_only_with_colors_dependency() {
    let toc =
        TocFile::from_file(&object_api_mainline_toc()).expect("Blizzard_ObjectAPI TOC parses");
    assert!(
        !toc.is_load_on_demand(),
        "TOC OMITS `## LoadOnDemand:` so `is_load_on_demand()` returns false — the ObjectAPI \
         must be eager-loaded because every dependent UI addon (QuestLog, ItemAPI consumers, \
         SpellbookFrame) references `Item:CreateFromItemID`, `Spell:CreateFromSpellID`, \
         `QuestCache:Get`, etc. at module-top-level. LoD would force every consumer to call \
         C_AddOns.LoadAddOn before resolving these constructors"
    );
    assert!(!toc.is_load_first());
    assert!(!toc.is_secure_env());

    assert!(
        toc.allows_screen(ScreenKind::Game),
        "`## AllowLoad: Game` must enable Game-screen discovery — `allows_screen` at \
         src/toc.rs:308 returns true for ScreenKind::Game when AllowLoad is `Game`"
    );
    assert!(
        !toc.allows_screen(ScreenKind::Login),
        "`## AllowLoad: Game` must NOT enable Login-screen discovery — the ObjectAPI is \
         in-game-only because every constructor (Item, Spell, PlayerLocation, QuestCache:Get) \
         references C_* namespaces that only resolve when the player has a character in-world. \
         Glue screens have no UnitGUID, no PlayerLocation, no QuestLog"
    );
    assert!(
        !toc.allows_screen(ScreenKind::CharacterSelect),
        "`## AllowLoad: Game` must NOT enable CharacterSelect — character-row data uses a \
         different API surface (CharacterServices) that does not need ObjectAPI's runtime \
         caches"
    );

    assert!(
        !toc.is_game_type_restricted(),
        "`## AllowLoadGameType: mainline` does NOT mark the addon as restricted from the \
         simulator's perspective — `is_game_type_restricted()` at src/toc.rs:294-302 returns \
         true ONLY for non-retail flavors (plunderstorm / classic / wrath / etc.). `mainline` \
         and `standard` are treated as retail-unrestricted because they ARE the retail flavor \
         this simulator targets. The metadata key is still present (the raw-bytes test pins \
         that), but the helper interprets `mainline` as the default flavor"
    );

    let deps = toc.dependencies();
    assert_eq!(
        deps,
        vec!["Blizzard_Colors"],
        "TOC must declare `## Dependencies: Blizzard_Colors`. Blizzard_Colors loads first to \
         publish ITEM_QUALITY_COLORS / BAG_ITEM_QUALITY_COLORS / DRESS_UP_FRAME_QUALITY_COLORS \
         lookup tables that ItemMixin's color-coding code paths consume at runtime. Note: \
         `dependencies()` at src/toc.rs:210-217 reads `Dependencies` as the second-priority \
         alias for the deps list — the canonical retail spelling here is the plural \
         `Dependencies` (rather than singular `RequiredDep`)"
    );

    assert!(
        toc.optional_deps().is_empty(),
        "Zero `## OptionalDeps:` — no soft sibling addons. ObjectAPI is a foundational \
         dependency for many addons but consumes only Blizzard_Colors itself"
    );
    assert!(
        toc.saved_variables().is_empty(),
        "Zero saved variables — every cache (QuestCache / CampaignCache / CampaignChapterCache) \
         is a process-lifetime memo of pure server data. Persisting would only stale the cache \
         across sessions"
    );
}

#[test]
fn blizzard_object_api_toc_declares_dependencies_in_raw_bytes() {
    let raw = std::fs::read_to_string(object_api_mainline_toc())
        .expect("Blizzard_ObjectAPI TOC reads utf-8");
    assert!(
        raw.contains("## DefaultState: enabled"),
        "TOC must declare `## DefaultState: enabled` — the addon is enabled by default in \
         the addon-manager UI. ObjectAPI is foundational, so disabling it would break every \
         addon that uses Item / Spell / QuestCache; the explicit `enabled` documents this"
    );
    assert!(
        raw.contains("## AllowLoad: Game"),
        "TOC must declare `## AllowLoad: Game` exactly — case-sensitive. Game-only screen \
         restriction lock"
    );
    assert!(
        raw.contains("## AllowLoadGameType: mainline"),
        "TOC must declare `## AllowLoadGameType: mainline` exactly. The mainline-only flavor \
         restriction lock — every file under Mainline\\ uses retail-only C_* namespaces"
    );
    assert!(
        raw.contains("## Dependencies: Blizzard_Colors"),
        "TOC must declare `## Dependencies: Blizzard_Colors` exactly. `dependencies()` at \
         src/toc.rs:210-217 reads RequiredDep / Dependencies / RequiredDeps as aliases; the \
         canonical retail spelling here is plural `Dependencies`"
    );
    assert!(
        !raw.contains("## LoadOnDemand:"),
        "TOC must NOT declare `## LoadOnDemand:` — the absence (rather than `LoadOnDemand: 0`) \
         is the canonical retail spelling for eager-loaded addons. Distinct from \
         Blizzard_Notification which uses the explicit `LoadOnDemand: 0` form"
    );
    assert!(
        !raw.contains("## SavedVariables"),
        "TOC must NOT declare any `## SavedVariables*` keys — caches are process-lifetime \
         memos of pure server data, no persistence"
    );
}

#[test]
fn blizzard_object_api_toc_lists_twelve_lua_files_under_mainline_subdir() {
    let toc =
        TocFile::from_file(&object_api_mainline_toc()).expect("Blizzard_ObjectAPI TOC parses");
    let listed: Vec<String> = toc
        .files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        listed, OBJECT_API_TOC_FILES,
        "TOC body must list exactly 12 Lua files in canonical order, all under the \
         `Mainline\\` subdirectory. The order encodes the dependency graph among the files: \
         ContinuableContainer first (no deps — base abstraction), then ItemLocation \
         (also no deps — value-type), then AsyncCallbackSystem (creates the three \
         EventListener globals consumed by Item/Spell/Quest), then ObjectCache (defines the \
         ObjectCache_Create helper used by Quest/Campaign/CampaignChapter caches), then Item / \
         Spell / PlayerLocation (3 constructor-table modules), then CampaignChapter / \
         Campaign / Quest (3 cache modules — each calls ObjectCache_Create at module-top so \
         the helper must already exist), then UiMapPoint (value-type), then CovenantCalling \
         last (depends on QuestCache from Quest.lua via the bounty.questID lookup pattern)"
    );

    for entry in &listed {
        assert!(
            entry.starts_with("Mainline/"),
            "TOC body entry `{entry}` must live under the Mainline/ subdirectory (the TOC \
             parser at src/toc.rs normalizes the raw `Mainline\\...` Windows-style backslash \
             separators to forward slashes when constructing PathBuf, so the comparison runs \
             against the normalized form). Every file is mainline-only — the Mainline/ \
             prefix mirrors the AllowLoadGameType: mainline metadata; flavor-split addons \
             typically organize files into flavor-named subdirectories rather than per-file \
             [AllowLoadGameType] annotations"
        );
        assert!(
            entry.ends_with(".lua"),
            "TOC body entry `{entry}` must be a Lua file — ObjectAPI ships ZERO XML files. \
             It is a pure data/logic addon (constructor tables + mixins + caches + event \
             listeners), no UI elements"
        );
    }
}

#[test]
fn blizzard_object_api_appears_in_game_screen_eager_discovery_only() {
    let ui = blizzard_ui_dir();

    let game_addons = discover_blizzard_addons_for_screen(&ui, ScreenKind::Game);
    let in_game = game_addons
        .iter()
        .any(|(name, _)| name == "Blizzard_ObjectAPI");
    assert!(
        in_game,
        "Blizzard_ObjectAPI must auto-discover on Game-screen — eager (no LoadOnDemand) AND \
         AllowLoad: Game makes `discover_blizzard_addons_for_screen(Game)` include the addon"
    );

    for screen in [
        ScreenKind::Login,
        ScreenKind::CharacterSelect,
        ScreenKind::CharacterCreate,
    ] {
        let addons = discover_blizzard_addons_for_screen(&ui, screen);
        let found = addons.iter().any(|(name, _)| name == "Blizzard_ObjectAPI");
        assert!(
            !found,
            "Blizzard_ObjectAPI must NOT auto-discover on screen {screen:?} — `## AllowLoad: \
             Game` restricts discovery to ScreenKind::Game only. ObjectAPI's constructors \
             dereference C_* namespaces that only resolve in-world (UnitGUID, QuestLog, \
             SpellBook), so glue-screen loading would emit immediate runtime errors"
        );
    }
}

prefork_full_ui_case! {
fn blizzard_object_api_loads_without_addon_specific_lua_errors(env: &WowLuaEnv) {

    let load_errors: Vec<String> = env
        .state()
        .borrow()
        .lua_errors
        .iter()
        .filter(|message| {
            message.contains("Blizzard_ObjectAPI")
                || message.contains("ItemMixin")
                || message.contains("SpellMixin")
                || message.contains("ItemLocationMixin")
                || message.contains("PlayerLocationMixin")
                || message.contains("QuestCache")
                || message.contains("CampaignCache")
                || message.contains("ObjectCache_Create")
                || message.contains("AsyncCallbackSystem")
                || message.contains("EventListener")
                || message.contains("CovenantCalling")
        })
        .cloned()
        .collect();
    assert!(
        load_errors.is_empty(),
        "Blizzard_ObjectAPI emitted addon-specific Lua errors during load:\n  {}",
        load_errors.join("\n  ")
    );
}
}

prefork_full_ui_case! {
fn blizzard_object_api_is_addon_loaded_after_eager_sweep(env: &WowLuaEnv) {

    let loaded: bool = env
        .eval("return C_AddOns.IsAddOnLoaded('Blizzard_ObjectAPI')")
        .expect("IsAddOnLoaded probe succeeds");
    assert!(
        loaded,
        "C_AddOns.IsAddOnLoaded('Blizzard_ObjectAPI') must return true after the eager \
         Game-screen sweep — no LoadOnDemand puts the addon in the eager set, no explicit \
         load_addon call needed"
    );
}
}

prefork_full_ui_case! {
fn blizzard_object_api_publishes_six_constructor_tables(env: &WowLuaEnv) {

    for table in PUBLIC_CONSTRUCTOR_TABLES {
        let kind: String = env
            .eval(&format!("return type(_G.{table})"))
            .unwrap_or_else(|err| panic!("type(_G.{table}) probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "_G.{table} must publish as a table — ObjectAPI exposes 6 constructor-style \
             tables that own static factory methods: `Item:CreateFromItemID(id)`, \
             `Item:CreateFromItemLocation(loc)` (Item.lua line 1); \
             `Spell:CreateFromSpellID(id)` (Spell.lua line 1); \
             `PlayerLocation:CreateFromGUID(g)`, `PlayerLocation:CreateFromUnit(u)` \
             (PlayerLocation.lua line 1); `ItemLocation:CreateEmpty()`, \
             `ItemLocation:CreateFromBagAndSlot(...)` (ItemLocation.lua line 1); \
             `UiMapPoint.CreateFromCoordinates(...)`, \
             `UiMapPoint.CreateFromVector2D(...)` (UiMapPoint.lua line 1); \
             `ContinuableContainer:AddContinuable(c)`, \
             `ContinuableContainer:ContinueOnLoad(cb)` (ContinuableContainer.lua line 1). \
             Each constructor calls `CreateFromMixins(<TableName>Mixin)` on the corresponding \
             mixin table"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_object_api_publishes_six_mixin_tables(env: &WowLuaEnv) {

    for mixin in PUBLIC_MIXIN_TABLES {
        let kind: String = env
            .eval(&format!("return type(_G.{mixin})"))
            .unwrap_or_else(|err| panic!("type(_G.{mixin}) probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "_G.{mixin} must publish as a table — ObjectAPI exposes 6 mixin tables that own \
             instance methods. ItemMixin owns ContinueOnLoad / GetItemID / GetItemName / \
             GetItemQuality / IsItemEmpty / etc. (Item.lua); SpellMixin owns SetSpellID / \
             GetSpellID / GetSpellName / etc. (Spell.lua); PlayerLocationMixin owns SetGUID / \
             GetGUID / SetUnit / etc. (PlayerLocation.lua); ItemLocationMixin owns \
             SetBagAndSlot / GetBagAndSlot / IsValid / HasAnyLocation / etc. \
             (ItemLocation.lua); CovenantCallingMixin owns Init / GetIndex / IsActive / etc. \
             (CovenantCalling.lua); AsyncCallbackSystemMixin owns Init / AddCallback / \
             FireCallbacks / ClearCallbacks (AsyncCallbackSystem.lua). \
             `CreateFromMixins(<MixinTable>)` walks the mixin's keys and copies the methods \
             onto the new instance"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_object_api_publishes_three_object_caches(env: &WowLuaEnv) {

    for cache in PUBLIC_CACHES {
        let cache_kind: String = env
            .eval(&format!("return type(_G.{cache})"))
            .unwrap_or_else(|err| panic!("type(_G.{cache}) probe failed: {err}"));
        assert_eq!(
            cache_kind, "table",
            "_G.{cache} must publish as a table — ObjectCache_Create returns a table with \
             `objects = {{}}` and a `Get(self, key)` closure that lazily constructs an \
             instance via CreateFromMixins(mixin) + Init(key) and memoizes it. The 3 caches: \
             QuestCache (Quest.lua) memoizes Quest objects keyed by questID; CampaignCache \
             (Campaign.lua) memoizes Campaign objects keyed by campaignID; \
             CampaignChapterCache (CampaignChapter.lua) memoizes CampaignChapter objects \
             keyed by chapterID. Note: the underlying QuestMixin / CampaignMixin / \
             CampaignChapterMixin tables are file-private locals — only the cache objects are \
             public"
        );

        let get_kind: String = env
            .eval(&format!("return type(_G.{cache}.Get)"))
            .unwrap_or_else(|err| panic!("type(_G.{cache}.Get) probe failed: {err}"));
        assert_eq!(
            get_kind, "function",
            "_G.{cache}.Get must be a function — the closure created by ObjectCache_Create \
             at ObjectCache.lua line 5"
        );
    }

    let helper_kind: String = env
        .eval("return type(_G.ObjectCache_Create)")
        .expect("ObjectCache_Create probe succeeds");
    assert_eq!(
        helper_kind, "function",
        "_G.ObjectCache_Create must be a function — the factory at ObjectCache.lua line 1 is \
         globally callable so any addon can create its own caches keyed by arbitrary mixin"
    );
}
}

prefork_full_ui_case! {
fn blizzard_object_api_publishes_three_event_listener_globals(env: &WowLuaEnv) {

    for listener in PUBLIC_EVENT_LISTENERS {
        let kind: String = env
            .eval(&format!("return type(_G.{listener})"))
            .unwrap_or_else(|err| panic!("type(_G.{listener}) probe failed: {err}"));
        assert_eq!(
            kind, "table",
            "_G.{listener} must publish as a table — AsyncCallbackSystem.lua lines 110-112 \
             instantiate three async-callback listeners via the file-private CreateListener \
             helper: ItemEventListener (ASYNC_ITEM, listens for ITEM_DATA_LOAD_RESULT, \
             accessor C_Item.RequestLoadItemDataByID), SpellEventListener (ASYNC_SPELL, \
             SPELL_DATA_LOAD_RESULT, C_Spell.RequestLoadSpellData), QuestEventListener \
             (ASYNC_QUEST, QUEST_DATA_LOAD_RESULT, C_QuestLog.RequestLoadQuestByID). Each \
             listener mixes in AsyncCallbackSystemMixin so it owns AddCallback / \
             AddCancelableCallback / ClearCallbacks / FireCallbacks. ItemMixin / SpellMixin / \
             QuestMixin call AddCallback during Init to register data-load completion \
             callbacks"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_object_api_publishes_async_callback_api_type_constants(env: &WowLuaEnv) {

    let kind: String = env
        .eval("return type(_G.AsyncCallbackAPIType)")
        .expect("AsyncCallbackAPIType probe succeeds");
    assert_eq!(
        kind, "table",
        "_G.AsyncCallbackAPIType must publish as a table — AsyncCallbackSystem.lua lines 9-13 \
         declare `AsyncCallbackAPIType = {{ ASYNC_QUEST = 1, ASYNC_ITEM = 2, ASYNC_SPELL = 3 \
         }};`. This is the addon-extends-Enum-style pattern (though using a plain global \
         rather than the engine's Enum table). The integer values key the file-private \
         permittedAPI dispatch table that maps each apiType to its event/accessor pair"
    );

    for (name, expected) in [("ASYNC_QUEST", 1i64), ("ASYNC_ITEM", 2), ("ASYNC_SPELL", 3)] {
        let actual: i64 = env
            .eval(&format!("return _G.AsyncCallbackAPIType.{name}"))
            .unwrap_or_else(|err| panic!("AsyncCallbackAPIType.{name} probe failed: {err}"));
        assert_eq!(
            actual, expected,
            "AsyncCallbackAPIType.{name} must equal {expected} — the 1/2/3 ordering keys the \
             permittedAPI dispatch table at AsyncCallbackSystem.lua lines 16-20. Reordering \
             would silently swap which event/accessor pair each apiType uses"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_object_api_does_not_leak_file_private_locals_as_globals(env: &WowLuaEnv) {

    for private in FILE_PRIVATE_LOCALS {
        let kind: String = env
            .eval(&format!("return type(_G.{private})"))
            .unwrap_or_else(|err| panic!("type(_G.{private}) probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "_G.{private} must be nil — these are file-scoped `local` declarations that must \
             not escape to `_G`. QuestMixin (Quest.lua line 1), CampaignMixin (Campaign.lua \
             line 1), CampaignChapterMixin (CampaignChapter.lua line 1) are kept private so \
             only the cache objects (QuestCache / CampaignCache / CampaignChapterCache) form \
             the public surface — instances are constructed exclusively through `cache:Get(\
             id)` rather than directly via `CreateFromMixins(QuestMixin)`. permittedAPI \
             (AsyncCallbackSystem.lua line 15) is the dispatch table mapping each apiType to \
             its event/accessor; keeping it private prevents arbitrary callers from binding \
             new event/accessor pairs (the comment at line 6 says \"the API is managed so that \
             arbitrary query functions cannot be executed\"). CallingsUpdater \
             (CovenantCalling.lua line 62) is a singleton CreateFrame(\"Frame\") that owns \
             COVENANT_CALLINGS_UPDATED + QUEST_TURNED_IN handlers; keeping it private \
             prevents addons from rebinding its event handlers"
        );
    }
}
}

prefork_full_ui_case! {
fn blizzard_object_api_does_not_load_unlisted_spell_book_item_location_file(env: &WowLuaEnv) {

    for global in UNLISTED_FILE_GLOBALS {
        let kind: String = env
            .eval(&format!("return type(_G.{global})"))
            .unwrap_or_else(|err| panic!("type(_G.{global}) probe failed: {err}"));
        assert_eq!(
            kind, "nil",
            "_G.{global} must be nil — `Mainline\\SpellBookItemLocation.lua` exists in the \
             addon directory but is NOT listed in the TOC body. The Lua loader only runs \
             files explicitly listed in the TOC (src/loader/lua_file.rs walks `toc.files`), \
             so an on-disk-but-unlisted file is dead weight — its globals never publish. This \
             pins the asymmetry: every other constructor-table file (Item / Spell / \
             PlayerLocation / ItemLocation / UiMapPoint) IS listed and DOES publish; \
             SpellBookItemLocation is intentionally absent from the load set, suggesting it \
             is either a work-in-progress or a deprecated module that retail still ships on \
             disk"
        );
    }

    let unlisted_file = object_api_dir()
        .join("Mainline")
        .join("SpellBookItemLocation.lua");
    assert!(
        unlisted_file.exists(),
        "Mainline\\SpellBookItemLocation.lua must exist on disk at {} — the file ships with \
         the addon (so the TOC could in principle add it later) even though the TOC does not \
         list it. Asserting both the file's existence AND its absence from the load set pins \
         the deliberate-omission interpretation: this is not a missing file, it is an \
         opt-out",
        unlisted_file.display()
    );
}
}
