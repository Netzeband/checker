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


#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Count {
    pub value: i32,
}

#[cfg(feature = "ssr")]
pub async fn player_handle_socket1(socket: std::sync::Arc<tokio::sync::Mutex<axum::extract::ws::WebSocket>>)
{
    use std::time::Duration;
    use std::ops::DerefMut;
    use leptos_server_signal::ServerSignal;

    let mut count = ServerSignal::<Count>::new("counter").unwrap();

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let mut locked_socket = socket.lock().await;
        let result = count.with(&mut locked_socket.deref_mut(), |count| count.value += 1).await;
        if result.is_err() {
            break;
        }
    }
}

#[cfg(feature = "ssr")]
pub async fn player_handle_socket2(socket: std::sync::Arc<tokio::sync::Mutex<axum::extract::ws::WebSocket>>) {
    use std::time::Duration;
    use std::ops::DerefMut;
    use leptos_server_signal::ServerSignal;

    let mut count = ServerSignal::<Count>::new("counter2").unwrap();

    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        let mut locked_socket = socket.lock().await;
        let result = count.with(locked_socket.deref_mut(), |count| count.value -= 1).await;
        if result.is_err() {
            break;
        }
    }
}

use chrono::{DateTime, Utc};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Player {
    pub name: Option<String>,
    pub last_ping: Option<DateTime<Utc>>,
    pub player_number: usize,
    pub is_assigned: bool,
}

#[cfg(feature = "ssr")]
pub async fn player_list_handler(socket: std::sync::Arc<tokio::sync::Mutex<axum::extract::ws::WebSocket>>) {
    use std::time::Duration;
    use std::ops::DerefMut;
    use leptos_server_signal::ServerSignal;

    let mut players = ServerSignal::<Vec<Player>>::new("players").unwrap();

    {
        let mut locked_socket = socket.lock().await;
        let result = players.with(locked_socket.deref_mut(), |players| {
            players.push(Player {
                name: None,
                last_ping: None,
                player_number: 0,
                is_assigned: false,
            });
            players.push(Player {
                name: None,
                last_ping: None,
                player_number: 1,
                is_assigned: false,
            });
        }).await;
        if result.is_err() {
            logging::error!("Error when sending initial player list.");
            return;
        }
    }

    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        let mut locked_socket = socket.lock().await;
        let result = players.with(locked_socket.deref_mut(), |_p| {}).await;
        if result.is_err() {
            break;
        }
    }
}

#[component]
pub fn PlayerAssignment() -> impl IntoView {
    use leptos_server_signal::create_server_signal;

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
    let count = create_server_signal::<Count>("counter");
    let count2 = create_server_signal::<Count>("counter2");

    let players = create_server_signal::<Vec<Player>>("players");

    view! {
        <p>{move || players.get().len()}</p>
        <For
            each=move || players.get()
            key=|player| player.player_number
            children=move |player: Player| {
                view! {
                    <p>{player.name}</p>
                }
            }
        />
        <p>"Count 1: " {move || count.get().value.to_string()}</p>
        <p>"Count 2: " {move || count2.get().value.to_string()}</p>
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