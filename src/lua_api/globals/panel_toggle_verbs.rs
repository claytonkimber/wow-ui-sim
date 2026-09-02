//! Panel-toggle verbs.
//!
//! Migrates 10 entries off `GLOBAL_NIL_STUBS` (`ToggleDropDownMenu` is
//! already registered from `create_frame/dropdown_api.rs`):
//!
//! | Verb                | Panel token    |
//! |---------------------|----------------|
//! | ToggleCharacter     | Character      |
//! | ToggleSpellBook     | SpellBook      |
//! | ToggleTalentFrame   | Talent         |
//! | ToggleQuestLog      | QuestLog       |
//! | ToggleWorldMap      | WorldMap       |
//! | ToggleFriendsFrame  | Friends        |
//! | ToggleGuildFrame    | Guild          |
//! | ToggleHelpFrame     | Help           |
//! | ToggleSocialPanel   | Social         |
//! | ToggleMinimap       | Minimap        |
//!
//! Each verb flips membership in `SimState.open_panels`. If a matching
//! Rust frame exists by canonical name (e.g. `CharacterFrame`), its
//! visibility is toggled in sync; otherwise the set is authoritative.
//!
//! Registered from `register_tail_globals` after `missing_surface`.

use crate::lua_api::methods::{
    borrow_state, borrow_state_mut, call_function_state, create_string, table_get,
};
use rilua::Val;
use rilua::vm::state::LuaState;
use rilua::{LuaApiMut, LuaResult};

/// (panel_token, companion_frame_name)
const PANELS: &[(&str, &str)] = &[
    ("Character", "CharacterFrame"),
    ("SpellBook", "SpellBookFrame"),
    ("Talent", "PlayerTalentFrame"),
    ("QuestLog", "QuestLogFrame"),
    ("WorldMap", "WorldMapFrame"),
    ("Friends", "FriendsFrame"),
    ("Guild", "GuildFrame"),
    ("Help", "HelpFrame"),
    ("Social", "FriendsFrame"),
    ("Minimap", "MinimapCluster"),
];

fn toggle_panel(state: &mut LuaState, panel: &'static str, frame: &'static str) -> LuaResult<()> {
    if try_toggle_panel_via_frame_method(state, frame)? {
        sync_open_panel_membership(state, panel, frame);
        return Ok(());
    }

    let is_now_open = {
        let mut st = borrow_state_mut(state)?;
        if st.open_panels.contains(panel) {
            st.open_panels.remove(panel);
            false
        } else {
            st.open_panels.insert(panel.to_string());
            true
        }
    };
    sync_frame_visibility(state, frame, is_now_open);
    Ok(())
}

