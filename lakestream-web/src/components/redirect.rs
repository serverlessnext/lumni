use leptos::*;
use web_sys::window;

#[component]
pub fn Redirect(cx: Scope) -> impl IntoView {
    const ERROR_MESSAGE: &str =
        "Failed to redirect. Please try to refresh the page.";

    let redirect_url = "/home";
    let error_message = create_rw_signal(cx, None::<String>);

    if let Some(window) = window() {
        if window.location().replace(redirect_url).is_err() {
            error_message.set(Some(ERROR_MESSAGE.to_string()));
        }
    }

    {
        if let Some(error) = error_message.get() {
            view! {
                cx,
                <div>{error}</div>
            }
        } else {
            view! {
                cx,
                <div>"Redirecting..."</div>
            }
        }
    }
}
