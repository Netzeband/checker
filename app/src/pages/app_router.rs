use leptos::*;
use leptos_router::*;

use crate::error_template::{AppError, ErrorTemplate};
use crate::pages::game_page::GamePage;
use crate::pages::landing_page::LandingPage;
use crate::pages::new_game_page::NewGamePage;

#[component]
pub fn AppRouter() -> impl IntoView {
    view! {
        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>
            <main>
                <Routes>
                    <Route path="/" view=LandingPage>
                        <Route path="" view=NewGamePage/>
                        <Route path="/games" view=|| view! { <Outlet/> }>
                            <Route path="" view=NewGamePage/>
                            <Route path=":id" view=GamePage/>
                        </Route>
                    </Route>
                </Routes>
            </main>
        </Router>
    }

}