use std::{collections::HashMap, hash::Hash, sync::Arc, time::Duration};
use leptos::*;
use leptos_router::{Params, use_params, use_location};
use uuid::Uuid;
use leptos_use::{
    use_clipboard, use_cookie_with_options, use_interval_fn, use_websocket_with_options, 
    utils::Pausable, ReconnectLimit, SameSite, UseCookieOptions, UseWebSocketOptions, UseWebSocketReturn
};
use codee::string::{FromToStringCodec, JsonSerdeCodec};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, TimeDelta};
use leptos_use::core::ConnectionReadyState;
use serde_json::to_string;

use crate::components::player::{self, PlayerInformation};



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
    let player_number = create_rw_signal::<Option<usize>>(None);
    let error_message = create_rw_signal::<Option<String>>(None);

    view! {
        <Show when=move || {error_message.get().is_some()}>
            <p class="content-error">"Error: "{error_message.get().unwrap()}</p>
        </Show>
        <Show when=move || {player_number.get().is_some()}>
            <p class="content-success">{format!("Player: {}", player_number.get().unwrap())}</p>
        </Show>
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
            <PlayerInformation 
                game_id=Signal::derive(move || id().unwrap())
                player_number=player_number
                error_message=error_message
            />
            /*<PlayerAssignment game_id=Signal::derive(move || id().unwrap())/>*/
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

#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct Player {
    pub name: Option<String>,
    pub last_ping: Option<DateTime<Utc>>,
    pub player_number: usize,
    pub is_assigned: bool,
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

#[cfg(feature = "ssr")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub public_data: Player,
    pub secret: Option<String>,
}

#[cfg(feature = "ssr")]
impl PlayerInfo {
    pub fn new(player_number: usize) -> Self {
        Self {
            public_data: Player {
                name: None,
                last_ping: None,
                player_number,
                is_assigned: false,
            },
            secret: None,
        }
    }
}

#[cfg(feature = "ssr")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerList {
    pub players: Vec<PlayerInfo>
}

#[cfg(feature = "ssr")]
impl PlayerList {
    pub fn new(number_of_players: usize) -> Self {
        let players = (0..number_of_players).map(|i| PlayerInfo::new(i)).collect();

        Self {
            players
        }
    }
}

#[cfg(feature = "ssr")]
#[derive(Clone, Debug)]
pub struct Game {
    data: Arc<dashmap::DashMap<&'static str, String>>,
}

#[cfg(feature = "ssr")]
impl Game {
    pub async fn new(id: Uuid) -> Self {
        let data = Arc::new(dashmap::DashMap::new());
        data.insert("id", serde_json::to_string(&id).unwrap());
        let players_string = serde_json::to_string(&PlayerList::new(2)).unwrap();        
        data.insert("players", players_string);

        Self {
            data
        }
    }

    pub async fn id(&self) -> Uuid {
        self.data.get("id").map(|v| {
            serde_json::from_str(&v).expect(format!("Cannot deserialize game id from string: '{:?}'", v).as_str())
        }).expect("Cannot find key 'id' in game data.")
    }

    pub async fn players(&self) -> Vec<PlayerInfo> {
        self.data.get("players").map(|v| {
            serde_json::from_str::<PlayerList>(v.value())
                .expect(format!("Cannot deserialize player list from string: '{:?}'", v.value()).as_str())
        }).expect("Cannot find key 'players' in game data.").players
    }

    pub async fn with_player<F>(&self, update_func: F)
    where 
        F: FnOnce(PlayerList) -> PlayerList
    {
        self.data.entry("players").and_modify(|v| {
            let players: PlayerList = serde_json::from_str(&v)
                .expect(format!("Cannot deserialize player list from string: '{:?}'", v).as_str());
            let updated_players = update_func(players);
            *v = serde_json::to_string(&updated_players).expect("Cannot serialize player list.");
        });
    }
}

#[cfg(feature = "ssr")]
pub struct GameState {
    game: tokio::sync::RwLock<HashMap<Uuid, Game>>,
}

