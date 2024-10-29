use leptos::*;
use uuid::Uuid;

#[component]
pub fn NewGamePage() -> impl IntoView {
    let new_game_id = Uuid::now_v7();
    let (existing_game_id, set_existing_game_id) = create_signal("".to_string());

    view! {
        <div class="p-2">
            <a class="btn btn-primary" href={format!("/games/{}", new_game_id)}>
                "New Game"
            </a>
        </div>
        <div class="p-2 w-full flex justify-center">
            <input
                type="text"
                class="input input-bordered w-full max-w-xl"
                on:input=move |ev| {
                    set_existing_game_id.set(event_target_value(&ev))
                }
                prop:value=existing_game_id
            />
            <a class="btn btn-primary mx-5" href={move || {format!("/games/{}", existing_game_id.get())}}>
                "Join Game"
            </a>
        </div>
    }
}
