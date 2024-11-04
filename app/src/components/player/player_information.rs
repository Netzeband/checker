use leptos::*;
use uuid::Uuid;

use super::use_player_assingment::use_player_assingment;
use super::player_assignment::PlayerAssignment;


#[component]
pub fn PlayerInformation(
    #[prop(into)]
    game_id: Signal<Uuid>,
    #[prop(into)]
    player_number: RwSignal<Option<usize>>,
    #[prop(into)]
    error_message: RwSignal<Option<String>>,
) -> impl IntoView {
    let player_secret = create_rw_signal::<Option<String>>(None);
    let player_assignment = use_player_assingment(
        game_id.clone(),
        player_number.clone(),
        player_secret.clone(),
        error_message.clone(),
    );

    view! {
        <PlayerAssignment
            game_id=game_id
            player_number=player_number
            error_message=error_message
            player_assignment=player_assignment
        />
    }
}
