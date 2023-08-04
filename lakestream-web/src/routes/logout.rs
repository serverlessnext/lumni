use leptos::*;
use web_sys::window;

#[component]
pub fn Logout(cx: Scope) -> impl IntoView {
    const ERROR_MESSAGE: &str = "Failed to log out. Try to refresh the page.";

    let redirect_url = "/";
    let logout_success = create_rw_signal(cx, None::<String>);

    if let Some(window) = window() {
        if window.location().replace(redirect_url).is_err() {
            logout_success.set(Some(ERROR_MESSAGE.to_string()));
        }
    }

    {
        if let Some(error) = logout_success.get() {
            view! {
                cx,
                <div>{error}</div>
            }
        } else {
            view! {
                cx,
                <div>"Logging out..."</div>
            }
        }
    }
}
