use super::super::shared::{opt_bool, opt_string, val_to_f64};
use crate::lua_api::frame::methods::methods_hierarchy::reparent_widget;
use crate::lua_api::globals::create_frame::helpers_shared::create_frame_instance;
use crate::lua_api::methods::{borrow_state, borrow_state_mut, frame_id_from_stack, frame_ref};
use crate::lua_bridge::{IntoStack, stack_val};
use crate::widget::WidgetType;
use rilua::vm::gc::arena::GcRef;
use rilua::vm::state::LuaState;
use rilua::vm::table::Table;
use rilua::{LuaResult, Val};

pub(super) fn scene_create_actor(state: &mut LuaState) -> LuaResult<u32> {
    let scene_id = frame_id_from_stack(state, 1)?;
    let name = opt_string(state, 2);
    let tag = name.clone();
    let actor_id = create_frame_instance(
        state,
        WidgetType::Frame,
        "ModelSceneActor",
        name,
        Some(scene_id),
        true,
        None,
    )?;
    let mut sim = borrow_state_mut(state)?;
    if let Some(scene) = sim.widgets.get_mut_visual(scene_id) {
        let model = scene.model_state_mut();
        model.model_scene_actor_ids.push(actor_id);
        if let Some(tag) = tag.filter(|t| !t.is_empty()) {
            model
                .model_scene_actor_tags
                .retain(|(existing, _)| existing != &tag);
            model.model_scene_actor_tags.push((tag, actor_id));
        }
    }
    drop(sim);
    let actor = frame_ref(state, actor_id)?;
    state.push(actor);
    Ok(1)
}

/// Returns the actor whose script tag matches `tag`, mirroring
/// `ModelSceneMixin:GetActorByTag`
/// (`vendor/wow-ui-source/Interface/AddOns/Blizzard_SharedXML/ModelSceneMixin.lua:136`).
/// AlliedRacesFrameMixin:UpdateModel relies on this lookup to attach
/// the per-race actor before re-applying the creature display ID.
pub(super) fn scene_get_actor_by_tag(state: &mut LuaState) -> LuaResult<u32> {
    let scene_id = frame_id_from_stack(state, 1)?;
    let Some(tag) = opt_string(state, 2) else {
        state.push(Val::Nil);
        return Ok(1);
    };
    let actor_id = borrow_state(state)?
        .widgets
        .get(scene_id)
        .and_then(|scene| {
            scene
                .model_state()
                .model_scene_actor_tags
                .iter()
                .find(|(existing, _)| existing == &tag)
                .map(|(_, id)| *id)
        });
    if let Some(actor_id) = actor_id {
        let actor = frame_ref(state, actor_id)?;
        state.push(actor);
    } else {
        state.push(Val::Nil);
    }
    Ok(1)
}

pub(super) fn scene_get_player_actor(state: &mut LuaState) -> LuaResult<u32> {
    let scene_id = frame_id_from_stack(state, 1)?;
    let actor_id = find_actor_by_tag(state, scene_id, "player")
        .map(Ok)
        .unwrap_or_else(|| create_player_actor(state, scene_id))?;
    let actor = frame_ref(state, actor_id)?;
    state.push(actor);
    Ok(1)
}

pub(super) fn scene_get_num_actors(state: &mut LuaState) -> LuaResult<u32> {
    let scene_id = frame_id_from_stack(state, 1)?;
    let count = borrow_state(state)?
        .widgets
        .get(scene_id)
        .map(|scene| scene.model_state().model_scene_actor_ids.len() as f64)
        .unwrap_or(0.0);
    count.into_stack(state)
}

fn find_actor_by_tag(state: &mut LuaState, scene_id: u64, tag: &str) -> Option<u64> {
    borrow_state(state)
        .ok()?
        .widgets
        .get(scene_id)?
        .model_state()
        .model_scene_actor_tags
        .iter()
        .find(|(existing, _)| existing == tag)
        .map(|(_, id)| *id)
}

fn create_player_actor(state: &mut LuaState, scene_id: u64) -> LuaResult<u64> {
    let actor_id = create_frame_instance(
        state,
        WidgetType::Frame,
        "ModelSceneActor",
        None,
        Some(scene_id),
        true,
        None,
    )?;
    let mut sim = borrow_state_mut(state)?;
    if let Some(scene) = sim.widgets.get_mut_visual(scene_id) {
        let model = scene.model_state_mut();
        model.model_scene_actor_ids.push(actor_id);
        model
            .model_scene_actor_tags
            .push(("player".to_string(), actor_id));
    }
    Ok(actor_id)
}

pub(super) fn scene_get_actor_at_index(state: &mut LuaState) -> LuaResult<u32> {
    let scene_id = frame_id_from_stack(state, 1)?;
    let index = val_to_f64(stack_val(state, 2)) as usize;
    let actor_id = borrow_state(state)?
        .widgets
        .get(scene_id)
        .and_then(|scene| {
            scene
                .model_state()
                .model_scene_actor_ids
                .get(index.saturating_sub(1))
                .copied()
        });
    if let Some(actor_id) = actor_id {
        let actor = frame_ref(state, actor_id)?;
        state.push(actor);
    } else {
        state.push(Val::Nil);
    }
    Ok(1)
}

