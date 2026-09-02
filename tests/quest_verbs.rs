//! Integration tests for `src/lua_api/globals/quest_verbs.rs`.

use wow_ui_sim::lua_api::WowLuaEnv;

fn env() -> WowLuaEnv {
    WowLuaEnv::new().expect("WowLuaEnv init")
}

fn fired(env: &WowLuaEnv, name: &str) -> bool {
    env.state()
        .borrow()
        .events
        .pending()
        .iter()
        .any(|e| e.name == name)
}

// ── ConfirmAcceptQuest ────────────────────────────────────────────────────────

#[test]
fn confirm_accept_quest_moves_offer_to_log_and_fires_accepted() {
    let env = env();
    env.state().borrow_mut().pending_quest_offer = Some(42);
    env.exec("ConfirmAcceptQuest()").unwrap();
    let st = env.state().borrow();
    assert!(st.pending_quest_offer.is_none());
    assert!(st.quest_log.contains(&42));
    drop(st);
    assert!(fired(&env, "QUEST_ACCEPTED"));
}

#[test]
fn confirm_accept_quest_without_offer_is_noop() {
    let env = env();
    env.state().borrow_mut().pending_quest_offer = None;
    let before_len = env.state().borrow().quest_log.len();
    env.exec("ConfirmAcceptQuest()").unwrap();
    assert_eq!(env.state().borrow().quest_log.len(), before_len);
    assert!(!fired(&env, "QUEST_ACCEPTED"));
}

#[test]
fn confirm_accept_quest_is_idempotent_for_same_id() {
    let env = env();
    {
        let mut st = env.state().borrow_mut();
        st.quest_log.push(99);
        st.pending_quest_offer = Some(99);
    }
    env.exec("ConfirmAcceptQuest()").unwrap();
    let st = env.state().borrow();
    let count = st.quest_log.iter().filter(|q| **q == 99).count();
    assert_eq!(count, 1, "quest_log must not duplicate entries");
}

// ── CloseQuestFrame ───────────────────────────────────────────────────────────

#[test]
fn close_quest_frame_fires_quest_finished_and_clears_offer() {
    let env = env();
    env.state().borrow_mut().pending_quest_offer = Some(7);
    env.exec("CloseQuestFrame()").unwrap();
    assert!(env.state().borrow().pending_quest_offer.is_none());
    assert!(fired(&env, "QUEST_FINISHED"));
}

// ── AcknowledgeAutoQuestPopUp ─────────────────────────────────────────────────

#[test]
fn acknowledge_auto_quest_popup_is_silent_dismiss() {
    let env = env();
    let before = env.state().borrow().events.pending().len();
    env.exec("AcknowledgeAutoQuestPopUp(123)").unwrap();
    let after = env.state().borrow().events.pending().len();
    assert_eq!(after, before, "popup dismiss must not queue events");
}

// ── QuestChoiceFrame_SetActiveChoice ──────────────────────────────────────────

#[test]
fn quest_choice_set_active_writes_choice_id() {
    let env = env();
    env.exec("QuestChoiceFrame_SetActiveChoice(4567)").unwrap();
    assert_eq!(env.state().borrow().quest_choice_id, Some(4567));
}

#[cfg(feature = "client-mists")]
#[test]
fn c_quest_choice_populates_seeded_options_and_records_response() {
    let env = env();
    let (choice_id, option_count, option_id, option_text): (i32, i32, i32, String) = env
        .eval(
            r#"
            local choiceID, _question, optionCount = C_QuestChoice.GetQuestChoiceInfo()
            local optionID, optionText = C_QuestChoice.GetQuestChoiceOptionInfo(0)
            C_QuestChoice.SendQuestChoiceResponse(optionID)
            return choiceID, optionCount, optionID, optionText
            "#,
        )
        .unwrap();

    assert_eq!(choice_id, 9001);
    assert_eq!(option_count, 2);
    assert_eq!(option_id, 101);
    assert_eq!(option_text, "Aid the Shado-Pan");
    assert_eq!(env.state().borrow().quest_choice_response_id, Some(101));
}

// ── QuestMapLogTitleButton_OnClick ────────────────────────────────────────────

#[test]
fn quest_map_log_title_button_click_records_selected_id() {
    let env = env();
    env.exec("QuestMapLogTitleButton_OnClick(8192)").unwrap();
    assert_eq!(env.state().borrow().selected_quest_log_id, Some(8192));
}

// ── SetAbandonQuest ───────────────────────────────────────────────────────────

#[test]
fn set_abandon_quest_removes_selected_and_fires_removed() {
    let env = env();
    {
        let mut st = env.state().borrow_mut();
        st.quest_log = vec![1, 2, 3];
        st.selected_quest_log_id = Some(2);
    }
    env.exec("SetAbandonQuest()").unwrap();
    let st = env.state().borrow();
    assert_eq!(st.quest_log, vec![1, 3]);
    assert_eq!(st.abandon_quest_id, Some(2));
    drop(st);
    assert!(fired(&env, "QUEST_REMOVED"));
}

#[test]
fn set_abandon_quest_without_selection_is_noop() {
    let env = env();
    env.state().borrow_mut().selected_quest_log_id = None;
    env.exec("SetAbandonQuest()").unwrap();
    assert!(env.state().borrow().abandon_quest_id.is_none());
    assert!(!fired(&env, "QUEST_REMOVED"));
}

// ── SetTrackedAchievement / UntrackAchievement ────────────────────────────────

#[test]
fn set_tracked_achievement_add_and_remove() {
    let env = env();
    env.exec("SetTrackedAchievement(100, true)").unwrap();
    assert!(env.state().borrow().tracked_achievements.contains(&100));
    env.exec("SetTrackedAchievement(100, false)").unwrap();
    assert!(!env.state().borrow().tracked_achievements.contains(&100));
}

#[test]
fn set_tracked_achievement_default_tracked_is_true() {
    let env = env();
    // Second arg omitted → default true.
    env.exec("SetTrackedAchievement(200)").unwrap();
    assert!(env.state().borrow().tracked_achievements.contains(&200));
}

#[test]
fn untrack_achievement_removes_entry() {
    let env = env();
    env.exec("SetTrackedAchievement(300, true)").unwrap();
    env.exec("UntrackAchievement(300)").unwrap();
    assert!(!env.state().borrow().tracked_achievements.contains(&300));
}

#[test]
fn untrack_achievement_unknown_is_noop() {
    let env = env();
    let before = env.state().borrow().tracked_achievements.len();
    env.exec("UntrackAchievement(999)").unwrap();
    assert_eq!(env.state().borrow().tracked_achievements.len(), before);
}
