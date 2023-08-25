use leptos::*;

use super::list_view::ConfigurationListView;


#[component]
pub fn Apps(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <div class="mt-0 p-4 bg-gray-200 rounded shadow w-full flex flex-wrap space-x-4">
            <a href="/apps/objectstore-s3" class="flex min-w-[16rem] w-1/5 max-w-sm py-4 px-4 bg-green-400 text-gray-800 hover:bg-green-500 transition duration-300 ease-in-out font-mono font-bold rounded mb-2">ObjectstoreS3</a>
        </div>
    }
}

#[component]
pub fn AppConfiguration(cx: Scope) -> impl IntoView {
    view! { cx,
        <ConfigurationListView />
    }
}
