# World Map Voice Chat Alerts

Initial investigation found a reduced-harness artifact, not a proven `WorldMapFrame` render-order bug in the live/full addon stack. Follow-up probes showed two separate behaviors:

- voice prompt alerts sort below the world map panel in a combined world-map/chat/channels stack
- the live-like `1024x768` overlap comes from the standalone chat voice button (`ChatFrameChannelButton`)

The second issue is now fixed in the simulator.

## Content

## Symptoms

- In the isolated world-map stack, `VoiceChatPromptActivateChannel` and `VoiceChatChannelActivatedNotification` can appear above the map panel even before startup settles.
- The visible frames are top-level voice prompt alerts, not children of `WorldMapFrame`.
- `ChatAlertFrame` stays as the simulator stub, so any shown voice alert is unmanaged and falls back to the default top-level position instead of the real chat-alert anchor chain.

## Root Cause

### 1. Missing `Blizzard_SocialToast`

`Blizzard_Channels` depends on `Blizzard_SocialToast`:

- [`Interface/BlizzardUI/Blizzard_Channels/Blizzard_Channels.toc`](../../../Interface/BlizzardUI/Blizzard_Channels/Blizzard_Channels.toc)
- [`Interface/BlizzardUI/Blizzard_SocialToast/Blizzard_SocialToast.toc`](../../../Interface/BlizzardUI/Blizzard_SocialToast/Blizzard_SocialToast.toc)

The voice prompt frames inherit through:

- `VoiceChatPromptActivateChannel` / `VoiceChatChannelActivatedNotification`
- `VoiceChatPromptTemplate`
- `SocialToastTemplate`

`SocialToastTemplate` is where `hidden="true"` lives:

- [`Interface/BlizzardUI/Blizzard_Channels/VoiceChatPrompt.xml`](../../../Interface/BlizzardUI/Blizzard_Channels/VoiceChatPrompt.xml)
- [`Interface/BlizzardUI/Blizzard_SocialToast/SocialToast.xml`](../../../Interface/BlizzardUI/Blizzard_SocialToast/SocialToast.xml)

When `Blizzard_Channels` is loaded without `Blizzard_SocialToast`, the template chain is incomplete at frame-creation time, so the prompt frames do not inherit `hidden="true"` and start shown.

Evidence from a clean single-load probe:

- Without `Blizzard_SocialToast`, immediately after loading `Blizzard_Channels`: `prompt_shown=true`, `notif_shown=true`, both alpha `1`.
- With `Blizzard_SocialToast` included before `Blizzard_Channels`: immediately after loading `Blizzard_Channels`: `prompt_shown=false`, `notif_shown=false`, both alpha `0`.

This rules out a generic XML hidden-inheritance bug. A synthetic probe with an explicit `SocialToastTemplate -> VoiceChatPromptTemplate -> VoiceChatPromptActivateChannel` chain still started hidden as expected.

### 2. Missing real `ChatAlertFrame`

The reduced stack also omits the Blizzard chat-frame addons that define the real `ChatAlertFrame`:

- [`Interface/BlizzardUI/Blizzard_ChatFrame/Mainline/FloatingChatFrameAlertFrame.xml`](../../../Interface/BlizzardUI/Blizzard_ChatFrame/Mainline/FloatingChatFrameAlertFrame.xml)
- [`Interface/BlizzardUI/Blizzard_ChatFrameBase/Mainline/ChatAlertFrameMixin.lua`](../../../Interface/BlizzardUI/Blizzard_ChatFrameBase/Mainline/ChatAlertFrameMixin.lua)

Without those addons, the simulator falls back to the stub created in:

- [`src/lua_api/globals/global_frames.rs`](../../../src/lua_api/globals/global_frames.rs)

That stub only provides no-op alert-container methods (`AddAutoAnchoredSubSystem`, `SetSubSystemAnchorPriority`, `UpdateAnchors`). So if the voice prompt frames become visible, they are not re-anchored into the chat-alert stack and remain at the default top-level position.

## Scope

This root cause was confirmed in the reduced world-map harness, not in a fully loaded game UI stack.

Follow-up check:

- A combined-stack regression in [`tests/render_order.rs`](../../../tests/render_order.rs) now loads world-map plus chat/voice addons together, forces `VoiceChatPromptActivateChannel` to overlap `WorldMapFrame`, and verifies the prompt renders before `WorldMapFrame.BorderFrame`.
- That means the simulator currently preserves the expected major ordering in this live-like configuration: voice prompt `LOW` strata, world map border `HIGH` strata.

Additional follow-up before the fix:

- In a live-like `1024x768` layout, [`ChatFrameChannelButton`](../../../Interface/BlizzardUI/Blizzard_ChatFrame/Mainline/FloatingChatFrameVoiceChat.xml) is visible by default and its icon atlas is `chatframe-button-icon-voicechat`.
- Its bounds overlap `WorldMapFrame` horizontally (`x=2..29` versus world map starting at `x=16`) and vertically in the lower-left of the map.
- A focused regression in [`tests/render_order.rs`](../../../tests/render_order.rs) confirmed that this button still rendered **before** `WorldMapFrame.BorderFrame`.

Why the boundary fix was no longer sufficient:

