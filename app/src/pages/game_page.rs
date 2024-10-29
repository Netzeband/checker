use leptos::*;
use leptos_router::{Params, use_params, use_location};
use uuid::Uuid;
use leptos_use::{use_clipboard, use_permission};

#[derive(Params, PartialEq)]
struct GameParams {
    id: Uuid
}

pub fn use_url() -> Signal<String> {
    let (url, set_url) = create_signal("".to_string());
    let location = use_location();
    create_effect(move |_| {
        set_url.set(
            window().location().origin().unwrap_or_default() + &location.pathname.get()
        );
    });

    url.into()
}

#[component]
pub fn GamePage() -> impl IntoView {
    let params = use_params::<GameParams>();
    let id = move || {
        params.with(
            move |p| {
                p.as_ref().map(|p| p.id).ok()
            }
        )
    };
    let text = {move || {id().map(|id| format!("{}", id)).unwrap_or_default()}};
    let game_url = use_url();

    view! {
        <Show
            when=move || { id().is_some() }
            fallback=|| view! {
                <ErrorMessage>
                    <div>
                        <h3 class="font-bold">Invalid game ID!</h3>
                        <div class="text-xs">Go back and create a new Game.</div>
                    </div>
                    <a class="btn btn-sm btn-error border-primary-content" href="/games">Back</a>
                </ErrorMessage>
            }
        >
            <p>This is game</p>
            <p>{move || {text()}}</p>
            <CopyToClipboardButton text=Signal::derive(game_url)/>
        </Show>
    }
}

#[component]
pub fn ErrorMessage(children: Children) -> impl IntoView {
    view! {
        <div role="alert" class="alert alert-error">
            <svg
                xmlns="http://www.w3.org/2000/svg"
                class="h-6 w-6 shrink-0 stroke-current"
                fill="none"
                viewBox="0 0 24 24"
            >
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
                />
            </svg>
            {children()}
        </div>
    }
}

#[component]
pub fn CopyToClipboardButton(
    #[prop(into)]
    text: Signal<String>
) -> impl IntoView {
    let clipboard_access = use_permission("clipboard_write");
    let clipboard = use_clipboard();

    view! {
        <p>"Clipboard Permission: " {move || clipboard_access().to_string()}</p>
        <p>"Clipboard Support: " {move || clipboard.is_supported.get()}</p>
        <p>"Text to copy: " {move || {text()}}</p>
        <button
            class="btn btn-primary"
            disabled=move || {!clipboard.is_supported.get()}
            on:click={
                let copy = clipboard.copy.clone();
                move |_| {
                    logging::log!("Hello World {}", text());
                    copy(text().as_str());
                }
            }
        >
            <Show when=move || clipboard.copied.get() fallback=move || "Copy">
                "Copied!"
            </Show>
        </button>
        /*<Show
            when=move || is_supported() || toggle()
            fallback=move || { view! { <p>"Not supported!"</p> } }
        >
            <button on:click={
                let copy = copy.clone();
                move |_| copy("Hello!")
            }>
                <Show when=move || copied.get() fallback=move || "Copy">
                    "Copied!"
                </Show>
            </button>
            <p>hello world</p>
        </Show>*/
    }
}