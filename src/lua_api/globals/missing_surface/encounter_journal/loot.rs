use crate::encounter_journal_data as data;
use crate::items;
use crate::lua_api::globals::strings::string_data::game_enums::ITEM_QUALITY_COLORS_DATA;
use crate::lua_api::methods::{
    borrow_state, borrow_state_mut, create_string, create_table, table_set, val_to_string,
};
use crate::lua_bridge::{FromStack, stack_val};
use rilua::vm::state::LuaState;
use rilua::{LuaResult, Val};

const INVENTORY_TYPE_SLOTS: &[(u8, &str)] = &[
    (1, "INVTYPE_HEAD"),
    (2, "INVTYPE_NECK"),
    (3, "INVTYPE_SHOULDER"),
    (4, "INVTYPE_BODY"),
    (5, "INVTYPE_CHEST"),
    (6, "INVTYPE_WAIST"),
    (7, "INVTYPE_LEGS"),
    (8, "INVTYPE_FEET"),
    (9, "INVTYPE_WRIST"),
    (10, "INVTYPE_HAND"),
    (11, "INVTYPE_FINGER"),
    (12, "INVTYPE_TRINKET"),
    (13, "INVTYPE_WEAPON"),
    (14, "INVTYPE_SHIELD"),
    (15, "INVTYPE_RANGED"),
    (16, "INVTYPE_CLOAK"),
    (17, "INVTYPE_2HWEAPON"),
    (20, "INVTYPE_ROBE"),
    (21, "INVTYPE_WEAPONMAINHAND"),
    (22, "INVTYPE_WEAPONOFFHAND"),
    (23, "INVTYPE_HOLDABLE"),
    (25, "INVTYPE_THROWN"),
    (26, "INVTYPE_RANGEDRIGHT"),
    (28, "INVTYPE_RELIC"),
];

pub(super) fn get_loot_info(state: &mut LuaState) -> LuaResult<u32> {
    let item_id = u32::from_stack(state, 1)?;
    let table = build_loot_table(state, item_id, 0);
    state.push(table);
    Ok(1)
}

pub(super) fn get_loot_info_by_index(state: &mut LuaState) -> LuaResult<u32> {
    let index = u32::from_stack(state, 1)? as usize;
    let (current_encounter, current_instance, difficulty) = {
        let sim = borrow_state(state)?;
        (
            sim.encounter_journal.current_encounter,
            sim.encounter_journal.current_instance,
            sim.encounter_journal.difficulty,
        )
    };
    let loot = filtered_loot(current_encounter, current_instance, difficulty);
    if index == 0 || index > loot.len() {
        return Ok(0);
    }
    let row = loot[index - 1];
    let item_id = row.item_id;
    let encounter_id = row.encounter_id;
    let table = build_loot_table(state, item_id, encounter_id);
    state.push(table);
    Ok(1)
}

pub(super) fn instance_has_loot(state: &mut LuaState) -> LuaResult<u32> {
    let instance_id = borrow_state(state)?.encounter_journal.current_instance;
    let has_loot = data::encounters_for_instance(instance_id)
        .iter()
        .any(|e| !data::loot_for_encounter(e.id).is_empty());
    state.push(Val::Bool(has_loot));
    Ok(1)
}

pub(super) fn ej_get_num_loot(state: &mut LuaState) -> LuaResult<u32> {
    let (encounter, instance, difficulty) = {
        let sim = borrow_state(state)?;
        (
            sim.encounter_journal.current_encounter,
            sim.encounter_journal.current_instance,
            sim.encounter_journal.difficulty,
        )
    };
    let count = filtered_loot(encounter, instance, difficulty).len();
    state.push(Val::Num(count as f64));
    Ok(1)
}

pub(super) fn ej_get_loot_info_by_index_global(state: &mut LuaState) -> LuaResult<u32> {
    get_loot_info_by_index(state)
}

pub(super) fn ej_get_loot_filter(state: &mut LuaState) -> LuaResult<u32> {
    let (class_filter, spec_filter) = {
        let sim = borrow_state(state)?;
        (
            sim.encounter_journal.class_filter,
            sim.encounter_journal.spec_filter,
        )
    };
    state.push(Val::Num(class_filter as f64));
    state.push(Val::Num(spec_filter as f64));
    Ok(2)
}

pub(super) fn ej_set_loot_filter(state: &mut LuaState) -> LuaResult<u32> {
    let class_filter = loot_filter_arg_or_zero(state, 1);
    let spec_filter = loot_filter_arg_or_zero(state, 2);
    let mut sim = borrow_state_mut(state)?;
    sim.encounter_journal.class_filter = class_filter;
    sim.encounter_journal.spec_filter = spec_filter;
    Ok(0)
}

fn loot_filter_arg_or_zero(state: &LuaState, index: i32) -> u32 {
    u32::from_stack(state, index)
        .ok()
        .or_else(|| {
            val_to_string(state, stack_val(state, index))
                .and_then(|value| value.parse::<u32>().ok())
        })
        .unwrap_or(0)
}

pub(super) fn ej_reset_loot_filter(state: &mut LuaState) -> LuaResult<u32> {
    let mut sim = borrow_state_mut(state)?;
    sim.encounter_journal.class_filter = 0;
    sim.encounter_journal.spec_filter = 0;
    Ok(0)
}

