//! Keep this place offline: sequential shard fetch, SHA-256 verify, then an
//! Origin Private File System write under `ewm/by-sha256/<sha256>`.
//!
//! Blob `<a download>` is fallback only when OPFS is unavailable. Do not fire
//! one browser download per identity when web storage can hold the bytes.
//!
//! OPFS uses `js_sys::Reflect` against `navigator.storage.getDirectory`,
//! `getDirectory` / `getDirectoryHandle`, `getFileHandle({create:true})`,
//! `createWritable`, `write`, and `close`, so this crate compiles without
//! FileSystem* web-sys features. Integrator may later add:
//! `StorageManager`, `Navigator`, `FileSystemDirectoryHandle`,
//! `FileSystemFileHandle`, `FileSystemGetDirectoryOptions`,
//! `FileSystemGetFileOptions`, `FileSystemWritableFileStream`.
//! Do not call `write_opfs_block` (QualiaDB 40_960-byte superblocks).

use super::*;
use qualia_core_db::net::disclosure::NetworkDisclosureRegistry;
use qualia_core_db::net::fetch_range_wasm::FetchRangeService;
use qualia_core_db::vibe_host::PoetSnapshot;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

static DOWNLOAD_BUSY: AtomicBool = AtomicBool::new(false);

pub(super) fn install(parent: &Element) -> Result<(), JsValue> {
    let wrap = doc().create_element("div")?;
    wrap.set_id("native-ewm-download");
    wrap.set_attribute("role", "region")?;
    wrap.set_attribute("aria-label", "Keep this place offline")?;
    let button = doc().create_element("button")?;
    button.set_attribute("type", "button")?;
    button.set_id("native-ewm-download-button");
    button.set_attribute("aria-label", "Keep this place offline in the browser")?;
    button.set_text_content(Some("Keep this place offline"));
    wrap.append_child(&button)?;
    let status = doc().create_element("p")?;
    status.set_id("native-ewm-download-status");
    status.set_attribute("role", "status")?;
    status.set_attribute("aria-live", "polite")?;
    status.set_text_content(Some(
        "Keeps the same first-paint files the stream uses in this browser’s Origin Private File System, under those SHA-256 identities. The downloads folder is used only if web storage is unavailable.",
    ));
    wrap.append_child(&status)?;
    let reading = doc()
        .create_element("a")?
        .dyn_into::<HtmlAnchorElement>()?;
    reading.set_id("native-ewm-download-manifest");
    reading.set_href("/data/ewm/catalog.q42");
    reading.set_text_content(Some("Catalog Q42 for the reading view"));
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
        "Opening the catalog Q42…",
    );
    let catalog_bytes = bytes("/data/ewm/catalog.q42", 2 * 1024 * 1024).await?;
    let mut host = PoetSnapshot::default();
    super::super::knowledge::admit(&mut host, &catalog_bytes, "catalog", "catalog.q42").map_err(err)?;
    let catalog = super::super::catalog::Catalog::from_host(&mut host).map_err(err)?;
    if catalog.streaming_default {
        return Err(err("Download pack must keep streaming.default false"));
    }
    let identities: Vec<_> = catalog
        .first_paint
        .iter()
        .filter_map(|id| catalog.entry(id))
        .collect();
    if identities.len() != 24 {
        return Err(err("Download pack first-paint must stay 24"));
    }
    let fetch = FetchRangeService::new(NetworkDisclosureRegistry::new());
    let controller = web_sys::AbortController::new()?;
    let mut opfs_dir = match opfs_identity_dir().await {
        Ok(dir) => Some(dir),
        Err(_) => None,
    };
    for (n, row) in identities.iter().enumerate() {
        text(
            "native-ewm-download-status",
            &format!(
                "Keeping {} of {} verified files in this browser…",
                n + 1,
                identities.len()
            ),
        );
        let url = row.url.as_str();
        fetch.authorize(url).map_err(err)?;
        let raw = fetch
            .get_whole_entry(url, 64 * 1024 * 1024, None, Some(&controller.signal()))
            .await
            .map_err(|e| {
                controller.abort();
                err(e)
            })?;
        let want = row.sha256.as_str();
        if hash(&raw) != want {
            controller.abort();
            return Err(err(format!("Download digest mismatch: {}", row.scene_id)));
        }
        if let Some(dir) = &opfs_dir {
            if write_opfs_identity(dir, want, &raw).await.is_err() {
                opfs_dir = None;
            }
        }
        if opfs_dir.is_none() {
            let name = format!("{}.10d", row.scene_id);
            save_file(&name, &raw)?;
        }
        if n + 1 < identities.len() {
            pause_ms(400).await?;
        }
    }
    if opfs_dir.is_some() {
        text(
            "native-ewm-download-status",
            "Kept an Origin Private File System offline copy of the same SHA-256 identities. Nothing was sent to the downloads folder.",
        );
    } else {
        text(
            "native-ewm-download-status",
            "Saved to your downloads folder. Origin Private File System is unavailable in this browser; the disk save succeeded.",
        );
    }
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

fn create_true() -> js_sys::Object {
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"create".into(), &JsValue::TRUE);
    opts
}

async fn call_async(this: &JsValue, method: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    let value = js_sys::Reflect::get(this, &method.into())?;
    if !value.is_function() {
        return Err(err(format!(
            "Origin Private File System {method} is unavailable in this browser"
        )));
    }
    let f: js_sys::Function = value.dyn_into()?;
    let result = match args.len() {
        0 => f.call0(this)?,
        1 => f.call1(this, &args[0])?,
        2 => f.call2(this, &args[0], &args[1])?,
        _ => return Err(err("OPFS helper argument limit")),
    };
    JsFuture::from(js_sys::Promise::resolve(&result)).await
}

async fn get_or_create_dir(parent: &JsValue, name: &str) -> Result<JsValue, JsValue> {
    let opts: JsValue = create_true().into();
    let name_v = JsValue::from_str(name);
    if js_sys::Reflect::get(parent, &"getDirectoryHandle".into())
        .ok()
        .map(|v| v.is_function())
        .unwrap_or(false)
    {
        call_async(parent, "getDirectoryHandle", &[name_v, opts]).await
    } else {
        call_async(parent, "getDirectory", &[name_v, opts]).await
    }
}

async fn storage_root() -> Result<JsValue, JsValue> {
    let window = web_sys::window().ok_or_else(|| err("no window context"))?;
    let navigator = js_sys::Reflect::get(&window, &"navigator".into())?;
    let storage = js_sys::Reflect::get(&navigator, &"storage".into())?;
    if storage.is_undefined() || storage.is_null() {
        return Err(err(
            "Origin Private File System is unavailable in this browser",
        ));
    }
    call_async(&storage, "getDirectory", &[]).await
}

async fn opfs_identity_dir() -> Result<JsValue, JsValue> {
    let root = storage_root().await?;
    let ewm = get_or_create_dir(&root, "ewm").await?;
    get_or_create_dir(&ewm, "by-sha256").await
}

async fn write_opfs_identity(dir: &JsValue, sha256: &str, bytes: &[u8]) -> Result<(), JsValue> {
    let opts: JsValue = create_true().into();
    let file = call_async(
        dir,
        "getFileHandle",
        &[JsValue::from_str(sha256), opts],
    )
    .await?;
    let writable = call_async(&file, "createWritable", &[]).await?;
    let array = js_sys::Uint8Array::from(bytes);
    call_async(&writable, "write", &[array.into()]).await?;
    call_async(&writable, "close", &[]).await?;
    Ok(())
}
