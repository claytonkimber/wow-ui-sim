//! Template helpers shared by the rilua loader/runtime path.

pub(crate) mod direct;

use crate::lua_api::methods::{frame_ref, sync_child_to_rilua, table_set};
use rilua::LuaResult;
use rilua::vm::state::LuaState;

pub fn set_intrinsic(state: &mut LuaState, frame_id: u64, base: &str) {
    let Ok(frame) = frame_ref(state, frame_id) else {
        return;
    };
    let value = crate::lua_api::methods::create_string(state, base);
    table_set(state, frame, "intrinsic", value);
}

pub fn assign_parent_key(
    state: &mut LuaState,
    parent_id: u64,
    parent_key: &str,
    child_id: u64,
) -> LuaResult<()> {
    let (target_parent_id, resolved_key) = resolve_parent_key_target(state, parent_id, parent_key);
    let Some(target_parent_id) = target_parent_id else {
        return Ok(());
    };

    {
        let mut sim = crate::lua_api::methods::borrow_state_mut(state)?;
        if let Some(parent) = sim.widgets.get_mut_visual(target_parent_id) {
            parent.children_keys.insert(resolved_key.clone(), child_id);
        }
        if sim.widgets.get(child_id).and_then(|child| child.parent_id) == Some(target_parent_id)
            && let Some(child) = sim.widgets.get_mut_visual(child_id)
        {
            child.parent_key = Some(resolved_key.clone());
        }
    }

    sync_child_to_rilua(state, target_parent_id, &resolved_key, child_id)
}

pub fn repair_direct_child_parent_keys(state: &mut LuaState, parent_id: u64) -> LuaResult<()> {
    let repairs = {
        let sim = crate::lua_api::methods::borrow_state(state)?;
        let Some(parent) = sim.widgets.get(parent_id) else {
            return Ok(());
        };
        let parent_name = parent.name.as_deref();

        parent
            .children
            .iter()
            .filter_map(|child_id| {
                let child = sim.widgets.get(*child_id)?;
                let key = child.parent_key.clone().or_else(|| {
                    infer_parent_key_from_child_name(
                        parent_name,
                        child.name.as_deref(),
                        child.widget_type,
                    )
                });
                let key = key?;
                (parent.children_keys.get(&key) != Some(child_id)).then(|| (key, *child_id))
            })
            .collect::<Vec<_>>()
    };

    for (key, child_id) in repairs {
        assign_parent_key(state, parent_id, &key, child_id)?;
    }

    Ok(())
}

pub fn repair_transparent_wrapper_parent_key_aliases(
    state: &mut LuaState,
    wrapper_id: u64,
) -> LuaResult<()> {
    let aliases = collect_transparent_wrapper_aliases(state, wrapper_id)?;

    for (target_parent_id, key, child_id) in aliases {
        sync_child_to_rilua(state, target_parent_id, &key, child_id)?;
    }

    Ok(())
}

fn collect_transparent_wrapper_aliases(
    state: &mut LuaState,
    wrapper_id: u64,
) -> LuaResult<Vec<(u64, String, u64)>> {
    let sim = crate::lua_api::methods::borrow_state(state)?;
    let Some(wrapper) = sim.widgets.get(wrapper_id) else {
        return Ok(Vec::new());
    };
    if !is_transparent_wrapper(wrapper) {
        return Ok(Vec::new());
    }
    let Some(target_parent_id) = wrapper.parent_id else {
        return Ok(Vec::new());
    };

    let target_parent_name = sim
        .widgets
        .get(target_parent_id)
        .and_then(|parent| parent.name.as_deref());
    let aliases = wrapper
        .children
        .iter()
        .flat_map(|child_id| {
            let Some(child) = sim.widgets.get(*child_id) else {
                return Vec::new();
            };
            let key = child.parent_key.clone().or_else(|| {
                infer_parent_key_from_child_name(
                    target_parent_name,
                    child.name.as_deref(),
                    child.widget_type,
                )
                .map(lowercase_first)
            });
            key.into_iter()
                .map(|key| (target_parent_id, key, *child_id))
                .collect::<Vec<_>>()
        })
        .collect();

    Ok(aliases)
}

