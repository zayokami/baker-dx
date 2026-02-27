use crate::components::baker::chat_area::{ChatArea, PendingTyping, ReplayTypingPhase};
use crate::components::baker::modals::{
    NewChatModal, NewChatSelection, ProfileModal, ReplayIntervalMode, ReplaySettings,
    ReplaySettingsModal, TutorialModal, UpdateAvailableModal,
};
use crate::components::baker::models::{
    BackgroundMode, ChatHeadStyle, Contact, Message, MessageKind, MessageReaction, Operator,
};
use crate::components::baker::sidebar::Sidebar;
use crate::components::baker::{avif_data_url_from_bytes, data_url_from_bytes, mime_from_filename};
use crate::dioxus_elements::FileData;
use chrono::Utc;
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
use reqwest::Client;
use serde::Deserialize;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::sleep;
use uuid::Uuid;

const MESSAGE_SOUND: Asset = asset!("/assets/sound/message.mp3");
const MESSAGE_SELF_SOUND: Asset = asset!("/assets/sound/message-self.mp3");

fn play_message_sound(is_self: bool) {
    let sound_src = if is_self {
        MESSAGE_SELF_SOUND.to_string()
    } else {
        MESSAGE_SOUND.to_string()
    };
    spawn(async move {
        let script = format!(
            r#"
            const audio = new Audio("{sound_src}");
            audio.volume = 0.5;
            audio.play();
        "#
        );
        let _ = document::eval(&script).await;
    });
}

#[derive(Clone, PartialEq)]
struct UpdateInfo {
    version: String,
    url: String,
}

#[derive(Deserialize)]
struct RepoConfig {
    owner: String,
    repo: String,
}

#[derive(Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    html_url: String,
}