#[cfg(feature = "ssr")]
impl GameState {
    pub fn new() -> Self {
        Self {
            game: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    pub async fn get_or_create_game(&self, game_id: Uuid) -> Game {        
        let mut games = self.game.write().await;
        let new_game = Game::new(game_id).await;
        let game = games.entry(game_id).or_insert_with( || {
            logging::log!("Creating new game: {:?}", game_id);
            new_game
        }).clone();
        game
    }    
}

#[cfg(feature = "ssr")]
pub async fn handle_players_websocket(
    mut socket: axum::extract::ws::WebSocket,
    axum::extract::Extension(game_state): axum::extract::Extension<Arc<GameState>>
) {
    use tokio::time::timeout;
    use futures::StreamExt;
    use axum::extract::ws::Message;

    let mut game: Option<Game> = None;

    let mut last_ping = Utc::now();
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
                        else if game.as_ref().unwrap().id().await != game_id {
                            logging::error!("Already selected game {}", game_id);
                        }
                    }
                    Ok(PlayerClientData::Alive(player_identity)) => { 
                        if let Some(game) = &game {
                            if player_identity.game_id == game.id().await {
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

        let current_time = Utc::now();
        if (current_time - last_ping) > TimeDelta::seconds(1) {
            last_ping = current_time;
            if let Some(ref game) = game {
                let players = game.players().await.iter().map(|p| p.public_data.clone()).collect();
                if socket.send_player_server_data(&PlayerServerData::PlayerList(players)).await.is_err() {
                    break;
                }            
            }    
        }
    }

    logging::log!("Players websocket closed by client.");
}


#[server(AssignPlayerToGame, "/api")]
pub async fn assign_player_to_game(
    game_id: Uuid,
    name: String,
    player_number: usize,    
) -> Result<PlayerAssignmentResult, ServerFnError> {
    use tokio::time::{sleep, Duration};
    use leptos_axum::extract;
    use axum::extract::Extension;

    let game_state= extract::<Extension<Arc<GameState>>>().await
        .expect("Cannot get the game-state extension.");
    let game = game_state.get_or_create_game(game_id).await;

    let mut player_secret: Option<String> = None;
    let mut status = PlayerAssignmentStatus::REFUSED;    
    game.with_player( |mut players| {
        if player_number < players.players.len() {            
            if !players.players[player_number].public_data.is_assigned {
                logging::log!("Assign player '{}' to player number '{}' in game '{}'", name, player_number, game_id);
                status = PlayerAssignmentStatus::ACCEPTED;
                player_secret = Some(Uuid::new_v4().to_string()); // todo: generate a better secret

                players.players[player_number].public_data.name = Some(name.clone());
                players.players[player_number].public_data.is_assigned = true;
                players.players[player_number].public_data.last_ping = Some(Utc::now());
                players.players[player_number].secret = player_secret.clone();
            }
            else {
                logging::error!("Player number {} is already assigned.", player_number);
            }
        }
        players
    }).await;
    sleep(Duration::from_secs(1)).await;
    Ok(PlayerAssignmentResult {
        player_number,
        player_secret: player_secret,
        status: status
    })
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
    let (now, set_now) = create_signal(Utc::now());
    let Pausable { .. } = use_interval_fn(
        move || {
            set_now.set(Utc::now());
        },
        1000,
    );


    view! {
        <p>{move || {format!("now: {:?}", now.get())}}</p>
        <div class="overflow-x-auto">        
            <table class="table">
                <tbody>
                    <For
                        each=move || players.get()
                        key=|player| player.clone()
                        let:player
                    >
                        <Show when=move || player.is_assigned>
                            <PlayerInfo player=player.clone() now=Signal::derive(now) />
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
                            match assign_player_to_game(game_id.get_untracked(), player_name.get_untracked(), 0).await {
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
    player: Player,
    #[prop(into)]
    now: Signal<DateTime<Utc>>,
) -> impl IntoView {
    view! {
        <tr>
            <th>
                {move || match player.last_ping {                    
                    Some(last_ping) if (now.get() - last_ping) < TimeDelta::seconds(10) => {
                        view! {<div class="badge badge-success badge-xs"></div>}
                    },
                    Some(last_ping) if (now.get()- last_ping) < TimeDelta::seconds(120) => {
                        view! {<div class="badge badge-warning  badge-xs"></div>}
                    },
                    _ => {view! {<div class="badge badge-error badge-xs"></div>} }
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