fn is_transparent_wrapper(frame: &crate::widget::Frame) -> bool {
    let has_xml_name = frame
        .name
        .as_deref()
        .is_some_and(|name| !name.starts_with("__tpl_"));
    !has_xml_name && frame.parent_key.is_none()
}

pub fn repair_descendant_name_aliases(state: &mut LuaState, parent_id: u64) -> LuaResult<()> {
    let aliases = {
        let sim = crate::lua_api::methods::borrow_state(state)?;
        let Some(parent) = sim.widgets.get(parent_id) else {
            return Ok(());
        };
        if parent.name.is_none() {
            return Ok(());
        }
        let mut aliases = Vec::new();
        let mut stack = parent.children.clone();
        while let Some(child_id) = stack.pop() {
            let Some(child) = sim.widgets.get(child_id) else {
                continue;
            };
            stack.extend(child.children.iter().copied());
            if child.parent_id == Some(parent_id) {
                continue;
            }
            if let Some(key) = infer_parent_key_from_child_name(
                parent.name.as_deref(),
                child.name.as_deref(),
                child.widget_type,
            ) {
                aliases.push((parent_id, lowercase_first(key), child_id));
            }
        }
        aliases
    };

    for (parent_id, key, child_id) in aliases {
        sync_child_to_rilua(state, parent_id, &key, child_id)?;
    }

    Ok(())
}

pub fn repair_transparent_descendant_parent_key_aliases(
    state: &mut LuaState,
    parent_id: u64,
) -> LuaResult<()> {
    let aliases = {
        let sim = crate::lua_api::methods::borrow_state(state)?;
        let Some(parent) = sim.widgets.get(parent_id) else {
            return Ok(());
        };
        let mut aliases = Vec::new();
        let mut stack = parent.children.clone();
        while let Some(frame_id) = stack.pop() {
            let Some(frame) = sim.widgets.get(frame_id) else {
                continue;
            };
            stack.extend(frame.children.iter().copied());
            let synthetic_name = frame
                .name
                .as_deref()
                .is_some_and(|name| name.starts_with("__tpl_"));
            if !synthetic_name || frame.parent_key.is_some() {
                continue;
            }
            for child_id in &frame.children {
                let Some(child) = sim.widgets.get(*child_id) else {
                    continue;
                };
                if let Some(key) = child.parent_key.as_ref() {
                    aliases.push((parent_id, key.clone(), *child_id));
                }
            }
        }
        aliases
    };

    for (parent_id, key, child_id) in aliases {
        sync_child_to_rilua(state, parent_id, &key, child_id)?;
    }

    Ok(())
}

fn lowercase_first(value: String) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return value;
    };
    first.to_lowercase().chain(chars).collect()
}

fn infer_parent_key_from_child_name(
    parent_name: Option<&str>,
    child_name: Option<&str>,
    child_type: crate::widget::WidgetType,
) -> Option<String> {
    let parent_name = parent_name?;
    let child_name = child_name?;
    let check_button_key = (child_type == crate::widget::WidgetType::CheckButton)
        .then(|| {
            ["Check", "CheckButton", "Checkbox"]
                .into_iter()
                .find(|suffix| child_name == format!("{parent_name}{suffix}"))
                .map(|_| "CheckButton".to_string())
        })
        .flatten();

    check_button_key.or_else(|| {
        child_name
            .strip_prefix(parent_name)
            .filter(|suffix| !suffix.is_empty())
            .map(str::to_string)
    })
}

fn resolve_parent_key_target(
    state: &LuaState,
    parent_id: u64,
    parent_key: &str,
) -> (Option<u64>, String) {
    if let Some(key) = parent_key.strip_prefix("$parent.") {
        let target_parent = crate::lua_api::methods::borrow_state(state)
            .ok()
            .and_then(|sim| {
                sim.widgets
                    .get(parent_id)
                    .and_then(|parent| parent.parent_id)
            });
        return (target_parent, key.to_string());
    }
    (Some(parent_id), parent_key.to_string())
}

pub fn fire_deferred_child_onloads(_state: &mut LuaState) -> usize {
    0
}

pub(super) fn get_size_values(size: &crate::xml::SizeXml) -> (Option<f32>, Option<f32>) {
    if size.x.is_some() || size.y.is_some() {
        (size.x, size.y)
    } else if let Some(abs) = &size.abs_dimension {
        (abs.x, abs.y)
    } else {
        (None, None)
    }
}
