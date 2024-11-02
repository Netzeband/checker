use std::time::Duration;
use leptos::*;
use leptos_router::{Params, use_params, use_location};
use uuid::Uuid;
use leptos_use::{use_clipboard, use_cookie_with_options, use_websocket, use_websocket_with_options, ReconnectLimit, SameSite, UseCookieOptions, UseWebSocketOptions, UseWebSocketReturn};
use codee::string::{FromToStringCodec, JsonSerdeCodec};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, TimeDelta};
use leptos_use::core::ConnectionReadyState;

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
            <PlayerAssignment game_id=Signal::derive(move || id().unwrap())/>
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

use serde_json::to_string;

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
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

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct PlayerIdentity {
    pub game_id: Uuid,
    pub player_number: usize,
    pub secret: String,
}

#[derive(Clone, Serialize, Deserialize)]
enum PlayerClientData {
    SelectGame(Uuid),
    Alive(PlayerIdentity),
}

#[derive(Clone, Serialize, Deserialize)]
enum PlayerServerData {
    PlayerList(Vec<Player>),
}


#[cfg(feature = "ssr")]
trait SendPlayerServerData {
    async fn send_player_server_data(&mut self, data: &PlayerServerData) -> Result<(), String>;
}

#[cfg(feature = "ssr")]
impl SendPlayerServerData for axum::extract::ws::WebSocket {
    async fn send_player_server_data(&mut self, data: &PlayerServerData) -> Result<(), String> {
        use axum::extract::ws::Message;

        let data = to_string(data).map_err(|_| "Cannot serialize player data.".to_string())?;
        self.send(Message::Text(data)).await.map_err(|_| "Connection closed by client.".to_string())
    }
}

#[cfg(feature = "ssr")]
trait ReceivePlayerClientData {
    fn receive_player_server_data(&self) -> Result<PlayerClientData, String>;
}

#[cfg(feature = "ssr")]
impl ReceivePlayerClientData for axum::extract::ws::Message {
    fn receive_player_server_data(&self) -> Result<PlayerClientData, String> {
        use axum::extract::ws::Message;

        match self {
            Message::Text(data) => {
                serde_json::from_str(data).map_err(|_| "Cannot deserialize player data.".to_string())
            }
            _ => Err("Unsupported message type.".to_string())
        }
    }
}

pub struct Game {
    id: Uuid,
}

impl Game {
    pub fn new(id: Uuid) -> Self {
        Self {
            id
        }
    }

    pub fn id(&self) -> Uuid {
        self.id.clone()
    }
}

pub struct GameState {
}

impl GameState {
    pub fn new() -> Self {
        Self {
        }
    }

    pub async fn get_or_create_game(&self, game_id: Uuid) -> Game {
        Game::new(game_id)
    }    
}

use std::sync::Arc;


#[cfg(feature = "ssr")]
pub async fn handle_players_websocket(
    mut socket: axum::extract::ws::WebSocket,
    axum::extract::Extension(game_state): axum::extract::Extension<Arc<GameState>>
) {
    use tokio::time::timeout;
    use futures::StreamExt;
    use axum::extract::ws::Message;

    let mut game: Option<Game> = None;
    let players = Vec::from([
        Player {
            name: None,
            last_ping: None,
            player_number: 0,
            is_assigned: false, 
        },
        Player {
            name: None,
            last_ping: None,
            player_number: 1,
            is_assigned: false,
        },
    ]);

    loop {
        match timeout(Duration::from_millis(1000), socket.next()).await {
            Ok(Some(Ok(message))) => {
                if let Message::Close(_) = message {
                    break;
                }
                match message.receive_player_server_data() {
                    Ok(PlayerClientData::SelectGame(game_id)) => { 
                        if game.is_none() {
                            logging::log!("Selecting game {}", game_id);
                            game = Some(game_state.get_or_create_game(game_id).await);
                        }
                        else if game.as_ref().unwrap().id() != game_id {
                            logging::error!("Already selected game {}", game_id);
                        }
                    }
                    Ok(PlayerClientData::Alive(player_identity)) => { 
                        if let Some(game) = &game {
                            if player_identity.game_id == game.id() {
                                // check if player secret is correct
                                // update last_ping time of player
                            }
                            else {
                                logging::error!("Received message for wrong game: {:?}", player_identity.game_id);
                            }
                        }
                        else {
                            logging::error!("Received message before selecting a game: {:?}", message);
                        }
                    }
                    Err(error) => { 
                        logging::error!("Cannot receive player data: {:?} for {:?}.", error, message);
                        break; 
                    }
                }
            }
            Ok(Some(Err(error))) => { 
                logging::error!("Websocket error: {:?}.", error);
                break; 
            }
            Ok(None) => { 
                break; 
            }
            Err(_) => { /* timeout */ }
        }

        if socket.send_player_server_data(&PlayerServerData::PlayerList(players.clone())).await.is_err() {
            break;
        }        
    }

    logging::log!("Players websocket closed by client.");
}


