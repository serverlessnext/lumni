use leptos::*;
use web_sys::window;

const DEFAULT_REDIRECT_URL: &str = "/console";

#[component]
pub fn Redirect(redirect_url: Option<String>) -> impl IntoView {
    const ERROR_MESSAGE: &str =
        "Failed to redirect. Please try to refresh the page.";

    let redirect_url = redirect_url.unwrap_or(DEFAULT_REDIRECT_URL.to_string());

    let error_message = create_rw_signal(None::<String>);

    if let Some(window) = window() {
        if window.location().replace(&redirect_url).is_err() {
            error_message.set(Some(ERROR_MESSAGE.to_string()));
        }
    }

    move || {
        if let Some(error) = error_message.get() {
            view! {
                <div>{error}</div>
            }
        } else {
            view! {
                <div>"Redirecting..."</div>
            }
        }
    }
}