fn try_toggle_player_spells_helper(
    state: &mut LuaState,
    helper_name: &str,
    args: &[Val],
) -> LuaResult<bool> {
    let global = Val::Table(state.global);
    let util = table_get(state, global, "PlayerSpellsUtil");
    let Val::Table(_) = util else {
        return Ok(false);
    };

    let helper = table_get(state, util, helper_name);
    let Val::Function(_) = helper else {
        return Ok(false);
    };

    match call_function_state(state, helper, args) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

fn try_load_addon(state: &mut LuaState, addon_name: &str) -> LuaResult<bool> {
    let global = Val::Table(state.global);
    let c_addons = table_get(state, global, "C_AddOns");
    let Val::Table(_) = c_addons else {
        return Ok(false);
    };

    let load_addon = table_get(state, c_addons, "LoadAddOn");
    let Val::Function(_) = load_addon else {
        return Ok(false);
    };

    let addon_name = create_string(state, addon_name);
    match call_function_state(state, load_addon, &[addon_name]) {
        Ok(result) => Ok(matches!(result, Val::Bool(true))),
        Err(_) => Ok(false),
    }
}

fn player_spells_surface_ready(state: &mut LuaState) -> bool {
    let global = Val::Table(state.global);
    for name in [
        "SelectableButtonMixin",
        "CreateAnchor",
        "TextureUtil",
        "SetClampedTextureRotation",
    ] {
        let value = table_get(state, global, name);
        if !matches!(value, Val::Table(_) | Val::Function(_)) {
            return false;
        }
    }
    true
}

fn try_toggle_panel_via_frame_method(state: &mut LuaState, frame_name: &str) -> LuaResult<bool> {
    let global = Val::Table(state.global);
    let frame = table_get(state, global, frame_name);
    let Val::Table(_) = frame else {
        return Ok(false);
    };

    let handler = table_get(state, frame, "HandleUserActionToggleSelf");
    let Val::Function(_) = handler else {
        return Ok(false);
    };

    let _ = call_function_state(state, handler, &[frame])?;
    Ok(true)
}

fn frame_visibility(state: &mut LuaState, frame_name: &str) -> Option<bool> {
    borrow_state(state).ok().and_then(|st| {
        st.widgets
            .get_id_by_name(frame_name)
            .and_then(|frame_id| st.widgets.get(frame_id).map(|frame| frame.visible))
    })
}

fn sync_open_panel_membership(state: &mut LuaState, panel: &str, frame_name: &str) {
    let is_open = frame_visibility(state, frame_name).unwrap_or(false);

    let Ok(mut st) = borrow_state_mut(state) else {
        return;
    };
    if is_open {
        st.open_panels.insert(panel.to_string());
    } else {
        st.open_panels.remove(panel);
    }
}

fn sync_frame_visibility(state: &mut LuaState, frame_name: &str, visible: bool) {
    let Ok(mut st) = borrow_state_mut(state) else {
        return;
    };
    let Some(frame_id) = st.widgets.get_id_by_name(frame_name) else {
        return;
    };
    st.set_frame_visible(frame_id, visible);
}

fn values_match(left: Val, right: Val) -> bool {
    match (left, right) {
        (Val::Nil, Val::Nil) => true,
        (Val::Bool(left), Val::Bool(right)) => left == right,
        (Val::Num(left), Val::Num(right)) => left == right,
        (Val::Str(left), Val::Str(right)) => left == right,
        (Val::Table(left), Val::Table(right)) => left == right,
        (Val::Function(left), Val::Function(right)) => left == right,
        (Val::Userdata(left), Val::Userdata(right)) => left == right,
        (Val::Thread(left), Val::Thread(right)) => left == right,
        (Val::LightUserdata(left), Val::LightUserdata(right)) => left == right,
        _ => false,
    }
}

macro_rules! define_toggle {
    ($fn_name:ident, $panel:literal, $frame:literal) => {
        fn $fn_name(state: &mut LuaState) -> LuaResult<u32> {
            toggle_panel(state, $panel, $frame)?;
            Ok(0)
        }
    };
}

define_toggle!(toggle_character, "Character", "CharacterFrame");
fn toggle_spell_book(state: &mut LuaState) -> LuaResult<u32> {
    let global = Val::Table(state.global);
    if matches!(table_get(state, global, "PlayerSpellsFrame"), Val::Nil) {
        let _ = try_load_addon(state, "Blizzard_PlayerSpells")?;
    }

    let was_visible = frame_visibility(state, "PlayerSpellsFrame");
    let expected_visibility = !was_visible.unwrap_or(false);
    let _ = try_toggle_player_spells_helper(state, "ToggleSpellBookFrame", &[])?;
    let helper_toggled_frame =
        frame_visibility(state, "PlayerSpellsFrame") == Some(expected_visibility);
    if helper_toggled_frame {
        sync_open_panel_membership(state, "SpellBook", "PlayerSpellsFrame");
    } else {
        toggle_panel(state, "SpellBook", "PlayerSpellsFrame")?;
    }
    Ok(0)
}

fn toggle_talent_frame(state: &mut LuaState) -> LuaResult<u32> {
    if !player_spells_surface_ready(state)
        || !try_toggle_player_spells_helper(state, "ToggleClassTalentFrame", &[])?
    {
        toggle_panel(state, "Talent", "PlayerTalentFrame")?;
    }
    Ok(0)
}

fn toggle_loadable_panel(
    state: &mut LuaState,
    addon_name: &str,
    panel: &'static str,
    frame: &'static str,
) -> LuaResult<u32> {
    let global = Val::Table(state.global);
    if matches!(table_get(state, global, frame), Val::Nil) {
        let _ = try_load_addon(state, addon_name)?;
    }
    toggle_panel(state, panel, frame)?;
    Ok(0)
}

fn collections_journal_is_shown(state: &mut LuaState) -> bool {
    borrow_state(state)
        .ok()
        .and_then(|st| {
            st.widgets
                .get_id_by_name("CollectionsJournal")
                .and_then(|frame_id| st.widgets.get(frame_id).map(|frame| frame.visible))
        })
        .unwrap_or(false)
}

fn collections_journal_tab_matches(
    state: &mut LuaState,
    frame: Val,
    tab_index: Val,
) -> LuaResult<bool> {
    if matches!(tab_index, Val::Nil) {
        return Ok(true);
    }

    let global = Val::Table(state.global);
    let get_selected_tab = table_get(state, global, "PanelTemplates_GetSelectedTab");
    let Val::Function(_) = get_selected_tab else {
        return Ok(false);
    };

    let selected_tab = call_function_state(state, get_selected_tab, &[frame])?;
    Ok(values_match(selected_tab, tab_index))
}

fn try_set_collections_journal_shown(state: &mut LuaState, tab_index: Val) -> LuaResult<bool> {
    let global = Val::Table(state.global);
    let frame = table_get(state, global, "CollectionsJournal");
    if matches!(frame, Val::Nil) {
        return Ok(false);
    }

    let set_shown = table_get(state, global, "SetCollectionsJournalShown");
    let Val::Function(_) = set_shown else {
        return Ok(false);
    };

    let tab_matches = collections_journal_tab_matches(state, frame, tab_index)?;
    let shown = !(collections_journal_is_shown(state) && tab_matches);
    let args = if matches!(tab_index, Val::Nil) {
        vec![Val::Bool(shown)]
    } else {
        vec![Val::Bool(shown), tab_index]
    };
    call_function_state(state, set_shown, &args)?;
    sync_open_panel_membership(state, "CollectionsJournal", "CollectionsJournal");
    Ok(true)
}

fn toggle_collections_journal(state: &mut LuaState) -> LuaResult<u32> {
    let tab_index = state.stack_get(state.base);
    let global = Val::Table(state.global);
    if matches!(table_get(state, global, "CollectionsJournal"), Val::Nil) {
        let _ = try_load_addon(state, "Blizzard_Collections")?;
    }
    if try_set_collections_journal_shown(state, tab_index)? {
        return Ok(0);
    }
    toggle_panel(state, "CollectionsJournal", "CollectionsJournal")?;
    Ok(0)
}

fn toggle_encounter_journal(state: &mut LuaState) -> LuaResult<u32> {
    toggle_loadable_panel(
        state,
        "Blizzard_EncounterJournal",
        "EncounterJournal",
        "EncounterJournal",
    )
}
define_toggle!(toggle_quest_log, "QuestLog", "QuestLogFrame");
define_toggle!(toggle_world_map, "WorldMap", "WorldMapFrame");
define_toggle!(toggle_friends_frame, "Friends", "FriendsFrame");
#[cfg(not(feature = "client-retail"))]
define_toggle!(toggle_guild_frame, "Guild", "GuildFrame");
define_toggle!(toggle_help_frame, "Help", "HelpFrame");
fn toggle_social_panel(state: &mut LuaState) -> LuaResult<u32> {
    let global = Val::Table(state.global);
    let toggle_friends_frame = table_get(state, global, "ToggleFriendsFrame");
    if matches!(toggle_friends_frame, Val::Function(_)) {
        let friends_tab = table_get(state, global, "FRIEND_TAB_FRIENDS");
        let args = if matches!(friends_tab, Val::Nil) {
            Vec::new()
        } else {
            vec![friends_tab]
        };
        call_function_state(state, toggle_friends_frame, &args)?;
        sync_open_panel_membership(state, "Social", "FriendsFrame");
        return Ok(0);
    }

    toggle_panel(state, "Social", "FriendsFrame")?;
    Ok(0)
}
define_toggle!(toggle_minimap, "Minimap", "MinimapCluster");

/// Panel-token table for introspection (exposed to docs + tests).
pub fn panel_tokens() -> &'static [(&'static str, &'static str)] {
    PANELS
}

