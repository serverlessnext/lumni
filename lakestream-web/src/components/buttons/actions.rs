use std::future::Future;
use std::sync::Arc;

use leptos::ev::MouseEvent;
use leptos::*;

use super::{ButtonType, ClickButton};
use crate::components::forms::FormError;

pub struct ActionTrigger<Action>
where
    Action: Future<Output = Result<(), FormError>> + 'static,
{
    button_type: ButtonType,
    action: Arc<dyn Fn() -> Action>,
}

impl<Action> ActionTrigger<Action>
where
    Action: Future<Output = Result<(), FormError>> + 'static,
{
    pub fn new(
        button_type: ButtonType,
        action: Arc<dyn Fn() -> Action>,
    ) -> Self {
        Self {
            button_type,
            action,
        }
    }

    pub fn render_view(&self, cx: Scope) -> View {
        let is_enabled = create_rw_signal(cx, true);

        let action = Arc::clone(&self.action); // clone action outside the closure

        let on_click = move |event: MouseEvent| {
            let action = Arc::clone(&action); // clone action inside the closure
            if !is_enabled.get() {
                return;
            }

            event.prevent_default();
            spawn_local(async move {
                is_enabled.set(false);
                if let Err(e) = (action)().await {
                    log!("Error executing action: {:?}", e);
                }
                is_enabled.set(true);
            });
        };

        view! { cx,
            <ClickButton button_type={self.button_type.clone()} enabled={is_enabled.into()} on_click={on_click} />
        }
        .into_view(cx)
    }
}
