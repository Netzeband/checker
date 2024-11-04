use leptos::*;
use leptos_use::{UseCookieOptions, SameSite};
use codee::string::FromToStringCodec;

use super::use_player_assingment::UsePlayerAssingmentResult;
use crate::utils::use_cookie_signal::{use_cookie_signal, UseCookieSignalResult};

const PLAYER_NAME_COOKIE_NAME: &str = "player_name";


#[component]
pub fn PlayerAssignment(
    #[prop(into)]
    player_number: RwSignal<Option<usize>>,
    player_assignment: UsePlayerAssingmentResult<
        impl Fn(usize, String) + Clone + 'static,
        impl Fn() + Clone + 'static,
        impl Fn() + Clone + 'static,
    >,
) -> impl IntoView {
    let UseCookieSignalResult {
        signal_reader: player_name,
        signal_writer: set_player_name,
        store_value: store_player_name,
    } = use_cookie_signal::<String, FromToStringCodec>(
        "Player".to_string(), 
        PLAYER_NAME_COOKIE_NAME, 
        UseCookieOptions::default()
            .max_age::<i64>(Some(1000*60*60*24*365)) // 1 year
            .same_site(SameSite::Lax)
        );
    let player_assignment_possible = move || { 
        !player_assignment.player_assignment_pending.get() && player_number.get().is_none()
    };

    view! {
        <Show when=move || player_number.get().is_none()>
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
                class="btn bg-red-800 hover:bg-red-700 text-content btn-xs ml-2"
                disabled={move || !player_assignment_possible()}
                    on:click={
                        let store_player_name = store_player_name.clone();
                        let assign_player = player_assignment.assign_player.clone();
                        move |_| { 
                            store_player_name();
                            assign_player(0, player_name.get());
                        }
                    }
                >
                    "Red Player"
                </button>
                <button
                    disabled={move || !player_assignment_possible()}
                    on:click={
                        let store_player_name = store_player_name.clone();
                        let assign_player = player_assignment.assign_player.clone();
                        move |_| { 
                            store_player_name();
                            assign_player(1, player_name.get());
                        }
                    }
                    class="btn bg-blue-800 hover:bg-blue-700 text-content btn-xs ml-2"
                >
                    "Blue Player"
                </button>
                <Show when=player_assignment.player_assignment_pending>
                    <span class="loading loading-spinner text-primary"></span>
                </Show>
            </div>
        </Show>
    }
}
