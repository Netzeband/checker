use leptos::*;
use uuid::Uuid;

#[component]
pub fn NewGamePage() -> impl IntoView {
    let game_uuid = Uuid::now_v7();

    view! {
        <a class="btn btn-primary" href={format!("/games/{}", game_uuid)}>
            "New Game"
        </a>
    }
}
