//! Small UIKit service bridge; NSH owns the UI and its event loop.
use anyhow::{Context, Result, bail};
use omasend::model::SendItem;
use std::ffi::{CStr, CString, c_char, c_void};

unsafe extern "C" {
    fn omasend_ios_request(
        kind: i32,
        value: *const c_char,
        context: *mut c_void,
        callback: extern "C" fn(*mut c_void, *const c_char),
    );
}

type Reply = async_channel::Sender<Result<serde_json::Value>>;

extern "C" fn complete(context: *mut c_void, value: *const c_char) {
    // The native bridge calls back exactly once and retains the JSON until return.
    let sender = unsafe { Box::from_raw(context.cast::<Reply>()) };
    let result = if value.is_null() {
        Err(anyhow::anyhow!("iOS service returned no response"))
    } else {
        serde_json::from_slice(unsafe { CStr::from_ptr(value) }.to_bytes())
            .context("Invalid iOS service response")
    };
    let _ = sender.try_send(result);
}

async fn request(kind: i32, value: &str) -> Result<serde_json::Value> {
    let value = CString::new(value)?;
    let (sender, receiver) = async_channel::bounded::<Result<serde_json::Value>>(1);
    unsafe {
        omasend_ios_request(
            kind,
            value.as_ptr(),
            Box::into_raw(Box::new(sender)).cast(),
            complete,
        );
    }
    let response = receiver.recv().await.context("iOS service stopped")??;
    if let Some(error) = response.get("error").and_then(|value| value.as_str()) {
        bail!("{error}");
    }
    Ok(response)
}

fn items(response: serde_json::Value) -> Vec<SendItem> {
    if let Some(text) = response.get("text").and_then(|value| value.as_str()) {
        return vec![SendItem::Text(text.into())];
    }
    response
        .get("files")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .map(|path| SendItem::File(path.into()))
        .collect()
}

pub async fn pick(folders: bool) -> Result<Vec<SendItem>> {
    request(i32::from(folders), "").await.map(items)
}

pub async fn photos() -> Result<Vec<SendItem>> {
    request(5, "").await.map(items)
}

pub async fn paste() -> Result<Vec<SendItem>> {
    request(2, "").await.map(items)
}

pub async fn open_url(url: &str) -> Result<()> {
    request(3, url).await.map(|_| ())
}

pub async fn share(path: &std::path::Path) -> Result<()> {
    request(4, path.to_str().context("Invalid file path")?)
        .await
        .map(|_| ())
}
