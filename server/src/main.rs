use app::*;
use axum::{Router, routing::get};
use fileserv::file_and_error_handler;
use leptos::*;
use leptos_axum::{generate_route_list, LeptosRoutes};

use app::pages::game_page::{player_handle_socket1, player_handle_socket2};

pub mod fileserv;

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // build our application with a route
    let app = Router::new()
        .leptos_routes(&leptos_options, routes, App)
        .route("/ssws", get(server_signal_websocket))
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn server_signal_websocket(ws: axum::extract::WebSocketUpgrade) -> axum::response::Response {
    ws.on_upgrade(handle_server_signal_socket)
}

async fn handle_server_signal_socket(socket: axum::extract::ws::WebSocket) {
    use tokio::sync::Mutex;
    use tokio::task::JoinSet;
    use std::sync::Arc;
    let socket = Arc::new(Mutex::new(socket));

    let mut handler_set = JoinSet::new();

    handler_set.spawn(player_handle_socket1(socket.clone()));
    handler_set.spawn(player_handle_socket2(socket.clone()));

    let _ = handler_set.join_all().await;
}
