use leptos::*;
use leptos_meta::*;

mod pages;
mod error_template;

use pages::app_router::AppRouter;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/checker.css"/>
        <Title text="Play Checker"/>
        <AppRouter/>
    }
}
