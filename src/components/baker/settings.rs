use crate::components::baker::layout::load_repo_config;
use crate::components::baker::models::{BackgroundMode, Operator};
use crate::components::baker::{
    avif_data_url_from_bytes, data_url_from_bytes, mime_from_filename, Route,
};
use crate::dioxus_elements::FileData;
use dioxus::prelude::*;
use uuid::Uuid;

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
                                    option { disabled: true, value: "custom_image",
                                        "自定义图片 - TODO"
                                    }
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
