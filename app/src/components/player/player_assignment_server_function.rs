
use leptos::*;
use serde::{Deserialize, Serialize};
use core::fmt;
use uuid::Uuid;
use std::str::FromStr;
use std::time::Duration;


#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
pub enum PlayerAssingmentError {
    UnknownError(String),
    InvalidPlayerNumber,
    PlayerAllreadyAssigned,
    InvalidPlayerSecret,
}

impl fmt::Display for PlayerAssingmentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PlayerAssingmentError::UnknownError(s) => write!(f, "{}", s),
            PlayerAssingmentError::InvalidPlayerNumber => write!(f, "Invalid player number."),
            PlayerAssingmentError::PlayerAllreadyAssigned => write!(f, "Player allready assigned."),
            PlayerAssingmentError::InvalidPlayerSecret => write!(f, "Invalid player secret."),
        }
    }
}

impl FromStr for PlayerAssingmentError {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(PlayerAssingmentError::UnknownError(s.to_string()))
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
pub struct PlayerAssignmentResult {
    pub player_number: usize,
    pub player_secret: String,
}

#[server(AssignPlayerToGame, "/api")]
pub async fn assign_player_to_game(
    game_id: Uuid,
    name: String,
    player_number: usize,    
) -> Result<PlayerAssignmentResult, ServerFnError<PlayerAssingmentError>> {
    logging::log!("Assigning player to game: {} {} {}", game_id, name, player_number);
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(PlayerAssignmentResult {
        player_number,
        player_secret: "foobar".to_string(),
    })
}

#[server(ReassignPlayerToGame, "/api")]
pub async fn reassign_player_to_game(
    game_id: Uuid,
    player_number: usize,    
    player_secret: String,
) -> Result<PlayerAssignmentResult, ServerFnError<PlayerAssingmentError>> {
    logging::log!("Reassigning player to game: {} {} {}", game_id, player_number, player_secret);
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(PlayerAssignmentResult {
        player_number,
        player_secret: "foobar".to_string(),
    })
}

#[server(UnassignPlayerFromGame, "/api")]
pub async fn unassign_player_from_game(
    game_id: Uuid,
    player_number: usize,    
    player_secret: String,
) -> Result<(), ServerFnError<PlayerAssingmentError>> {
    logging::log!("Unassigning player from game: {} {} {}", game_id, player_number, player_secret);
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(())
}