pub(super) fn ej_get_inv_type_sort_order(state: &mut LuaState) -> LuaResult<u32> {
    let inv_type = u32::from_stack(state, 1).unwrap_or(0);
    let order = match inv_type {
        1 => 1,                       // INVTYPE_HEAD
        3 => 2,                       // INVTYPE_SHOULDER
        16 => 3,                      // INVTYPE_CLOAK
        5 | 20 => 4,                  // INVTYPE_CHEST / INVTYPE_ROBE
        4 => 5,                       // INVTYPE_BODY (shirt)
        19 => 6,                      // INVTYPE_TABARD
        9 => 7,                       // INVTYPE_WRIST
        10 => 8,                      // INVTYPE_HAND
        6 => 9,                       // INVTYPE_WAIST
        7 => 10,                      // INVTYPE_LEGS
        8 => 11,                      // INVTYPE_FEET
        2 => 12,                      // INVTYPE_NECK
        11 => 13,                     // INVTYPE_FINGER
        12 => 14,                     // INVTYPE_TRINKET
        13 | 17 | 21 | 22 | 26 => 15, // weapons
        14 => 16,                     // INVTYPE_SHIELD
        15 | 23 | 25 | 28 => 17,      // ranged/holdable/relic
        _ => 99,
    };
    state.push(Val::Num(order as f64));
    Ok(1)
}

pub(super) fn ej_get_num_encounters_for_loot_by_index(state: &mut LuaState) -> LuaResult<u32> {
    state.push(Val::Num(1.0));
    Ok(1)
}

pub(super) fn ej_is_loot_list_out_of_date(state: &mut LuaState) -> LuaResult<u32> {
    state.push(Val::Bool(false));
    Ok(1)
}

fn build_loot_table(state: &mut LuaState, item_id: u32, encounter_id: u32) -> Val {
    let table = create_table(state);
    let item = items::get_item(item_id);
    let name_str = item.map(|i| i.name).unwrap_or("");
    let icon_id = loot_icon_file_data_id(item) as f64;
    let quality_color = loot_quality_color_code(item);
    let inv_type_num = item.map(|i| i.inventory_type as f64).unwrap_or(0.0);
    let link_str = item_link(item_id);
    let slot_str = inv_type_slot(item.map(|i| i.inventory_type).unwrap_or(0));

    let name = create_string(state, name_str);
    let item_quality = create_string(state, quality_color);
    let link = create_string(state, &link_str);
    let slot = create_string(state, slot_str);
    let armor_type = create_string(state, "");

    table_set(state, table, "itemID", Val::Num(item_id as f64));
    table_set(state, table, "name", name);
    table_set(state, table, "icon", Val::Num(icon_id));
    table_set(state, table, "itemQuality", item_quality);
    table_set(state, table, "inventoryType", Val::Num(inv_type_num));
    table_set(state, table, "link", link);
    table_set(state, table, "slot", slot);
    table_set(state, table, "armorType", armor_type);
    table_set(state, table, "encounterID", Val::Num(encounter_id as f64));
    table_set(state, table, "displayAsPerPlayerLoot", Val::Bool(false));
    table_set(state, table, "displayAsExtremelyRare", Val::Bool(false));
    table_set(state, table, "displayAsVeryRare", Val::Bool(false));
    table_set(state, table, "handError", Val::Bool(false));
    table_set(state, table, "weaponTypeError", Val::Bool(false));
    table_set(state, table, "displaySeasonID", Val::Nil);
    table_set(state, table, "filterType", Val::Num(0.0));
    table
}

fn loot_icon_file_data_id(item: Option<&items::ItemInfo>) -> u32 {
    item.and_then(|i| (i.icon_file_data_id != 0).then_some(i.icon_file_data_id))
        .unwrap_or(134400)
}

fn loot_quality_color_code(item: Option<&items::ItemInfo>) -> &'static str {
    let quality = item.map(|i| i.quality).unwrap_or(1);
    item_quality_color_code(quality)
}

fn item_quality_color_code(quality: u8) -> &'static str {
    ITEM_QUALITY_COLORS_DATA
        .iter()
        .find_map(|(index, _, _, _, hex)| (*index == quality as i32).then_some(*hex))
        .unwrap_or("ffffffff")
}

fn item_link(item_id: u32) -> String {
    if item_id == 0 {
        return String::new();
    }
    let name = items::get_item(item_id)
        .map(|i| i.name)
        .unwrap_or("Unknown");
    format!("|cffffffff|Hitem:{item_id}|h[{name}]|h|r")
}

fn inv_type_slot(inv_type: u8) -> &'static str {
    INVENTORY_TYPE_SLOTS
        .iter()
        .find_map(|(id, name)| (*id == inv_type).then_some(*name))
        .unwrap_or("")
}

fn filtered_loot(encounter_id: u32, instance_id: u32, difficulty: u32) -> Vec<&'static data::Loot> {
    if encounter_id != 0 {
        return data::loot_for_encounter(encounter_id)
            .iter()
            .copied()
            .filter(|l| {
                difficulty == 0
                    || l.difficulty_mask == 0
                    || (l.difficulty_mask & (1 << difficulty.saturating_sub(1)) != 0)
            })
            .collect();
    }
    if instance_id != 0 {
        let mut all = Vec::new();
        for encounter in data::encounters_for_instance(instance_id) {
            all.extend(data::loot_for_encounter(encounter.id).iter().copied());
        }
        return all;
    }
    Vec::new()
}