pub fn register_all(lua: &mut rilua::Lua) -> crate::Result<()> {
    LuaApiMut::register_function(lua, "ToggleCharacter", toggle_character)?;
    LuaApiMut::register_function(lua, "ToggleSpellBook", toggle_spell_book)?;
    LuaApiMut::register_function(lua, "ToggleTalentFrame", toggle_talent_frame)?;
    LuaApiMut::register_function(lua, "ToggleQuestLog", toggle_quest_log)?;
    LuaApiMut::register_function(lua, "ToggleWorldMap", toggle_world_map)?;
    LuaApiMut::register_function(lua, "ToggleFriendsFrame", toggle_friends_frame)?;
    #[cfg(not(feature = "client-retail"))]
    LuaApiMut::register_function(lua, "ToggleGuildFrame", toggle_guild_frame)?;
    LuaApiMut::register_function(lua, "ToggleHelpFrame", toggle_help_frame)?;
    LuaApiMut::register_function(lua, "ToggleSocialPanel", toggle_social_panel)?;
    LuaApiMut::register_function(lua, "ToggleMinimap", toggle_minimap)?;
    LuaApiMut::register_function(lua, "ToggleCollectionsJournal", toggle_collections_journal)?;
    LuaApiMut::register_function(lua, "ToggleEncounterJournal", toggle_encounter_journal)?;
    Ok(())
}