- `ChatFrameChannelButton` is `MEDIUM` strata, but its parent `ChatFrame1ButtonFrame` is `LOW`, so the button becomes its own `MEDIUM`-strata root around raw frame level 5.
- `WorldMapFrameTemplate` is `toplevel="true"`, but the panel root remains near raw frame level 1.
- Commit `783358874` correctly made explicit `Raise()`/`Lower()` use `raise_order` only as a same-raw-level tie-breaker. `set_frame_visible()` still implemented top-level auto-raise by calling that same function, so showing the map could not move its lower raw level above the chat button.
- Same-target-strata map descendants reached `MEDIUM` through intermediate `LOW` wrappers and became independent roots. The resulting bucket split the map segment: the root and some descendants appeared before the chat button while later descendants appeared after it.

Final fix:

- Keep explicit `Raise()`/`Lower()` constrained to same-level siblings.
- Track a monotonic internal show order only for active top-level frames in `SimState`; this derived state does not consume storage on every `Frame`.
- After normal per-strata emission, assign every emitted frame/region ID to its nearest active top-level ancestor across intermediate strata. Regular IDs retain their relative order, while top-level groups are appended contiguously in show order. The owning root anchors its segment and duplicate emitted IDs remain present.
- `SetToplevel(true)` on an already shown frame initializes the order. Hiding removes the active order; showing again assigns a newer order. Nested top-level ownership uses the nearest active ancestor.
- A top-level visibility transition invalidates cached buckets for a complete regroup instead of using the same-strata surgical repair path.

Verification after the final fix:

- [`src/lua_api/state_render_tests.rs`](../../../src/lua_api/state_render_tests.rs) covers the cross-strata segment, explicit lower-level `Raise()` boundary, repeated hide/show ordering, and nearest nested top-level owner; the focused state-render filter passes 8/8.
- [`tests/world_map_voice_button_order.rs`](../../../tests/world_map_voice_button_order.rs) verifies that `ChatFrameChannelButton` renders before every overlapping `WorldMapFrame` widget in the live-like `1024x768` stack; the exact regression passes 1/1 at commit `dfd997a05`.

Inference:

- The main simulator path in [`src/bin/wow_sim/addon_loading.rs`](../../../src/bin/wow_sim/addon_loading.rs) loads the discovered Blizzard addons instead of the hand-picked reduced list.
- The reduced harness in [`tests/render_order.rs`](../../../tests/render_order.rs) manually narrows the addon set, so it can omit prerequisites that the full game load normally has.

Current conclusion:

- The reduced harness issue was real and is now understood.
- A live/full-stack **voice prompt** render-order bug has **not** been reproduced by this investigation.
- The live-like `1024x768` overlap was a real render-order bug affecting `ChatFrameChannelButton`, not just a layout quirk.
- `UIParent`/`WorldFrame` boundaries remain necessary, but the final regression came from conflating top-level show ordering with explicit same-level `Raise()` semantics and then splitting cross-strata descendants into separate roots.
- If a user still sees the icon above the map in a real/full simulator run, that needs a separate reproduction against the exact frame/icon involved rather than more reduced-stack reasoning.

## Practical Fix Direction

- If a reduced stack wants to load `Blizzard_Channels`, it also needs `Blizzard_SocialToast`.
- If that stack expects alert positioning to match retail, it also needs the chat-alert system (`Blizzard_ChatFrameBase` / `Blizzard_ChatFrame`) instead of the `ChatAlertFrame` stub.
- If the goal is only world-map rendering coverage, the simpler option is to avoid pulling `Blizzard_Channels` into the reduced stack unless the voice/chat prerequisites are intentionally included too.

## Sources

- [tests/render_order.rs](../../../tests/render_order.rs) — reduced world-map addon list and startup harness
- [global_frames.rs](../../../src/lua_api/globals/global_frames.rs) — `ChatAlertFrame` stub setup
- [Blizzard_Channels.toc](../../../Interface/BlizzardUI/Blizzard_Channels/Blizzard_Channels.toc) — `Blizzard_SocialToast` dependency
- [VoiceChatPrompt.xml](../../../Interface/BlizzardUI/Blizzard_Channels/VoiceChatPrompt.xml) — voice prompt frame definitions
- [SocialToast.xml](../../../Interface/BlizzardUI/Blizzard_SocialToast/SocialToast.xml) — `SocialToastTemplate hidden="true"`
- [FloatingChatFrameAlertFrame.xml](../../../Interface/BlizzardUI/Blizzard_ChatFrame/Mainline/FloatingChatFrameAlertFrame.xml) — real `ChatAlertFrame`
- [ChatAlertFrameMixin.lua](../../../Interface/BlizzardUI/Blizzard_ChatFrameBase/Mainline/ChatAlertFrameMixin.lua) — real alert positioning behavior
- [state_render.rs](../../../src/lua_api/state_render.rs) — strata root discovery, top-level show ordering, bucket grouping, cached repair, and explicit `Raise()` ordering
- [state_render_tests.rs](../../../src/lua_api/state_render_tests.rs) — top-level cross-strata grouping and ordering boundaries
- [world_map_voice_button_order.rs](../../../tests/world_map_voice_button_order.rs) — live-like overlap regression
- [frame_collect.rs](../../../src/iced_app/frame_collect.rs) — rendered frame collection

## See Also

- [[world-map-frame-level-rebuilds]] — separate world-map-specific investigation
- [[transparent-wrapper-render-order]] — real world-map render-order bug, unrelated to the voice alert overlay
- [[addon-loading]] — addon discovery and load-order behavior
