use leptos::*;
use leptos_router::use_location;
use leptos_use::{use_cookie_with_options, UseCookieOptions, SameSite, UseTimeoutFnReturn, use_timeout_fn};
use serde::{Deserialize, Serialize};
use codee::string::JsonSerdeCodec;
use core::fmt;
use uuid::Uuid;

use super::player_assignment_server_functiony::{assign_player_to_game, reassign_player_to_game, unassign_player_from_game};


const PLAYER_ASSIGNMENT_COOKIE_NAME: &str = "player_assignment";
const PLAYER_ASSIGNMENT_COOKIE_LIFETIME_IN_SEC: i64 = 60*60*24*14; // 14 days
const PLAYER_ASSINGMENT_TIMEOUT_IN_SEC: f64 = 10.0;


#[derive(Debug, Clone)]
pub struct AssingmentError {
    message: String,
}

impl AssingmentError {
    pub fn new(message: &str) -> Self {
        Self { message: message.to_string() }
    }
}

impl fmt::Display for AssingmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cannot assign/reassign or unassign player: {}", self.message)
    }
}


pub struct UsePlayerAssingmentResult<AssignFn, ReassignFn, UnassignFn> 
where 
    AssignFn: Fn(usize, String) + Clone + 'static,
    ReassignFn: Fn() + Clone + 'static,
    UnassignFn: Fn() + Clone + 'static,
{
    pub player_assignment_pending: Signal<bool>,
    pub assign_player: AssignFn,
    pub reassign_player: ReassignFn,
    pub unassign_player: UnassignFn,
}


pub fn use_player_assingment(
    game_id: Signal<Uuid>,
    player_number: RwSignal<Option<usize>>,
    player_secret: RwSignal<Option<String>>,
    assignment_error: RwSignal<Option<String>>,
) -> UsePlayerAssingmentResult::<
    impl Fn(usize, String) + Clone + 'static,
    impl Fn() + Clone + 'static,
    impl Fn() + Clone + 'static,
