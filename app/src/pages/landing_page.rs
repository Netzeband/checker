use leptos::*;
use leptos_router::*;


#[component]
pub fn LandingPage() -> impl IntoView {
    view! {
        <html data-theme="dark"></html>
        <div class="w-full">
            <div class="navbar bg-base-200 w-full px-10">
                <p class="text-3xl">Checker</p>
            </div>
            <div class="px-10 py-5 w-full flex flex-col items-center">
                <Outlet/>
            </div>
        </div>
    }
}
