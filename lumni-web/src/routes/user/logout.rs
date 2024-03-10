use leptos::*;
use web_sys::window;

#[component]
pub fn Logout() -> impl IntoView {
    const ERROR_MESSAGE: &str = "Failed to log out. Try to refresh the page.";

    let redirect_url = "/";
    let logout_success = create_rw_signal(None::<String>);

    if let Some(window) = window() {
        if window.location().replace(redirect_url).is_err() {
            logout_success.set(Some(ERROR_MESSAGE.to_string()));
        }
    }

    {
        if let Some(error) = logout_success.get() {
            view! {
                <div>{error}</div>
            }
        } else {
            view! {
                <div>"Logging out..."</div>
            }
        }
    }
}
