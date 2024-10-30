use leptos::*;
use leptos_router::{Params, use_params, use_location};
use uuid::Uuid;
use leptos_use::{use_clipboard, use_cookie_with_options, SameSite, UseCookieOptions};
use codee::string::FromToStringCodec;
use serde::{Deserialize, Serialize};

#[derive(Params, PartialEq)]
struct GameParams {
    id: Uuid
}

pub fn use_url() -> Signal<String> {
    let (url, set_url) = create_signal("".to_string());
    let location = use_location();
    create_effect(move |_| {
        set_url.set(
            window().location().origin().unwrap_or_default() + location.pathname.get().as_str()
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
            <GameInfo game_id=Signal::derive(id)/>
            <PlayerAssignment/>
        </Show>
    }
}

#[derive(Deserialize, Serialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum PlayerAssignmentStatus {
    REFUSED,
    ACCEPTED,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct PlayerAssignmentResult {
    player_number: usize,
    player_secret: Option<String>,
    status: PlayerAssignmentStatus
}

#[server(AssignPlayerToGame, "/api")]
pub async fn assign_player_to_game(
    name: String,
    player_number: usize,
) -> Result<PlayerAssignmentResult, ServerFnError> {
    use tokio::time::{sleep, Duration};

    let player_secret = String::from("my player secret");
    logging::log!("Assign player '{}' to player number '{}'", name, player_number);
    sleep(Duration::from_secs(1)).await;
    Ok(PlayerAssignmentResult {
        player_number,
        player_secret: Some(player_secret),
        status: PlayerAssignmentStatus::ACCEPTED,
    })
}

#[component]
pub fn PlayerAssignment() -> impl IntoView {
    let location = use_location();
    let (
        player_name_cookie, set_player_name_cookie
    ) = use_cookie_with_options::<String, FromToStringCodec>(
        "player_name",
        UseCookieOptions::default()
            .max_age::<i64>(Some(1000*60*60*24*365)) // 1 year
            .same_site(SameSite::Lax),
    );
    let (
        player_secret_cookie, set_player_secret_cookie
    ) = use_cookie_with_options::<String, FromToStringCodec>(
        "player_secret",
        UseCookieOptions::default()
            .max_age::<i64>(Some(1000*60*60*24*14)) // 14 days
            .same_site(SameSite::Lax)
            .path(location.pathname.get_untracked()),
    );
    let (player_name, set_player_name) = create_signal("Player".to_string());
    let is_player_assigned = move || { player_secret_cookie.get().is_some() };
    let (player_assignment_pending, set_player_assignment_pending) = create_signal(false);
    let player_assignment_possible = move || {!is_player_assigned() && !player_assignment_pending.get()};

    if player_name_cookie.get_untracked().is_none() {
        set_player_name_cookie.set(Some(player_name.get()));
    }
    else {
        set_player_name.set(player_name_cookie.get_untracked().expect("Value should exist here."));
    }

    view! {
        <Show when=move || !is_player_assigned()>
            <div class="p-2 w-full flex justify-center">
                <p>"Join game as "</p>
                <input
                    disabled={move || !player_assignment_possible()}
                    type="text"
                    class="input input-bordered max-w-xs input-xs ml-2"
                    on:input=move |ev| {
                        set_player_name.set(event_target_value(&ev))
                    }
                    prop:value=player_name
                />
                <button
                    disabled={move || !player_assignment_possible()}
                    class="btn bg-red-800 hover:bg-red-700 text-content btn-xs ml-2"
                    on:click={move |_| {
                        set_player_assignment_pending.set(true);
                        set_player_name_cookie.set(Some(player_name.get()));
                        spawn_local(async move {
                            match assign_player_to_game(player_name.get_untracked(), 0).await {
                                Ok(result) if result.status == PlayerAssignmentStatus::ACCEPTED => {
                                    logging::log!("Player assignment successful: {:?}", result);
                                    set_player_secret_cookie.set(result.player_secret);
                                }
                                Ok(result) => {
                                    logging::log!("Was not able to assign player: {:?}", result.status);
                                }
                                Err(error) => {
                                    logging::error!("Player assignment failed: {:?}", error);
                                }
                            }
                            set_player_assignment_pending.set(false);
                        });
                    }}
                >
                    "Red Player"
                </button>
                <button
                    disabled={move || !player_assignment_possible()}
                    on:click={move |_| {
                        set_player_assignment_pending.set(true);
                        set_player_name_cookie.set(Some(player_name.get()));
                    }}
                    class="btn bg-blue-800 hover:bg-blue-700 text-content btn-xs ml-2"
                >
                    "Blue Player"
                </button>
                <Show when=player_assignment_pending>
                    <span class="loading loading-spinner text-primary"></span>
                </Show>
            </div>
        </Show>
    }
}

#[component]
pub fn GameInfo(
    #[prop(into)]
    game_id: Signal<Option<Uuid>>
) -> impl IntoView {
    let game_id = {move || {game_id().map(|id| format!("{}", id)).unwrap_or_default()}};
    let game_url = use_url();

    view! {
        <div class="flex justify-start p-2">
            <p class="m-1">"This is game "</p>
            <code class="bg-base-200 m-1 px-1">{move || {game_id()}}</code>
            <CopyToClipboardButton
                text_to_copy=Signal::derive(game_url)
                text="Share"
                class="btn btn-primary btn-xs m-1"
            />
        </div>
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
    text_to_copy: Signal<String>,
    #[prop(into)]
    text: MaybeSignal<String>,
    #[prop(default = "btn btn-primary")]
    class: &'static str,
) -> impl IntoView {
    let clipboard = use_clipboard();

    view! {
        <button
            class={class}
            disabled=move || {!clipboard.is_supported.get()}
            on:click={
                let copy = clipboard.copy.clone();
                move |_| {
                    copy(text_to_copy.get().as_str());
                }
            }
        >
            <Show when=move || clipboard.copied.get() fallback=move || text.get()>
                "Copied!"
            </Show>
        </button>
    }
}