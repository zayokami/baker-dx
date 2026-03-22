// The dioxus prelude contains a ton of common items used in dioxus apps. It's a good idea to import wherever you
// need dioxus
use dioxus::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

use components::baker::storage::v2::AppState;
use components::baker::storage::{load_state, save_state};
use components::baker::Route;

mod components;

// We can import assets in dioxus with the `asset!` macro. This macro takes a path to an asset relative to the crate root.
// The macro returns an `Asset` type that will display as the path to the asset in the browser or a local path in desktop bundles.
const FAVICON: Asset = asset!("/assets/favicon.ico");
// The asset macro also minifies some assets like CSS and JS to make bundled smaller
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const MODAL_CSS: Asset = asset!("/assets/styling/modal.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

const FONT: Asset = asset!("/assets/SourceHanSansSC-Regular.otf");
const FONT_BENDER: Asset = asset!("/assets/bender.otf");

fn main() {
    #[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
    {
        let icon = load_window_icon();
        let cfg = dioxus::desktop::Config::new().with_icon(icon);
        dioxus::LaunchBuilder::desktop().with_cfg(cfg).launch(App);
    }

    #[cfg(any(target_arch = "wasm32", not(feature = "desktop")))]
    dioxus::launch(App);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
fn load_window_icon() -> dioxus::desktop::tao::window::Icon {
    let bytes = include_bytes!("../icons/icon.png");
    let image = image::load_from_memory(bytes)
        .expect("icon decode failed")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    dioxus::desktop::tao::window::Icon::from_rgba(rgba, width, height).expect("icon rgba failed")
}

/// App is the main component of our app. Components are the building blocks of dioxus apps. Each component is a function
/// that takes some props and returns an Element. In this case, App takes no props because it is the root of our app.
///
/// Components should be annotated with `#[component]` to support props, better error messages, and autocomplete
#[component]
fn App() -> Element {
    let app_state = use_signal(AppState::default);
    let storage_ready = use_signal(|| false);
    let load_started = use_hook(|| Rc::new(Cell::new(false)));
    let save_revision = use_hook(|| Rc::new(Cell::new(0u64)));
    let skip_initial_save = use_hook(|| Rc::new(Cell::new(false)));
    let load_started_for_effect = load_started.clone();
    let save_revision_for_load = save_revision.clone();
    let skip_initial_save_for_load = skip_initial_save.clone();
    let save_revision_for_save = save_revision.clone();
    let skip_initial_save_for_save = skip_initial_save.clone();

    use_context_provider(|| app_state);

    use_effect(move || {
        if load_started_for_effect.get() {
            return;
        }
        load_started_for_effect.set(true);

        let mut app_state = app_state;
        let mut storage_ready = storage_ready;
        let save_revision = save_revision_for_load.clone();
        let skip_initial_save = skip_initial_save_for_load.clone();
        spawn(async move {
            let loaded = load_state().await;
            save_revision.set(loaded.revision);
            skip_initial_save.set(loaded.skip_initial_save);
            app_state.set(loaded.state);
            storage_ready.set(true);
        });
    });

    use_effect(move || {
        if !storage_ready() {
            return;
        }
        let snapshot = app_state.read().clone();
        if skip_initial_save_for_save.get() {
            skip_initial_save_for_save.set(false);
            return;
        }

        let next_revision = save_revision_for_save.get().saturating_add(1);
        save_revision_for_save.set(next_revision);

        spawn(async move {
            if let Err(e) = save_state(&snapshot, next_revision).await {
                #[cfg(target_arch = "wasm32")]
                {
                    spawn(async move {
                        let _ = document::eval(&format!(
                            "console.error(\"failed to save state: {}\")",
                            e
                        ))
                        .await;
                    });
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    error!("failed to save state: {}", e);
                }
            }
        });
    });

    let font_face = format!(
        r#"
        @font-face {{
            font-family: 'Source Han Sans SC';
            src: url('/assets/{}') format('opentype');
            font-weight: normal;
            font-style: normal;
        }}"#,
        FONT.bundled().bundled_path()
    );

    let font_face_bender = format!(
        r#"
        @font-face {{
            font-family: 'Bender';
            src: url('/assets/{}') format('opentype');
            font-weight: normal;
            font-style: normal;
        }}"#,
        FONT_BENDER.bundled().bundled_path()
    );

    // The `rsx!` macro lets us define HTML inside of rust. It expands to an Element with all of our HTML inside.
    if !storage_ready() {
        return rsx! {
            document::Link { rel: "icon", href: FAVICON }
            document::Link { rel: "stylesheet", href: MAIN_CSS }
            document::Link { rel: "stylesheet", href: TAILWIND_CSS }
            document::Link { rel: "stylesheet", href: MODAL_CSS }
            document::Style { {font_face.clone()} }
            document::Style { {font_face_bender.clone()} }
            document::Title { "Baker" }

            LoadingNameCard {}
        };
    }

    rsx! {
        // In addition to element and text (which we will see later), rsx can contain other components. In this case,
        // we are using the `document::Link` component to add a link to our favicon and main CSS file into the head of our app.
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "stylesheet", href: MODAL_CSS }
        document::Script { src: "https://unpkg.com/@zumer/snapdom/dist/snapdom.js" }
        document::Style { {font_face} }
        document::Style { {font_face_bender.clone()} }
        document::Title { "Baker" }

        // The router component renders the route enum we defined above. It will handle synchronization of the URL and render
        // the layouts and components for the active route.
        Router::<Route> {}
    }
}

#[component]
fn LoadingNameCard() -> Element {
    rsx! {
        div {
            class: "w-full h-screen flex items-center justify-center bg-black overflow-hidden",
            style: "font-family: 'Bender'",

            div {
                class: "overflow-hidden shadow-[0_10px_28px_rgba(0,0,0,0.45)]",
                style: "width: 320px; background-color: rgb(220, 220, 220);",

                div { class: "flex flex-col",

                    div {
                        class: "px-3 pt-3",
                        style: "background-color: rgb(220, 220, 220);",

                        h1 { class: "text-black leading-none text-2xl", "Baker-Dx" }
                    }

                    div {
                        class: "w-full px-3",
                        style: "background-color: rgb(255, 255, 0);",

                        span { class: "block text-black text-sm", "Endfield Industries" }
                    }
                }
            }
        }
    }
}