#[component]
pub fn PlayerAssignment(
    game_id: Signal<Uuid>,
) -> impl IntoView {
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
    let (player_number, set_player_number) = create_signal::<Option<usize>>(None);
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
    let (players, set_players) = create_signal::<Vec<Player>>(Vec::new());
    let UseWebSocketReturn {
        ready_state: players_socket_ready_state,
        message: players_socket_message,
        send: players_socket_send,
        ..
    } = use_websocket_with_options::<PlayerClientData, PlayerServerData, JsonSerdeCodec>(
        "/players",
        UseWebSocketOptions::default()
            .reconnect_limit(ReconnectLimit::Infinite)
            .on_close(|_| logging::log!("Lost connection to players websocket."))
            .reconnect_interval(5000),
    );
    let players_socket_send1 = players_socket_send.clone();
    create_effect(move |_| {
        players_socket_message.with(|message| {
            match message {
                Some(PlayerServerData::PlayerList(players)) => {
                    set_players.set(players.clone());
                    if let Some(player_number) = player_number.get_untracked() {
                        players_socket_send1(&PlayerClientData::Alive(PlayerIdentity {
                            game_id: game_id.get_untracked(),
                            player_number: player_number,
                            secret: player_secret_cookie.get_untracked().unwrap_or_default(),
                        }));    
                    }
                }
                _ => {}
            }
        });
    });
    let players_socket_send2 = players_socket_send.clone();
    create_effect(move |_| {
        players_socket_ready_state.with(|state| {
            if state == &ConnectionReadyState::Open {
                players_socket_send2(&PlayerClientData::SelectGame(game_id.get()));
            }
        });
    });


    view! {
        <div class="overflow-x-auto">
            <table class="table">
                <tbody>
                    <For
                        each=move || players.get()
                        key=|player| player.player_number
                        let:player
                    >
                        <Show when=move || player.is_assigned>
                            <PlayerInfo player=player.clone()/>
                        </Show>
                    </For>
                </tbody>
            </table>
        </div>
        <div>
        </div>
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
pub fn PlayerInfo(
    #[prop(into)]
    player: Player
) -> impl IntoView {
    view! {
        <tr>
            <th>
                {move || match player.last_ping {
                    Some(last_ping) if (last_ping - Utc::now()) < TimeDelta::seconds(10) => {
                        view! {<div class="badge badge-success badge-sm"></div>}
                    },
                    Some(last_ping) if (last_ping - Utc::now()) < TimeDelta::seconds(120) => {
                        view! {<div class="badge badge-warning  badge-sm"></div>}
                    },
                    _ => {view! {<div class="badge badge-error badge-sm"></div>} }
                }}
            </th>
            <th>{player.name.unwrap_or_else(|| "Unknown".to_string())}</th>
            <th>
                {move || match player.player_number {
                    0 => view! {<div class="badge bg-red-700">Player Red</div>},
                    1 => view! {<div class="badge bg-blue-700">Player Blue</div>},
                    i => {
                        logging::error!("Unknown player number: {}", i);
                        view! { <div>Unknown player assignment</div> }
                    }
                }}
            </th>
        </tr>
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