pub(super) fn scene_take_actor(state: &mut LuaState) -> LuaResult<u32> {
    let scene_id = frame_id_from_stack(state, 1)?;
    let actor_id = {
        let mut sim = borrow_state_mut(state)?;
        sim.widgets.get_mut_visual(scene_id).and_then(|scene| {
            let model = scene.model_state_mut();
            let popped = model.model_scene_actor_ids.pop()?;
            model.model_scene_actor_tags.retain(|(_, id)| *id != popped);
            Some(popped)
        })
    };
    if let Some(actor_id) = actor_id {
        let mut sim = borrow_state_mut(state)?;
        reparent_widget(&mut sim.widgets, actor_id, None);
        drop(sim);
        let actor = frame_ref(state, actor_id)?;
        state.push(actor);
    } else {
        state.push(Val::Nil);
    }
    Ok(1)
}

/// Drops every actor from the scene's actor pool, mirroring
/// `ModelSceneMixin:ClearScene` /
/// `ModelSceneMixin:ReleaseAllActors`
/// (`vendor/wow-ui-source/Interface/AddOns/Blizzard_SharedXML/ModelSceneMixin.lua:16,217`).
/// Drained actors are reparented away from the scene so the scene's
/// `children` list and `model_scene_actor_ids` agree, matching what
/// `scene_take_actor` does for a single actor.
pub(super) fn scene_clear_scene(state: &mut LuaState) -> LuaResult<u32> {
    let scene_id = frame_id_from_stack(state, 1)?;
    let actor_ids: Vec<u64> = {
        let mut sim = borrow_state_mut(state)?;
        sim.widgets
            .get_mut_visual(scene_id)
            .and_then(|scene| scene.existing_model_state_mut())
            .map(|model| {
                model.model_scene_actor_tags.clear();
                std::mem::take(&mut model.model_scene_actor_ids)
            })
            .unwrap_or_default()
    };
    let mut sim = borrow_state_mut(state)?;
    for actor_id in actor_ids {
        reparent_widget(&mut sim.widgets, actor_id, None);
    }
    Ok(0)
}

pub(super) fn scene_set_view_insets(state: &mut LuaState) -> LuaResult<u32> {
    let id = frame_id_from_stack(state, 1)?;
    let l = val_to_f64(stack_val(state, 2)) as f32;
    let r = val_to_f64(stack_val(state, 3)) as f32;
    let t = val_to_f64(stack_val(state, 4)) as f32;
    let b = val_to_f64(stack_val(state, 5)) as f32;
    let mut sim = borrow_state_mut(state)?;
    if let Some(frame) = sim.widgets.get_mut_visual(id) {
        let view_insets = (l, r, t, b);
        frame.update_model_state(view_insets != (0.0, 0.0, 0.0, 0.0), |model| {
            model.model_scene_state.view_insets = view_insets;
        });
    }
    Ok(0)
}

pub(super) fn scene_get_view_insets(state: &mut LuaState) -> LuaResult<u32> {
    let id = frame_id_from_stack(state, 1)?;
    let (l, r, t, b) = {
        let sim = borrow_state(state)?;
        sim.widgets
            .get(id)
            .map(|f| f.model_state().model_scene_state.view_insets)
            .unwrap_or((0.0, 0.0, 0.0, 0.0))
    };
    (l as f64, r as f64, t as f64, b as f64).into_stack(state)
}

pub(super) fn scene_is_allow_overlapped_models(state: &mut LuaState) -> LuaResult<u32> {
    let id = frame_id_from_stack(state, 1)?;
    let allow = borrow_state(state)?
        .widgets
        .get(id)
        .map(|frame| {
            frame
                .model_state()
                .model_scene_state
                .allow_overlapped_models
        })
        .unwrap_or(false);
    state.push(Val::Bool(allow));
    Ok(1)
}

