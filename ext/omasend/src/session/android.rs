//! Android services use system pickers; NSH continues to own application UI.
use anyhow::{Context, Result, bail};
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JClass, JString, JValue},
    sys::jlong,
};
use omasend::model::SendItem;
use std::collections::HashMap;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicU64, Ordering},
};

type Reply = async_channel::Sender<Result<serde_json::Value>>;
static VM: OnceLock<JavaVM> = OnceLock::new();
static CLASS: OnceLock<GlobalRef> = OnceLock::new();
static PENDING: OnceLock<Mutex<HashMap<u64, Reply>>> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);

#[unsafe(no_mangle)]
extern "system" fn Java_com_longbridge_nsh_OmasendActivity_initialize(
    mut env: JNIEnv<'_>,
    class: JClass<'_>,
    home: JString<'_>,
    cache: JString<'_>,
) {
    // Called on Activity creation before starting the Rust UI or its workers.
    if VM.get().is_some() {
        return;
    }
    let setup = || -> Result<()> {
        let home: String = env.get_string(&home)?.into();
        let cache: String = env.get_string(&cache)?.into();
        unsafe {
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", format!("{home}/config"));
            std::env::set_var("XDG_DATA_HOME", format!("{home}/data"));
            std::env::set_var("TMPDIR", cache);
        }
        let _ = VM.set(env.get_java_vm()?);
        let _ = CLASS.set(env.new_global_ref(class)?);
        Ok(())
    };
    if let Err(error) = setup() {
        tracing::error!(%error, "Android services initialization failed");
    }
}

#[unsafe(no_mangle)]
extern "system" fn Java_com_longbridge_nsh_OmasendActivity_complete(
    mut env: JNIEnv<'_>,
    _: JClass<'_>,
    id: jlong,
    json: JString<'_>,
) {
    let result = env
        .get_string(&json)
        .map(|value| String::from(value))
        .map_err(anyhow::Error::from)
        .and_then(|value| serde_json::from_str(&value).context("Invalid Android response"));
    if let Some(sender) = PENDING
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .remove(&(id as u64))
    {
        let _ = sender.try_send(result);
    }
}

async fn request(kind: i32, value: &str) -> Result<serde_json::Value> {
    let vm = VM.get().context("Android services unavailable")?;
    let class = CLASS.get().context("Android services unavailable")?;
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let (sender, receiver) = async_channel::bounded(1);
    PENDING
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .insert(id, sender);
    let result = (|| -> Result<()> {
        let mut env = vm.attach_current_thread()?;
        let string = env.new_string(value)?;
        env.call_static_method(
            <&JClass>::from(class.as_obj()),
            "request",
            "(JILjava/lang/String;)V",
            &[
                JValue::Long(id as jlong),
                JValue::Int(kind),
                JValue::Object(string.as_ref()),
            ],
        )?;
        Ok(())
    })();
    if let Err(error) = result {
        PENDING.get().unwrap().lock().unwrap().remove(&id);
        return Err(error);
    }
    let response = receiver.recv().await.context("Android service stopped")??;
    if let Some(error) = response.get("error").and_then(|v| v.as_str()) {
        bail!("{error}");
    }
    Ok(response)
}

fn items(value: serde_json::Value) -> Vec<SendItem> {
    if let Some(text) = value.get("text").and_then(|v| v.as_str()) {
        return vec![SendItem::Text(text.into())];
    }
    value
        .get("files")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
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
