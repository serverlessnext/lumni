use leptos::ev::SubmitEvent;
use leptos::html::Input;
use leptos::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{AesKeyGenParams, CryptoKey, Pbkdf2Params};

use crate::base::GlobalState;
use crate::utils::convert_types::string_to_uint8array;

#[component]
pub fn LoginForm(cx: Scope) -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided");

    let (_key, set_crypto_key) = create_slice(
        cx,
        state,
        |state| state.crypto_key.clone(),
        |state, crypto_key| state.crypto_key = crypto_key,
    );

    let password_ref: NodeRef<Input> = create_node_ref(cx);

    let on_submit = move |ev: SubmitEvent| {
        // Remove this line to allow default form submission behavior
        ev.prevent_default();

        let password = password_ref().expect("password to exist").value();

        // TODO: ideally replace this with a random (/derived) salt
        // may not be necessary if we're using a random IV for each encryption
        // need to be investigated further
        // https://developer.mozilla.org/en-US/docs/Web/API/SubtleCrypto
        let salt = "your_salt_value_here";

        // Spawn a new task to wait for the future from derive_key to complete
        spawn_local(async move {
            match derive_key_websys(&password, &salt).await {
                Ok(crypto_key) => {
                    log!("Key stored as: {:?}", crypto_key);
                    set_crypto_key(Some(crypto_key));
                }
                Err(err) => {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Error deriving key: {:?}",
                        err
                    )));
                }
            }
        });
    };

    view! { cx,
        <form class="flex flex-col w-96"  on:submit=on_submit>
            <div class="flex flex-col mb-4">
                <label class="mb-2">"Password"</label>
                <input type="password"
                    class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                    node_ref=password_ref
                />
            </div>

            <button
                type="submit"
                class="bg-blue-600 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
            >
                "Log In"
            </button>
        </form>
    }
}

async fn derive_key_websys(
    password: &str,
    salt: &str,
) -> Result<CryptoKey, JsValue> {
    let iterations = 100000;
    let key_length = 256;
    let window = web_sys::window().expect("no global `window` exists");
    let crypto = window.crypto().expect("no `crypto` on `window`");
    let subtle = crypto.subtle();

    let password_data = string_to_uint8array(password);
    let salt_data = string_to_uint8array(salt);

    let key_usages_js = js_sys::Array::new();
    key_usages_js.push(&JsValue::from_str("deriveKey"));

    let password_key_promise = subtle.import_key_with_str(
        "raw",
        &password_data,
        "PBKDF2",
        false,
        &key_usages_js.into(),
    )?;

    let password_key: CryptoKey =
        wasm_bindgen_futures::JsFuture::from(password_key_promise)
            .await?
            .dyn_into()?;

    let key_usages_js = js_sys::Array::new();
    key_usages_js.push(&JsValue::from_str("encrypt"));
    key_usages_js.push(&JsValue::from_str("decrypt"));

    let derived_key_promise = subtle.derive_key_with_object_and_object(
        &Pbkdf2Params::new(
            "PBKDF2",
            &JsValue::from_str("SHA-256"),
            iterations,
            &salt_data.into(),
        ),
        &password_key,
        &AesKeyGenParams::new("AES-GCM", key_length),
        true,
        &key_usages_js.into(),
    )?;

    let derived_key: CryptoKey =
        wasm_bindgen_futures::JsFuture::from(derived_key_promise)
            .await?
            .dyn_into()?;

    // used for temporary logging
    let raw_key_promise = subtle.export_key("raw", &derived_key)?;
    let raw_key: js_sys::ArrayBuffer =
        wasm_bindgen_futures::JsFuture::from(raw_key_promise)
            .await?
            .dyn_into()?;
    let raw_key_vec: Vec<u8> = js_sys::Uint8Array::new(&raw_key).to_vec();
    log::info!("raw_key_vec: {:?}", raw_key_vec);

    Ok(derived_key)
}