/// Rebuilds the scene's actor pool from the static `modelSceneID` manifest
/// (`SimState::model_scenes`), mirroring
/// `ModelSceneMixin:TransitionToModelSceneID`
/// (`vendor/wow-ui-source/Interface/AddOns/Blizzard_SharedXML/ModelSceneMixin.lua:71`).
///
/// Real WoW reads `C_ModelInfo.GetModelSceneInfoByID(modelSceneID)` and
/// re-creates one actor per declared row. We carry the same per-scene tag
/// list keyed on `modelSceneID`, so `ClearScene + Transition + GetActorByTag`
/// rounds-trips for `Blizzard_AlliedRacesFrameUI:UpdateModel`. Visual
/// transitions (camera pan, fade) are out of scope — actor pool + the
/// `self.modelSceneID` book-keeping is enough for tag lookup, the only
/// observable contract addons rely on.
///
/// We deliberately do not invoke `self.resetCallback` here. The Blizzard
/// mixin only fires the callback from `:Reset()`, and AlliedRaces' callback
/// (`AlliedRacesFrameMixin.OnModelSceneReset`) calls back into
/// `UpdateModel → TransitionToModelSceneID`, which would loop forever.
pub(super) fn scene_transition_to_model_scene_id(state: &mut LuaState) -> LuaResult<u32> {
    let scene_id = frame_id_from_stack(state, 1)?;
    let Val::Table(scene_ref) = stack_val(state, 1) else {
        return Ok(0);
    };
    let target_scene_id = val_to_f64(stack_val(state, 2)) as i64;
    let force = opt_bool(state, 5).unwrap_or(false);

    let tags = lookup_model_scene_actor_tags(state, target_scene_id)?;
    let Some(tags) = tags else { return Ok(0) };

    if !force && current_model_scene_id_matches(state, scene_ref, target_scene_id) {
        return Ok(0);
    }

    drain_existing_actors(state, scene_id)?;
    rebuild_actor_pool(state, scene_id, &tags)?;
    write_model_scene_id(state, scene_ref, target_scene_id);
    Ok(0)
}

fn lookup_model_scene_actor_tags(
    state: &mut LuaState,
    scene_id: i64,
) -> LuaResult<Option<Vec<String>>> {
    let sim = borrow_state(state)?;
    let tags = sim
        .model_scenes
        .get(&scene_id)
        .filter(|t| !t.is_empty())
        .cloned();
    Ok(tags)
}

fn current_model_scene_id_matches(
    state: &mut LuaState,
    scene_ref: GcRef<Table>,
    target_scene_id: i64,
) -> bool {
    let key = state.gc.intern_string(b"modelSceneID");
    let Some(scene_table) = state.gc.tables.get(scene_ref) else {
        return false;
    };
    matches!(
        scene_table.get_str(key, &state.gc.string_arena),
        Val::Num(n) if (n as i64) == target_scene_id
    )
}

fn drain_existing_actors(state: &mut LuaState, scene_id: u64) -> LuaResult<()> {
    let actor_ids: Vec<u64> = {
        let mut sim = borrow_state_mut(state)?;
        sim.widgets
            .get_mut_visual(scene_id)
            .and_then(|scene| scene.existing_model_state_mut())
            .map(|model| {
                model.model_scene_actor_tags.clear();
                std::mem::take(&mut model.model_scene_actor_ids)
            })
            .unwrap_or_default()
    };
    let mut sim = borrow_state_mut(state)?;
    for actor_id in actor_ids {
        reparent_widget(&mut sim.widgets, actor_id, None);
    }
    Ok(())
}

fn rebuild_actor_pool(state: &mut LuaState, scene_id: u64, tags: &[String]) -> LuaResult<()> {
    for tag in tags {
        let actor_id = create_frame_instance(
            state,
            WidgetType::Frame,
            "ModelSceneActor",
            Some(tag.clone()),
            Some(scene_id),
            true,
            None,
        )?;
        let mut sim = borrow_state_mut(state)?;
        if let Some(scene) = sim.widgets.get_mut_visual(scene_id) {
            let model = scene.model_state_mut();
            model.model_scene_actor_ids.push(actor_id);
            model.model_scene_actor_tags.push((tag.clone(), actor_id));
        }
    }
    Ok(())
}

fn write_model_scene_id(state: &mut LuaState, scene_ref: GcRef<Table>, scene_id: i64) {
    let key = state.gc.intern_string(b"modelSceneID");
    if let Some(t) = state.gc.tables.get_mut(scene_ref) {
        let _ = t.raw_set(
            Val::Str(key),
            Val::Num(scene_id as f64),
            &state.gc.string_arena,
        );
    }
    state.gc.barrier_back(scene_ref);
}

/// Stores a Lua callback at `self.resetCallback`, mirroring
/// `ModelSceneMixin:SetResetCallback`
/// (`vendor/wow-ui-source/Interface/AddOns/Blizzard_SharedXML/ModelSceneMixin.lua:56`).
/// `ModelSceneMixin:Reset()` reads `self.resetCallback` and invokes it
/// with the scene as the only argument, so writing through the frame
/// metatable (same path `self.resetCallback = cb` would take) keeps both
/// the widget method and the inherited Blizzard `Reset()` consistent.
/// Non-function / non-nil arguments are ignored to match the mixin
/// contract (any value other than a callable would crash `Reset()`).
pub(super) fn scene_set_reset_callback(state: &mut LuaState) -> LuaResult<u32> {
    let frame_val = stack_val(state, 1);
    let Val::Table(frame_ref) = frame_val else {
        return Ok(0);
    };
    let callback = stack_val(state, 2);
    if !matches!(callback, Val::Function(_) | Val::Nil) {
        return Ok(0);
    }
    let key = state.gc.intern_string(b"resetCallback");
    if let Some(t) = state.gc.tables.get_mut(frame_ref) {
        let _ = t.raw_set(Val::Str(key), callback, &state.gc.string_arena);
    }
    state.gc.barrier_back(frame_ref);
    Ok(0)
}