> 
{
    let (player_assingment_cookie, set_player_assignment_cookie) = use_player_assignment_cookie();
    let UseTimeoutFnReturn { 
        start: start_assignment_timeout, 
        stop: stop_assignment_timeout, 
        is_pending: is_assignment_pending,
        ..
    } = use_timeout_fn(
        move |_| {
            logging::error!("Player re-/un-/assignment timeout.");
            player_secret.set(None);
            player_number.set(None);
            set_player_assignment_cookie.set(None);
        },
        PLAYER_ASSINGMENT_TIMEOUT_IN_SEC * 1000.0,
    );

    let start_assignment_timeout_for_assing_player = start_assignment_timeout.clone();
    let stop_assignment_timeout_for_assing_player = stop_assignment_timeout.clone();
    let assign_player = move |requested_player_number: usize, player_name: String| { 
        start_assignment_timeout_for_assing_player(());
        let stop_assignment_timeout_for_assing_player = stop_assignment_timeout_for_assing_player.clone();
        spawn_local(async move {
            match assign_player_to_game(game_id.get_untracked(), player_name, requested_player_number).await {
                Ok(result) => {
                    logging::log!("Player assignment successful.");
                    player_secret.set(Some(result.player_secret.clone()));
                    player_number.set(Some(result.player_number));
                    set_player_assignment_cookie.set(Some(PlayerAssignmentData {
                        player_number: result.player_number,
                        player_secret: result.player_secret.clone(),
                    }));
                }
                Err(error) => {
                    logging::error!("Player assignment failed: {:?}", error);
                    assignment_error.set(Some("Player assignment failed.".to_string()));
                    player_secret.set(None);
                    player_number.set(None);
                    set_player_assignment_cookie.set(None);
                }
            }
            stop_assignment_timeout_for_assing_player();
        });        
    };

    let start_assignment_timeout_for_reassing_player = start_assignment_timeout.clone();
    let stop_assignment_timeout_for_reassing_player = stop_assignment_timeout.clone();
    let inner_reassign_player = move |player_assingment_data: PlayerAssignmentData| { 
        start_assignment_timeout_for_reassing_player(());
        let stop_assignment_timeout_for_reassing_player = stop_assignment_timeout_for_reassing_player.clone();
        spawn_local(async move {
            match reassign_player_to_game(
                game_id.get_untracked(), 
                player_assingment_data.player_number, 
                player_assingment_data.player_secret
            ).await {
                Ok(result) => {
                    logging::log!("Player reassignment successful.");
                    player_secret.set(Some(result.player_secret.clone()));
                    player_number.set(Some(result.player_number));
                    set_player_assignment_cookie.set(Some(PlayerAssignmentData {
                        player_number: result.player_number,
                        player_secret: result.player_secret.clone(),
                    }));
                }
                Err(error) => {
                    logging::error!("Player reassignment failed: {:?}", error);
                    assignment_error.set(Some("Player reassignment failed.".to_string()));
                    player_secret.set(None);
                    player_number.set(None);
                    set_player_assignment_cookie.set(None);
                }
            }
            stop_assignment_timeout_for_reassing_player();
        });        
    };

    let start_assignment_timeout_for_unassing_player = start_assignment_timeout.clone();
    let stop_assignment_timeout_for_unassing_player = stop_assignment_timeout.clone();
    let inner_unassign_player = move |player_assingment_data: PlayerAssignmentData| { 
        start_assignment_timeout_for_unassing_player(());
        let stop_assignment_timeout_for_unassing_player = stop_assignment_timeout_for_unassing_player.clone();
        spawn_local(async move {
            match unassign_player_from_game(
                game_id.get_untracked(), 
                player_assingment_data.player_number, 
                player_assingment_data.player_secret
            ).await {
                Ok(_) => {
                    logging::log!("Player successfully unassigned.");
                    player_secret.set(None);
                    player_number.set(None);
                    set_player_assignment_cookie.set(None);
                }
                Err(error) => {
                    logging::error!("Player unassignment failed: {:?}", error);
                }
            }
            stop_assignment_timeout_for_unassing_player();
        });
    };

    // Automatically reassign a player in case the cookie is set but the secret not
    //  this only happens when there is a browser reload.
    let inner_reassign_player_for_auto_reassign = inner_reassign_player.clone();
    create_effect(move |_| {
        if let Some(player_assingment_cookie) = player_assingment_cookie.get() {
            if player_secret.get().is_none() {
                let _ = inner_reassign_player_for_auto_reassign(player_assingment_cookie);
            }
        }
    });

    let reassign_player = move || { 
        if let Some(player_assingment_data) = player_assingment_cookie.get() {
            inner_reassign_player(player_assingment_data);
        }
        else {
            logging::log!("Cannot reassign player, because no previous assingment data found (cookie).");
        }                
    };

    let unassign_player = move || { 
        if let Some(player_assingment_data) = player_assingment_cookie.get() {
            inner_unassign_player(player_assingment_data);
        }
        else {
            logging::log!("Cannot unassign player, because no previous assingment data found (cookie).");
        }                
    };

    UsePlayerAssingmentResult {
        player_assignment_pending: Signal::derive(move || { is_assignment_pending.get() }),
        assign_player,
        reassign_player,
        unassign_player,
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlayerAssignmentData {
    pub player_number: usize,
    pub player_secret: String,
}


fn use_player_assignment_cookie() -> (Signal<Option<PlayerAssignmentData>>, WriteSignal<Option<PlayerAssignmentData>>) {
    let location = use_location();

    let (
        player_assignment_cookie, set_player_assingment_cookie
    ) = use_cookie_with_options::<PlayerAssignmentData, JsonSerdeCodec>(
        PLAYER_ASSIGNMENT_COOKIE_NAME,
        UseCookieOptions::default()
            .max_age::<i64>(Some(PLAYER_ASSIGNMENT_COOKIE_LIFETIME_IN_SEC * 1000)) // 14 days
            .same_site(SameSite::Lax)
            .path(location.pathname.get_untracked()),
    );

    (player_assignment_cookie, set_player_assingment_cookie)
}
