//! Download this place: sequential shard fetch, SHA-256 verify, browser save.
//! Wasm offline is download-to-disk. OPFS is not yet wired.

use super::*;
use qualia_core_db::net::disclosure::NetworkDisclosureRegistry;
use qualia_core_db::net::fetch_range_wasm::FetchRangeService;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

static DOWNLOAD_BUSY: AtomicBool = AtomicBool::new(false);

pub(super) fn install(parent: &Element) -> Result<(), JsValue> {
    let wrap = doc().create_element("div")?;
    wrap.set_id("native-ewm-download");
    wrap.set_attribute("role", "region")?;
    wrap.set_attribute("aria-label", "Download this place")?;
    let button = doc().create_element("button")?;
    button.set_attribute("type", "button")?;
    button.set_id("native-ewm-download-button");
    button.set_attribute("aria-label", "Download this place as verified static files")?;
    button.set_text_content(Some("Download this place"));
    wrap.append_child(&button)?;
    let status = doc().create_element("p")?;
    status.set_id("native-ewm-download-status");
    status.set_attribute("role", "status")?;
    status.set_attribute("aria-live", "polite")?;
    status.set_text_content(Some(
        "Saves the same first-paint files the stream uses. Browser OPFS is not yet wired.",
    ));
    wrap.append_child(&status)?;
    let reading = doc()
        .create_element("a")?
        .dyn_into::<HtmlAnchorElement>()?;
    reading.set_id("native-ewm-download-manifest");
    reading.set_href("/packages/download/first-paint.json");
    reading.set_text_content(Some("Pack list for the reading view"));
    wrap.append_child(&reading)?;
    parent.append_child(&wrap)?;
    event(button.unchecked_ref(), "click", move |_| {
        if DOWNLOAD_BUSY.swap(true, Ordering::SeqCst) {
            return;
        }
        set_download_busy(true);
        wasm_bindgen_futures::spawn_local(async {
            let result = run().await;
            DOWNLOAD_BUSY.store(false, Ordering::SeqCst);
            set_download_busy(false);
            if let Err(e) = result {
                if let Some(status) = doc().get_element_by_id("native-ewm-download-status") {
                    status.set_text_content(Some(
                        "Download could not finish. The reading view remains available.",
                    ));
                }
                show_error(e);
            }
        });
    })?;
    Ok(())
}

fn set_download_busy(busy: bool) {
    if let Some(button) = doc().get_element_by_id("native-ewm-download-button") {
        if busy {
            let _ = button.set_attribute("disabled", "");
            let _ = button.set_attribute("aria-busy", "true");
        } else {
            let _ = button.remove_attribute("disabled");
            let _ = button.remove_attribute("aria-busy");
        }
    }
}

async fn run() -> Result<(), JsValue> {
    text(
        "native-ewm-download-status",
        "Fetching the place pack list…",
    );
    let pack: Value = json_file("/packages/download/first-paint.json", 2 * 1024 * 1024).await?;
    if pack["schema"] != "webizen.ewm-download-pack/v1" {
        return Err(err("Unsupported download pack"));
    }
    if pack["streaming_default"].as_bool() == Some(true) {
        return Err(err("Download pack must keep streaming.default false"));
    }
    let identities = pack["identities"]
        .as_array()
        .ok_or(err("Download pack has no identities"))?;
    let fetch = FetchRangeService::new(NetworkDisclosureRegistry::new());
    let controller = web_sys::AbortController::new()?;
    for (n, row) in identities.iter().enumerate() {
        text(
            "native-ewm-download-status",
            &format!(
                "Downloading {} of {} verified files…",
                n + 1,
                identities.len()
            ),
        );
        let url = row["url"].as_str().ok_or(err("download url"))?;
        fetch.authorize(url).map_err(err)?;
        let raw = fetch
            .get_whole_entry(url, 64 * 1024 * 1024, None, Some(&controller.signal()))
            .await
            .map_err(|e| {
                controller.abort();
                err(e)
            })?;
        let want = row["sha256"].as_str().ok_or(err("download sha256"))?;
        if hash(&raw) != want {
            controller.abort();
            return Err(err(format!(
                "Download digest mismatch: {}",
                row["scene_id"].as_str().unwrap_or("?")
            )));
        }
        let name = format!("{}.10d", row["scene_id"].as_str().unwrap_or("place-entry"));
        save_file(&name, &raw)?;
        if n + 1 < identities.len() {
            pause_ms(400).await?;
        }
    }
    text(
        "native-ewm-download-status",
        "Saved to your downloads folder. This browser session does not yet keep an OPFS offline copy.",
    );
    Ok(())
}

fn save_file(filename: &str, bytes: &[u8]) -> Result<(), JsValue> {
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::of1(&array);
    let options = BlobPropertyBag::new();
    options.set_type("application/octet-stream");
    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options)?;
    let href = Url::create_object_url_with_blob(&blob)?;
    let a = doc()
        .create_element("a")?
        .dyn_into::<HtmlAnchorElement>()?;
    a.set_href(&href);
    a.set_download(filename);
    a.click();
    let _ = Url::revoke_object_url(&href);
    Ok(())
}

async fn pause_ms(ms: i32) -> Result<(), JsValue> {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let _ = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
    });
    JsFuture::from(promise).await?;
    Ok(())
}