#[derive(Clone, PartialEq)]
struct ReplayContext {
    contact_id: String,
    prev_sender_id: Option<String>,
}

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: u64) {
    TimeoutFuture::new(ms.min(u32::MAX as u64) as u32).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn sleep_ms(ms: u64) {
    sleep(Duration::from_millis(ms)).await;
}

fn schedule_animate_off_in_state(
    mut app_state: Signal<crate::components::baker::models::AppState>,
    contact_id: String,
    msg_id: String,
) {
    spawn(async move {
        sleep_ms(220).await;
        if let Some(msgs) = app_state.write().messages.get_mut(&contact_id) {
            if let Some(msg) = msgs.iter_mut().find(|m| m.id == msg_id) {
                msg.animate = false;
            }
        }
    });
}

fn schedule_animate_off_in_list(mut list: Signal<Vec<Message>>, msg_id: String) {
    spawn(async move {
        sleep_ms(220).await;
        list.with_mut(|msgs| {
            if let Some(msg) = msgs.iter_mut().find(|m| m.id == msg_id) {
                msg.animate = false;
            }
        });
    });
}

fn parse_version(input: &str) -> Option<Vec<u64>> {
    let trimmed = input.trim();
    let without_prefix = trimmed.strip_prefix('v').unwrap_or(trimmed);
    let base = without_prefix.split('-').next().unwrap_or(without_prefix);
    let mut parts = Vec::new();
    for part in base.split('.') {
        let value = part.parse::<u64>().ok()?;
        parts.push(value);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

fn is_remote_newer(local: &str, remote: &str) -> bool {
    let local_parts = match parse_version(local) {
        Some(parts) => parts,
        None => return false,
    };
    let remote_parts = match parse_version(remote) {
        Some(parts) => parts,
        None => return false,
    };
    let max_len = local_parts.len().max(remote_parts.len());
    for i in 0..max_len {
        let a = *local_parts.get(i).unwrap_or(&0);
        let b = *remote_parts.get(i).unwrap_or(&0);
        if b > a {
            return true;
        }
        if b < a {
            return false;
        }
    }
    false
}

fn load_repo_config() -> Option<RepoConfig> {
    let raw = include_str!("../../../github-list-releases-parameters.json");
    serde_json::from_str(raw).ok()
}

async fn fetch_latest_release() -> Option<ReleaseResponse> {
    let config = load_repo_config()?;
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        config.owner, config.repo
    );
    let client = Client::new();
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body = response.text().await.ok()?;
    serde_json::from_str(&body).ok()
}

#[cfg(target_arch = "wasm32")]
async fn open_url(url: String) {
    let url_json = serde_json::to_string(&url).unwrap_or_else(|_| "\"\"".to_string());
    let script = format!("window.open({url_json}, '_blank');");
    let _ = document::eval(&script).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn open_url(url: String) {
    let _ = webbrowser::open(&url);
}

#[component]
pub fn BakerLayout() -> Element {
    let mut app_state = use_context::<Signal<crate::components::baker::models::AppState>>();

    let mut show_new_chat = use_signal(|| false);
    let mut show_profile = use_signal(|| false);
    let mut show_tutorial = use_signal(|| false);

    let mut selected_contact_id = use_signal(|| Option::<String>::None);
    let mut menu_close_token = use_signal(|| 0usize);
    let mut replay_request_msg_id = use_signal(|| Option::<String>::None);
    let mut replay_active = use_signal(|| Option::<ReplayContext>::None);
    let mut replay_messages = use_signal(Vec::<Message>::new);
    let mut replay_token = use_signal(|| 0usize);
    let mut replay_pending = use_signal(|| Option::<PendingTyping>::None);
    let mut update_info = use_signal(|| Option::<UpdateInfo>::None);
    let mut update_checked = use_signal(|| false);

    let navigator = use_navigator();

    use_effect(move || {
        if update_checked() {
            return;
        }
        update_checked.set(true);
        let mut update_info = update_info;
        let app_state = app_state;
        spawn(async move {
            let latest = match fetch_latest_release().await {
                Some(release) => release,
                None => return,
            };
            let local_version = env!("CARGO_PKG_VERSION");
            if !is_remote_newer(local_version, &latest.tag_name) {
                return;
            }
            let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
            let snooze_date = app_state.read().update_snooze_date.clone();
            if snooze_date.as_deref() == Some(today.as_str()) {
                return;
            }
            update_info.set(Some(UpdateInfo {
                version: latest.tag_name,
                url: latest.html_url,
            }));
        });
    });

    let mut add_message = move |sender_id: String, content: String, kind: MessageKind| {
        let current_contact_id = match selected_contact_id() {
            Some(id) => id,
            None => return,
        };

        let is_self = sender_id == app_state.read().user_profile.id;
        let new_id = {
            let mut state = app_state.write();
            let messages = state
                .messages
                .entry(current_contact_id.clone())
                .or_default();
            let new_id = Uuid::new_v4().to_string();

            messages.push(Message {
                id: new_id.clone(),
                sender_id,
                content,
                kind,
                animate: true,
                reactions: Vec::new(),
            });
            new_id
        };
        play_message_sound(is_self);
        schedule_animate_off_in_state(app_state, current_contact_id, new_id);
    };

    let handle_send = move |content: String| {
        let user_id = app_state.read().user_profile.id.clone();
        add_message(user_id, content, MessageKind::Normal);
    };

    let mut handle_send_other = move |sender_id: String, content: String| {
        add_message(sender_id, content, MessageKind::Normal);
    };

    let handle_send_status = move |content: String| {
        let user_id = app_state.read().user_profile.id.clone();
        add_message(user_id, content, MessageKind::Status);
    };

    let handle_send_image = move |data_url: String| {
        let user_id = app_state.read().user_profile.id.clone();
        add_message(user_id, data_url, MessageKind::Image);
    };

    let handle_send_sticker = move |sticker_src: String| {
        let user_id = app_state.read().user_profile.id.clone();
        add_message(user_id, sticker_src, MessageKind::Sticker);
    };

    let mut handle_send_sticker_other = move |sender_id: String, sticker_src: String| {
        add_message(sender_id, sticker_src, MessageKind::Sticker);
    };
    let handle_add_sticker = move |sticker_src: String| {
        let trimmed = sticker_src.trim();
        if trimmed.is_empty() {
            return;
        }
        let mut state = app_state.write();
        if !state.stickers.iter().any(|s| s == trimmed) {
            state.stickers.push(trimmed.to_string());
        }
    };

    let selected_contact = {
        let selected_id = selected_contact_id();
        app_state
            .read()
            .contacts
            .iter()
            .find(|c| {
                if let Some(id) = selected_id.as_ref() {
                    id == &c.id
                } else {
                    false
                }
            })
            .cloned()
    };

    // Derived signals for Sidebar
    let contacts = use_memo(move || app_state.read().contacts.clone());
    let stickers = use_memo(move || app_state.read().stickers.clone());

    // Derived signals for ChatArea
    let messages = use_memo(move || {
        if let Some(id) = selected_contact_id() {
            if let Some(replay) = replay_active() {
                if replay.contact_id == id {
                    return replay_messages();
                }
            }
            app_state
                .read()
                .messages
                .get(&id)
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    });

    use_effect(move || {
        let current = selected_contact_id();
        if let Some(id) = current {
            let mut state = app_state.write();
            if let Some(msgs) = state.messages.get_mut(&id) {
                for msg in msgs.iter_mut() {
                    msg.animate = false;
                }
            }
        }
    });

    // Derived signals for Modals
    let operators = use_signal(move || app_state.read().operators.clone());
    // Sync operators back to state when modified in modal (workaround for SettingsModal signature)
    use_effect(move || {
        let current_ops = operators.read();
        if *current_ops != app_state.read().operators {
            app_state.write().operators = current_ops.clone();
        }
    });
    let background = use_signal(move || app_state.read().background.clone());
    use_effect(move || {
        let current_background = background.read();
        if *current_background != app_state.read().background {
            app_state.write().background = current_background.clone();
        }
    });

    let add_contact = move |selection: NewChatSelection| {
        let mut state = app_state.write();
        match selection {
            NewChatSelection::Single(op) => {
                let op_id = op.id.clone();
                if !state.contacts.iter().any(|c| c.id == op_id) {
                    state.contacts.push(Contact {
                        id: op_id.clone(),
                        unread_count: 0,
                        chat_head_style: ChatHeadStyle::Default,
                        name: op.name.clone(),
                        avatar_url: op.avatar_url.clone(),
                        participant_ids: vec![op_id.clone()],
                        is_group: false,
                    });
                }
                selected_contact_id.set(Some(op_id));
                show_new_chat.set(false);
            }
            NewChatSelection::Group {
                name,
                avatar_url,
                member_ids,
            } => {
                let group_id = Uuid::new_v4().to_string();
                state.contacts.push(Contact {
                    id: group_id.clone(),
                    unread_count: 0,
                    chat_head_style: ChatHeadStyle::Default,
                    name,
                    avatar_url,
                    participant_ids: member_ids,
                    is_group: true,
                });
                selected_contact_id.set(Some(group_id));
                show_new_chat.set(false);
            }
        }
    };

    // Helper to update user profile
    let update_profile = move |(name, avatar): (String, String)| {
        let mut state = app_state.write();
        state.user_profile.name = name;
        state.user_profile.avatar_url = avatar;
        show_profile.set(false);
    };

    // Helper to delete message
    let delete_message = move |msg_id: String| {
        if let Some(contact_id) = selected_contact_id() {
            let mut state = app_state.write();
            if let Some(msgs) = state.messages.get_mut(&contact_id) {
                msgs.retain(|m| m.id != msg_id);
            }
        }
    };

    // Helper to edit message
    let edit_message = move |(msg_id, new_content): (String, String)| {
        if let Some(contact_id) = selected_contact_id() {
            let mut state = app_state.write();
            if let Some(msgs) = state.messages.get_mut(&contact_id) {
                if let Some(msg) = msgs.iter_mut().find(|m| m.id == msg_id) {
                    msg.content = new_content;
                }
            }
        }
    };

    let add_reaction = move |(msg_id, reaction): (String, String)| {
        let reaction = reaction.trim().to_string();
        if reaction.is_empty() {
            return;
        }
        let sender_id = app_state.read().user_profile.id.clone();
        if let Some(contact_id) = selected_contact_id() {
            let mut state = app_state.write();
            if let Some(msgs) = state.messages.get_mut(&contact_id) {
                if let Some(msg) = msgs.iter_mut().find(|m| m.id == msg_id) {
                    msg.reactions.push(MessageReaction {
                        content: reaction,
                        sender_id,
                    });
                }
            }
        }
    };

    let delete_reaction = move |msg_id: String| {
        if let Some(contact_id) = selected_contact_id() {
            let mut state = app_state.write();
            if let Some(msgs) = state.messages.get_mut(&contact_id) {
                if let Some(msg) = msgs.iter_mut().find(|m| m.id == msg_id) {
                    msg.reactions.clear();
                }
            }
        }
    };

    let insert_message =
        move |(before_id, content, sender_id_opt): (String, String, Option<String>)| {
            if let Some(contact_id) = selected_contact_id() {
                let sender_id = match sender_id_opt {
                    // 我方
                    None => app_state.read().user_profile.id.clone(),
                    // 指定发送者（单聊对方或群组选定成员）
                    Some(id) => id,
                };
                let new_id = {
                    let mut state = app_state.write();
                    let messages = state.messages.entry(contact_id.clone()).or_default();
                    let new_id = Uuid::new_v4().to_string();
                    let insert_index = messages
                        .iter()
                        .position(|m| m.id == before_id)
                        .unwrap_or(messages.len());

                    messages.insert(
                        insert_index,
                        Message {
                            id: new_id.clone(),
                            sender_id,
                            content,
                            kind: MessageKind::Normal,
                            animate: true,
                            reactions: Vec::new(),
                        },
                    );
                    new_id
                };

                schedule_animate_off_in_state(app_state, contact_id, new_id);
            }
        };

    let update_chat_head_style = move |style: ChatHeadStyle| {
        if let Some(contact_id) = selected_contact_id() {
            let mut state = app_state.write();
            if let Some(contact) = state.contacts.iter_mut().find(|c| c.id == contact_id) {
                contact.chat_head_style = style;
            }
        }
    };

    let mut cancel_replay = {
        let mut replay_active = replay_active;
        let mut replay_messages = replay_messages;
        let mut replay_token = replay_token;
        let mut replay_pending = replay_pending;
        move || {
            replay_token.set(replay_token() + 1);
            replay_active.set(None);
            replay_messages.set(Vec::new());
            replay_pending.set(None);
        }
    };

    let mut clear_messages = {
        let mut cancel_replay = cancel_replay;
        let mut app_state = app_state;
        let selected_contact_id = selected_contact_id;
        move || {
            if let Some(contact_id) = selected_contact_id() {
                let mut state = app_state.write();
                state.messages.insert(contact_id, Vec::new());
                cancel_replay();
            }
        }
    };

    let mut clear_chat = {
        let mut cancel_replay = cancel_replay;
        let mut app_state = app_state;
        let mut selected_contact_id = selected_contact_id;
        move || {
            if let Some(contact_id) = selected_contact_id() {
                let mut state = app_state.write();
                state.messages.remove(&contact_id);
                state.contacts.retain(|c| c.id != contact_id);
                selected_contact_id.set(None);
                cancel_replay();
            }
        }
    };

    use_effect(move || {
        let current = selected_contact_id();
        if let Some(replay) = replay_active() {
            if Some(replay.contact_id) != current {
                replay_token.set(replay_token() + 1);
                replay_active.set(None);
                replay_messages.set(Vec::new());
                replay_pending.set(None);
            }
        }
    });

    let mut start_replay = {
        let mut replay_messages = replay_messages;
        let mut replay_active = replay_active;
        let mut replay_token = replay_token;
        let mut replay_pending = replay_pending;
        let app_state = app_state;
        let selected_contact_id = selected_contact_id;
        move |start_msg_id: String, settings: ReplaySettings| {
            let contact_id = match selected_contact_id() {
                Some(id) => id,
                None => return,
            };
            let all_messages = app_state
                .read()
                .messages
                .get(&contact_id)
                .cloned()
                .unwrap_or_default();
            let start_index = all_messages
                .iter()
                .position(|m| m.id == start_msg_id)
                .unwrap_or(0);
            let prev_sender_id = if start_index > 0 {
                Some(all_messages[start_index - 1].sender_id.clone())
            } else {
                None
            };

            let token = replay_token() + 1;
            replay_token.set(token);
            replay_messages.set(Vec::new());
            replay_pending.set(None);
            replay_active.set(Some(ReplayContext {
                contact_id,
                prev_sender_id,
            }));

            let mut replay_messages_async = replay_messages;
            let replay_token_async = replay_token;
            let mut replay_pending_async = replay_pending;
            let settings_clone = settings.clone();
            let user_id = app_state.read().user_profile.id.clone();
            spawn(async move {
                for msg in all_messages.into_iter().skip(start_index) {
                    if replay_token_async() != token {
                        break;
                    }
                    if settings_clone.gap_ms > 0 {
                        sleep_ms(settings_clone.gap_ms).await;
                    }
                    if matches!(msg.kind, MessageKind::Status) {
                        replay_messages_async.with_mut(|list| {
                            list.push(Message {
                                animate: true,
                                ..msg.clone()
                            });
                        });
                        let is_self = msg.sender_id == user_id;
                        play_message_sound(is_self);
                        schedule_animate_off_in_list(replay_messages_async, msg.id);
                        continue;
                    }
                    let typing_ms = if matches!(msg.kind, MessageKind::Image) {
                        // 图片消息内容是 data URL，字符数极大，按字数模式下强制走固定时长
                        settings_clone.fixed_ms
                    } else {
                        match settings_clone.mode {
                            ReplayIntervalMode::Fixed => settings_clone.fixed_ms,
                            ReplayIntervalMode::PerChar => {
                                let len = msg.content.chars().count() as u64;
                                len.saturating_mul(settings_clone.per_char_ms)
                            }
                        }
                    };
                    let is_other = msg.sender_id != user_id;
                    if is_other {
                        let msg_id = msg.id.clone();
                        replay_messages_async.with_mut(|list| {
                            list.push(Message {
                                animate: true,
                                reactions: Vec::new(),
                                ..msg.clone()
                            });
                        });
                        replay_pending_async.set(Some(PendingTyping {
                            id: msg_id.clone(),
                            phase: ReplayTypingPhase::Typing,
                        }));
                        schedule_animate_off_in_list(replay_messages_async, msg_id.clone());
                        if typing_ms > 0 {
                            sleep_ms(typing_ms).await;
                        }
                        replay_pending_async.set(Some(PendingTyping {
                            id: msg_id,
                            phase: ReplayTypingPhase::Reveal,
                        }));
                        play_message_sound(false);
                        sleep_ms(200).await;
                        replay_pending_async.set(None);
                        if !msg.reactions.is_empty() {
                            if settings_clone.gap_ms > 0 {
                                sleep_ms(settings_clone.gap_ms).await;
                            }
                            let reactions = msg.reactions.clone();
                            let msg_id = msg.id.clone();
                            replay_messages_async.with_mut(|list| {
                                if let Some(item) = list.iter_mut().find(|m| m.id == msg_id) {
                                    item.reactions = reactions;
                                }
                            });
                        }
                    } else {
                        replay_pending_async.set(None);
                        if typing_ms > 0 {
                            sleep_ms(typing_ms).await;
                        }
                        replay_messages_async.with_mut(|list| {
                            list.push(Message {
                                animate: true,
                                reactions: Vec::new(),
                                ..msg.clone()
                            });
                        });
                        play_message_sound(true);
                        schedule_animate_off_in_list(replay_messages_async, msg.id.clone());
                        if !msg.reactions.is_empty() {
                            if settings_clone.gap_ms > 0 {
                                sleep_ms(settings_clone.gap_ms).await;
                            }
                            let reactions = msg.reactions.clone();
                            let msg_id = msg.id.clone();
                            replay_messages_async.with_mut(|list| {
                                if let Some(item) = list.iter_mut().find(|m| m.id == msg_id) {
                                    item.reactions = reactions;
                                }
                            });
                        }
                    }
                }
                replay_pending_async.set(None);
            });
        }
    };

    let user_profile = app_state.read().user_profile.clone();
    let hide_tutorial = app_state.read().hide_tutorial;
    let replay_pending_for_contact = use_memo(move || {
        if let Some(replay) = replay_active() {
            if let Some(selected_id) = selected_contact_id() {
                if replay.contact_id == selected_id {
                    return replay_pending();
                }
            }
        }
        None
    });
    let background_style = use_memo(move || {
        let bg = app_state.read().background.clone();
        match bg.mode {
            BackgroundMode::DotDark => {
                "background-color: #1a1a1a; background-image: radial-gradient(#2a2a2a 1px, transparent 1px); background-size: 20px 20px;".to_string()
            }
            BackgroundMode::DotLight => {
                "background-color: #f2f2f2; background-image: radial-gradient(#d0d0d0 1px, transparent 1px); background-size: 20px 20px;".to_string()
            }
            BackgroundMode::CustomColor => format!("background-color: {};", bg.custom_color),
            BackgroundMode::CustomImage => {
                if bg.custom_image.is_empty() {
                    format!("background-color: {};", bg.custom_color)
                } else {
                    format!("background-image: url({}); background-size: cover; background-position: center; background-repeat: no-repeat; background-color: #1a1a1a;", bg.custom_image)
                }
            }
        }
    });

    rsx! {
        div {
            class: "w-full h-screen bg-cover bg-center flex flex-col overflow-hidden text-sans",
            style: "{background_style}",
            onclick: move |_| menu_close_token.set(menu_close_token() + 1),

            // Modals
            if show_new_chat() {
                NewChatModal {
                    operators,
                    on_close: move |_| show_new_chat.set(false),
                    on_select: add_contact,
                }
            }

            if show_profile() {
                ProfileModal {
                    current_name: user_profile.name.clone(),
                    current_avatar: user_profile.avatar_url.clone(),
                    on_close: move |_| show_profile.set(false),
                    on_save: update_profile,
                }
            }
            if show_tutorial() {
                TutorialModal {
                    on_close: move |_| show_tutorial.set(false),
                    on_confirm: move |dont_show| {
                        if dont_show {
                            app_state.write().hide_tutorial = true;
                        }
                        show_tutorial.set(false);
                    },
                }
            }

            if replay_request_msg_id().is_some() {
                ReplaySettingsModal {
                    on_close: move |_| replay_request_msg_id.set(None),
                    on_start: move |settings| {
                        if let Some(msg_id) = replay_request_msg_id() {
                            start_replay(msg_id, settings);
                        }
                        replay_request_msg_id.set(None);
                    },
                }
            }
            if let Some(info) = update_info() {
                UpdateAvailableModal {
                    latest_version: info.version.clone(),
                    release_url: info.url.clone(),
                    on_close: move |_| update_info.set(None),
                    on_skip_today: move |_| {
                        let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
                        app_state.write().update_snooze_date = Some(today);
                        update_info.set(None);
                    },
                    on_update_now: move |url| {
                        update_info.set(None);
                        spawn(async move {
                            open_url(url).await;
                        });
                    },
                }
            }

            // 顶部导航栏
            div { class: "h-16 flex items-center px-8 justify-between shrink-0 z-10",
                div { class: "flex items-center gap-4",
                    span {
                        class: "text-white text-base font-bold cursor-pointer select-none hover:text-gray-300 transition-colors",
                        ondoubleclick: move |_| {
                            navigator.push(Route::SettingsPage {});
                        },
                        "//BAKER/好友沟通"
                    }
                    if !hide_tutorial {
                        a {
                            class: "text-blue-300 text-sm underline hover:text-blue-200 transition-colors",
                            href: "#",
                            onclick: move |evt| {
                                evt.prevent_default();
                                show_tutorial.set(true);
                            },
                            "点击这里看教程！！"
                        }
                    }
                }

                // Profile Button
                div {
                    class: "flex items-center gap-3 cursor-pointer hover:bg-white/5 p-2 rounded-lg transition-colors",
                    onclick: move |_| show_profile.set(true),
                    span { class: "text-gray-300 text-sm", "{user_profile.name}" }
                    div { class: "w-8 h-8 rounded bg-gray-600 overflow-hidden border border-gray-500",
                        if !user_profile.avatar_url.is_empty() {
                            img {
                                src: "{user_profile.avatar_url}",
                                class: "w-full h-full object-cover",
                            }
                        } else {
                            div { class: "w-full h-full flex items-center justify-center text-white text-xs font-bold",
                                "{user_profile.name.chars().next().unwrap_or('?')}"
                            }
                        }
                    }
                }
            }

            // 主内容区
            div { class: "flex-1 flex overflow-hidden p-8 gap-8 min-h-0",

                // 左侧栏
                Sidebar {
                    contacts,
                    operators,
                    selected_contact_id,
                    on_add_click: move |_| {
                        cancel_replay();
                        show_new_chat.set(true);
                    },
                }

                // 右侧聊天区
                if let Some(contact) = selected_contact {
                    {
                        let replay_prev_sender_id = replay_active()
                            .and_then(|replay| {
                                if replay.contact_id == contact.id {
                                    replay.prev_sender_id
                                } else {
                                    None
                                }
                            });
                        let force_first_avatar = replay_active()
                            .map(|replay| replay.contact_id == contact.id)
                            .unwrap_or(false);
                        rsx! {
                            ChatArea {
                                contact,
                                operators,
                                messages,
                                user_profile,
                                menu_close_token,
                                first_prev_sender_id: replay_prev_sender_id,
                                force_first_avatar,
                                pending_typing: replay_pending_for_contact,
                                on_send_message: handle_send,
                                on_send_other_message: move |(sender_id, text)| {
                                    handle_send_other(sender_id, text);
                                },
                                on_send_status: handle_send_status,
                                on_send_image: handle_send_image,
                                on_send_sticker: handle_send_sticker,
                                on_send_sticker_other: move |(sender_id, sticker)| {
                                    handle_send_sticker_other(sender_id, sticker);
                                },
                                stickers,
                                on_add_sticker: handle_add_sticker,
                                on_delete_message: delete_message,
                                on_edit_message: edit_message,
                                on_add_reaction: add_reaction,
                                on_delete_reaction: delete_reaction,
                                on_insert_message: insert_message,
                                on_start_replay: move |msg_id| replay_request_msg_id.set(Some(msg_id)),
                                on_update_chat_head_style: update_chat_head_style,
                                on_clear_messages: move |_| clear_messages(),
                                on_clear_chat: move |_| clear_chat(),
                            }
                        }
                    }
                } else {
                    div { class: "flex-1 flex items-center justify-center text-gray-500 bg-gray-900/50 border border-gray-600 backdrop-blur-sm",
                        "请选择一个会话"
                    }
                }
            }
        }
    }
}

#[component]
pub fn SettingsPage() -> Element {
    let mut app_state = use_context::<Signal<crate::components::baker::models::AppState>>();

    let repo_config = load_repo_config().unwrap();
    let repo_url = format!(
        "https://github.com/{}/{}",
        repo_config.owner, repo_config.repo
    );

    let mut operators = use_signal(move || app_state.read().operators.clone());
    use_effect(move || {
        let current_ops = operators.read();
        if *current_ops != app_state.read().operators {
            app_state.write().operators = current_ops.clone();
        }
    });
    let mut background = use_signal(move || app_state.read().background.clone());
    use_effect(move || {
        let current_background = background.read();
        if *current_background != app_state.read().background {
            app_state.write().background = current_background.clone();
        }
    });

    let mut new_name = use_signal(|| "".to_string());
    let mut new_avatar_preview = use_signal(|| "".to_string());
    let mut editing_operator_id = use_signal(|| Option::<String>::None);
    let mut edit_name = use_signal(|| "".to_string());
    let mut edit_avatar_preview = use_signal(|| "".to_string());

    #[derive(Clone, PartialEq)]
    enum SettingsSection {
        Operators,
        Background,
        About,
    }

    let mut section = use_signal(|| SettingsSection::Operators);

    let handle_add = move |_| {
        let name = new_name();
        let avatar = new_avatar_preview();
        if !name.is_empty() {
            let id = Uuid::new_v4().to_string();
            operators.write().push(Operator {
                id,
                name,
                avatar_url: avatar,
            });
            new_name.set("".to_string());
            new_avatar_preview.set("".to_string());
        }
    };

    let mut handle_delete = move |id: String| {
        operators.write().retain(|op| op.id != id);
    };
    let mut handle_edit_start = move |op: Operator| {
        editing_operator_id.set(Some(op.id.clone()));
        edit_name.set(op.name);
        edit_avatar_preview.set(op.avatar_url);
    };
    let mut handle_edit_cancel = move |_| {
        editing_operator_id.set(None);
    };
    let mut handle_edit_save = move |id: String| {
        let name = edit_name();
        let avatar = edit_avatar_preview();
        if let Some(op) = operators.write().iter_mut().find(|op| op.id == id) {
            op.name = name.clone();
            op.avatar_url = avatar.clone();
        }
        let mut state = app_state.write();
        if let Some(contact) = state.contacts.iter_mut().find(|c| c.id == id) {
            contact.name = name;
            contact.avatar_url = avatar;
        }
        editing_operator_id.set(None);
    };

    let ops_list = operators.read().clone();
    let current_background = background.read().clone();
    let background_mode_value = match current_background.mode {
        BackgroundMode::DotDark => "dot_dark",
        BackgroundMode::DotLight => "dot_light",
        BackgroundMode::CustomColor => "custom_color",
        BackgroundMode::CustomImage => "custom_image",
    };
    let operators_tab_class = if matches!(section(), SettingsSection::Operators) {
        "bg-[#2b2b2b] text-white"
    } else {
        "text-gray-400 hover:text-white hover:bg-white/5"
    };
    let background_tab_class = if matches!(section(), SettingsSection::Background) {
        "bg-[#2b2b2b] text-white"
    } else {
        "text-gray-400 hover:text-white hover:bg-white/5"
    };
    let about_tab_class = if matches!(section(), SettingsSection::About) {
        "bg-[#2b2b2b] text-white"
    } else {
        "text-gray-400 hover:text-white hover:bg-white/5"
    };

    let background_style = use_memo(move || {
        let bg = background.read().clone();
        match bg.mode {
            BackgroundMode::DotDark => {
                "background-color: #1a1a1a; background-image: radial-gradient(#2a2a2a 1px, transparent 1px); background-size: 20px 20px;".to_string()
            }
            BackgroundMode::DotLight => {
                "background-color: #f2f2f2; background-image: radial-gradient(#d0d0d0 1px, transparent 1px); background-size: 20px 20px;".to_string()
            }
            BackgroundMode::CustomColor => format!("background-color: {};", bg.custom_color),
            BackgroundMode::CustomImage => {
                if bg.custom_image.is_empty() {
                    format!("background-color: {};", bg.custom_color)
                } else {
                    format!("background-image: url({}); background-size: cover; background-position: center; background-repeat: no-repeat; background-color: #1a1a1a;", bg.custom_image)
                }
            }
        }
    });

    let navigator = use_navigator();

    rsx! {
        div {
            class: "w-full h-screen bg-cover bg-center flex flex-col overflow-hidden text-sans",
            style: "{background_style}",
            div { class: "h-14 flex items-center gap-3 px-6 border-b border-gray-600 bg-[#1f1f1f]/80 backdrop-blur-sm",
                button {
                    class: "text-gray-300 hover:text-white text-lg px-2 py-1 rounded-lg hover:bg-white/5 transition-colors",
                    onclick: move |_| {
                        navigator.push(Route::BakerLayout {});
                    },
                    "←"
                }
                h1 { class: "text-white text-lg font-bold", "设置" }
            }
            div { class: "flex-1 flex min-h-0",
                div { class: "w-64 shrink-0 border-r border-gray-700 bg-[#1f1f1f]/70 p-4",
                    div { class: "space-y-2",
                        button {
                            class: "w-full text-left px-3 py-2 rounded-lg text-sm transition-colors {operators_tab_class}",
                            onclick: move |_| section.set(SettingsSection::Operators),
                            "干员管理"
                        }
                        button {
                            class: "w-full text-left px-3 py-2 rounded-lg text-sm transition-colors {background_tab_class}",
                            onclick: move |_| section.set(SettingsSection::Background),
                            "背景设置"
                        }
                        button {
                            class: "w-full text-left px-3 py-2 rounded-lg text-sm transition-colors {about_tab_class}",
                            onclick: move |_| section.set(SettingsSection::About),
                            "关于"
                        }
                    }
                }
                div { class: "flex-1 min-h-0 overflow-y-auto p-8",
                    if matches!(section(), SettingsSection::Operators) {
                        div { class: "max-w-[820px] space-y-6",
                            div { class: "p-4 bg-[#2b2b2b] rounded-xl border border-gray-600",
                                h2 { class: "text-white text-base font-bold mb-3",
                                    "添加新干员"
                                }
                                div { class: "space-y-3 mb-3",
                                    input {
                                        class: "w-full bg-[#222] border border-gray-600 rounded px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500",
                                        placeholder: "干员代号 (Name)",
                                        value: "{new_name}",
                                        oninput: move |e| new_name.set(e.value()),
                                    }
                                    input {
                                        class: "w-full bg-[#222] border border-gray-600 rounded px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500",
                                        r#type: "file",
                                        accept: "image/*",
                                        onchange: move |evt| {
                                            let files: Vec<FileData> = evt.files();
                                            if let Some(file) = files.first().cloned() {
                                                let file_name: String = file.name();
                                                let mime = file
                                                    .content_type()
                                                    .unwrap_or_else(|| mime_from_filename(&file_name).to_string());
                                                let mut preview = new_avatar_preview;
                                                spawn(async move {
                                                    if let Ok(bytes) = file.read_bytes().await {
                                                        let bytes_vec = bytes.to_vec();
                                                        let data_url = avif_data_url_from_bytes(bytes_vec.clone())
                                                            .unwrap_or_else(|| data_url_from_bytes(&mime, bytes_vec));
                                                        preview.set(data_url);
                                                    }
                                                });
                                            }
                                        },
                                    }
                                }
                                button {
                                    class: "w-full bg-blue-600 hover:bg-blue-500 text-white py-2 rounded text-sm font-medium transition-colors",
                                    onclick: handle_add,
                                    "添加干员"
                                }
                            }

                            div { class: "space-y-2",
                                for op in ops_list {
                                    {
                                        let op_id = op.id.clone();
                                        let op_clone = op.clone();
                                        if editing_operator_id() == Some(op_id.clone()) {
                                            rsx! {
                                                div { class: "p-4 bg-[#2b2b2b] rounded border border-gray-700 space-y-3",
                                                    div { class: "flex items-center gap-3",
                                                        div { class: "w-12 h-12 rounded bg-gray-600 flex items-center justify-center overflow-hidden",
                                                            if !edit_avatar_preview().is_empty() {
                                                                img {
                                                                    src: "{edit_avatar_preview}",
                                                                    class: "w-full h-full object-cover",
                                                                }
                                                            } else {
                                                                span { class: "text-white font-bold", "{edit_name.read().chars().next().unwrap_or('?')}" }
                                                            }
                                                        }
                                                        input {
                                                            class: "flex-1 bg-[#222] border border-gray-600 rounded px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500",
                                                            value: "{edit_name}",
                                                            oninput: move |e| edit_name.set(e.value()),
                                                        }
                                                    }
                                                    input {
                                                        class: "w-full bg-[#222] border border-gray-600 rounded px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500",
                                                        r#type: "file",
                                                        accept: "image/*",
                                                        onchange: move |evt| {
                                                            let files: Vec<FileData> = evt.files();
                                                            if let Some(file) = files.first().cloned() {
                                                                let file_name: String = file.name();
                                                                let mime = file
                                                                    .content_type()
                                                                    .unwrap_or_else(|| mime_from_filename(&file_name).to_string());
                                                                let mut preview = edit_avatar_preview;
                                                                spawn(async move {
                                                                    if let Ok(bytes) = file.read_bytes().await {
                                                                        let bytes_vec = bytes.to_vec();
                                                                        let data_url = avif_data_url_from_bytes(bytes_vec.clone())
                                                                            .unwrap_or_else(|| data_url_from_bytes(&mime, bytes_vec));
                                                                        preview.set(data_url);
                                                                    }
                                                                });
                                                            }
                                                        },
                                                    }
                                                    div { class: "flex justify-end gap-3",
                                                        button {
                                                            class: "px-3 py-1 text-gray-400 hover:text-white text-sm",
                                                            onclick: move |_| handle_edit_cancel(()),
                                                            "取消"
                                                        }
                                                        button {
                                                            class: "px-3 py-1 bg-blue-600 hover:bg-blue-500 text-white rounded text-sm font-medium",
                                                            onclick: move |_| handle_edit_save(op_id.clone()),
                                                            "保存"
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                div { class: "flex items-center justify-between p-3 bg-[#2b2b2b] rounded border border-gray-700",
                                                    div { class: "flex items-center gap-3",
                                                        div { class: "w-10 h-10 rounded bg-gray-600 flex items-center justify-center overflow-hidden",
                                                            if !op.avatar_url.is_empty() {
                                                                img {
                                                                    src: "{op.avatar_url}",
                                                                    class: "w-full h-full object-cover",
                                                                }
                                                            } else {
                                                                span { class: "text-white font-bold", "{op.name.chars().next().unwrap_or('?')}" }
                                                            }
                                                        }
                                                        span { class: "text-white font-medium", "{op.name}" }
                                                    }
                                                    div { class: "flex items-center gap-3",
                                                        button {
                                                            class: "text-gray-300 hover:text-white text-sm px-2 py-1",
                                                            onclick: move |_| handle_edit_start(op_clone.clone()),
                                                            "编辑"
                                                        }
                                                        button {
                                                            class: "text-red-400 hover:text-red-300 text-sm px-2 py-1",
                                                            onclick: move |_| handle_delete(op_id.clone()),
                                                            "删除"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if matches!(section(), SettingsSection::Background) {
                        div { class: "max-w-[820px] space-y-6",
                            h2 { class: "text-white text-base font-bold", "背景设置" }
                            div { class: "space-y-3",
                                select {
                                    class: "w-full bg-[#222] border border-gray-600 rounded px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500",
                                    value: "{background_mode_value}",
                                    oninput: move |e| {
                                        let mut bg = background.write();
                                        bg.mode = match e.value().as_str() {
                                            "dot_light" => BackgroundMode::DotLight,
                                            "custom_color" => BackgroundMode::CustomColor,
                                            "custom_image" => BackgroundMode::CustomImage,
                                            _ => BackgroundMode::DotDark,
                                        };
                                    },
                                    option { value: "dot_dark", "点状-深色" }
                                    option { value: "dot_light", "点状-浅色" }
                                    option { value: "custom_color", "自定义颜色" }
                                    option { value: "custom_image", "自定义图片" }
                                }
                                if matches!(current_background.mode, BackgroundMode::CustomColor) {
                                    div { class: "flex items-center gap-3",
                                        input {
                                            class: "w-24 h-10 bg-transparent border border-gray-600 rounded",
                                            r#type: "color",
                                            value: "{current_background.custom_color}",
                                            oninput: move |e| {
                                                let mut bg = background.write();
                                                bg.custom_color = e.value();
                                                bg.mode = BackgroundMode::CustomColor;
                                            },
                                        }
                                    }
                                }
                                if matches!(current_background.mode, BackgroundMode::CustomImage) {
                                    div { class: "flex items-center gap-3",
                                        input {
                                            class: "flex-1 bg-[#222] border border-gray-600 rounded px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500",
                                            r#type: "file",
                                            accept: "image/*",
                                            onchange: move |evt| {
                                                let files: Vec<FileData> = evt.files();
                                                if let Some(file) = files.first().cloned() {
                                                    let file_name: String = file.name();
                                                    let mime = file
                                                        .content_type()
                                                        .unwrap_or_else(|| mime_from_filename(&file_name).to_string());
                                                    let mut bg = background;
                                                    spawn(async move {
                                                        if let Ok(bytes) = file.read_bytes().await {
                                                            let bytes_vec = bytes.to_vec();
                                                            let data_url = avif_data_url_from_bytes(bytes_vec.clone())
                                                                .unwrap_or_else(|| data_url_from_bytes(&mime, bytes_vec));
                                                            let mut settings = bg.write();
                                                            settings.custom_image = data_url;
                                                            settings.mode = BackgroundMode::CustomImage;
                                                        }
                                                    });
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    } else if matches!(section(), SettingsSection::About) {
                        div {
                            h1 { class: "text-4xl font-bold", "Baker" }
                            h2 { class: "text-xl mt-2 mb-10",
                                "用以还原《明日方舟：终末地》中Baker的应用。"
                            }

                            p { "项目作者：Wanye_7300" }
                            div {
                                "开源在："
                                a {
                                    class: "text-blue-500 hover:underline",
                                    href: repo_url,
                                    {repo_url.clone()}
                                }
                            }

                            h2 { class: "text-xl mt-10 mb-2", "开源协议" }
                            p { "本项目基于 MIT 协议开源。" }
                            p { class: "font-mono", "MIT License" }
                            p { class: "font-mono", "Copyright (c) 2026 Wanye_7300" }
                            p { class: "font-mono",
                                "Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the \"Software\"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:"
                            }
                            p { class: "font-mono",
                                "The above copyright notice and this permission notice (including the next paragraph) shall be included in all copies or substantial portions of the Software."
                            }
                            p { class: "font-mono",
                                "THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE."
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/")]
    BakerLayout {},
    #[route("/settings")]
    SettingsPage {},
}
