use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct DetectionStateDto {
    detected: Option<DetectedMedia>,
    outcome: Option<String>,
    status_message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct DetectedMedia {
    player_name: String,
    anime_title: Option<String>,
    episode: Option<u32>,
    service_name: Option<String>,
    raw_title: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AnimeSearchResult {
    service_id: u64,
    title: String,
    title_english: Option<String>,
    episodes: Option<u32>,
}

#[wasm_bindgen(inline_js = "
export async function tauriInvoke(cmd, args) {
  if (!window.__TAURI_INTERNALS__?.invoke) throw new Error('Tauri invoke unavailable');
  return await window.__TAURI_INTERNALS__.invoke(cmd, args ?? {});
}
")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn tauriInvoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

async fn invoke<T: for<'de> Deserialize<'de>>(cmd: &str, args: serde_json::Value) -> Result<T, String> {
    let js = JsValue::from_str(&args.to_string());
    let out = tauriInvoke(cmd, js).await.map_err(js_err)?;
    serde_wasm_bindgen::from_value(out).map_err(|e| e.to_string())
}

fn js_err(err: JsValue) -> String {
    err.as_string().unwrap_or_else(|| "unknown js error".into())
}

#[component]
fn App() -> impl IntoView {
    let (tab, set_tab) = signal("now_playing".to_string());
    let (status, set_status) = signal("Loading...".to_string());
    let (detection, set_detection) = signal(DetectionStateDto::default());
    let (library_json, set_library_json) = signal(String::new());
    let (search_q, set_search_q) = signal(String::new());
    let (search_results, set_search_results) = signal(Vec::<AnimeSearchResult>::new());

    Effect::new(move |_| {
        spawn_local(async move {
            match invoke::<DetectionStateDto>("get_now_playing_state", serde_json::json!({})).await {
                Ok(state) => {
                    set_status.set(state.status_message.clone());
                    set_detection.set(state);
                }
                Err(e) => set_status.set(format!("Failed to load state: {e}")),
            }
        });
    });

    let refresh_library = move |_| {
        spawn_local(async move {
            match invoke::<serde_json::Value>("get_library", serde_json::json!({ "status": null, "query": null })).await {
                Ok(rows) => set_library_json.set(rows.to_string()),
                Err(e) => set_library_json.set(format!("Error: {e}")),
            }
        });
    };

    let run_search = move |_| {
        let q = search_q.get_untracked();
        spawn_local(async move {
            match invoke::<Vec<AnimeSearchResult>>("search_remote", serde_json::json!({ "query": q })).await {
                Ok(rows) => set_search_results.set(rows),
                Err(e) => set_status.set(format!("Search failed: {e}")),
            }
        });
    };

    view! {
        <main class="app">
            <header>
                <h1>"Ryuuji"</h1>
                <p>{move || status.get()}</p>
            </header>

            <nav class="tabs">
                <button on:click=move |_| set_tab.set("now_playing".into())>"Now Playing"</button>
                <button on:click=move |_| set_tab.set("library".into())>"Library"</button>
                <button on:click=move |_| set_tab.set("search".into())>"Search"</button>
                <button on:click=move |_| set_tab.set("settings".into())>"Settings"</button>
            </nav>

            {move || match tab.get().as_str() {
                "now_playing" => view! {
                    <section>
                        <h2>"Now Playing"</h2>
                        <p>{move || format!("Player: {}", detection.get().detected.as_ref().map(|d| d.player_name.clone()).unwrap_or_else(|| "None".into()))}</p>
                        <p>{move || format!("Title: {}", detection.get().detected.as_ref().and_then(|d| d.anime_title.clone()).unwrap_or_else(|| "-".into()))}</p>
                        <p>{move || format!("Episode: {}", detection.get().detected.as_ref().and_then(|d| d.episode).map(|x| x.to_string()).unwrap_or_else(|| "-".into()))}</p>
                    </section>
                }.into_any(),
                "library" => view! {
                    <section>
                        <h2>"Library"</h2>
                        <button on:click=refresh_library>"Refresh"</button>
                        <pre>{move || library_json.get()}</pre>
                    </section>
                }.into_any(),
                "search" => view! {
                    <section>
                        <h2>"Search"</h2>
                        <input
                            prop:value=move || search_q.get()
                            on:input=move |ev| set_search_q.set(event_target_value(&ev))
                            placeholder="Search online"
                        />
                        <button on:click=run_search>"Search"</button>
                        <ul>
                            <For
                                each=move || search_results.get()
                                key=|x| x.service_id
                                children=move |item| view! { <li>{format!("{} ({})", item.title, item.service_id)}</li> }
                            />
                        </ul>
                    </section>
                }.into_any(),
                _ => view! {
                    <section>
                        <h2>"Settings"</h2>
                        <p>"Service auth/import are available via backend commands."</p>
                    </section>
                }.into_any(),
            }}
        </main>
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    mount_to_body(App);
}
