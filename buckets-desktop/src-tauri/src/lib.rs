// SPDX-License-Identifier: MIT
//! Buckets desktop app: a Tauri shell over the buckets library (reads) and
//! the buckets CLI (changes).

mod cli;
mod model;
mod preview;
mod repo;
mod session;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

use model::{ActivityItem, AppStatus, Board, BucketFiles, CommitChange, CommitInfo, FileDiff, Preview, Source};
use session::{repo_root_for, Session, Settings};

struct AppState {
    session: Option<Session>,
    error: Option<String>,
    settings: Mutex<Settings>,
}

impl AppState {
    fn root(&self) -> Result<PathBuf, String> {
        self.session
            .as_ref()
            .map(|s| s.root.clone())
            .ok_or_else(|| "No repository is open".to_string())
    }

    fn settings(&self) -> Settings {
        self.settings
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }
}

/// Run blocking work (database, disk, CLI) off the UI thread.
async fn blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("Background task failed: {}", e))?
}

#[tauri::command]
async fn app_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    let settings = state.settings();
    let repo = state.session.as_ref().map(Session::info);
    let error = state.error.clone();
    let configured = settings.cli_path.clone();
    let cli = blocking(move || Ok(cli::info(configured.as_deref()))).await?;
    Ok(AppStatus {
        repo,
        error,
        me: buckets::data::lock::current_owner(),
        recent: settings
            .recent
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
        cli,
        settings,
    })
}

#[tauri::command]
async fn board(state: State<'_, AppState>) -> Result<Board, String> {
    let root = state.root()?;
    blocking(move || repo::board(&root)).await
}

#[tauri::command]
async fn bucket_files(state: State<'_, AppState>, bucket_id: Uuid) -> Result<BucketFiles, String> {
    let root = state.root()?;
    blocking(move || repo::bucket_files(&root, bucket_id)).await
}

#[tauri::command]
async fn bucket_history(state: State<'_, AppState>, bucket_id: Uuid) -> Result<Vec<CommitInfo>, String> {
    state.root()?;
    blocking(move || repo::history(bucket_id)).await
}

#[tauri::command]
async fn commit_changes(
    state: State<'_, AppState>,
    bucket_id: Uuid,
    commit_id: Uuid,
) -> Result<Vec<CommitChange>, String> {
    state.root()?;
    blocking(move || repo::commit_changes(bucket_id, commit_id)).await
}

#[tauri::command]
async fn activity(state: State<'_, AppState>, limit: Option<i64>) -> Result<Vec<ActivityItem>, String> {
    let root = state.root()?;
    blocking(move || repo::activity(&root, limit.unwrap_or(100).clamp(1, 1000))).await
}

#[tauri::command]
async fn file_preview(
    state: State<'_, AppState>,
    bucket_id: Uuid,
    path: String,
    source: Source,
) -> Result<Preview, String> {
    let root = state.root()?;
    blocking(move || {
        let bucket = repo::find_bucket(&root, bucket_id)?;
        preview::preview(&root, &bucket, &path, &source)
    })
    .await
}

#[tauri::command]
async fn file_diff(
    state: State<'_, AppState>,
    bucket_id: Uuid,
    path: String,
    left: Source,
    right: Source,
) -> Result<FileDiff, String> {
    let root = state.root()?;
    blocking(move || {
        let bucket = repo::find_bucket(&root, bucket_id)?;
        preview::diff(&root, &bucket, &path, &left, &right)
    })
    .await
}

/// A change, carried out by the buckets CLI.
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
enum Action {
    Commit { bucket_id: Uuid, message: String },
    Lock { bucket_id: Uuid, paths: Vec<String> },
    Unlock { bucket_id: Uuid, paths: Vec<String>, force: bool },
    Finalize { bucket_id: Uuid, pebble_ids: Vec<Uuid> },
    Expect { bucket_id: Uuid, from_bucket_id: Option<Uuid>, description: String },
    Revert { bucket_id: Uuid, path: String, commit_id: Option<Uuid> },
    Stash { bucket_id: Uuid, pop: bool, message: Option<String> },
    Push,
    Pull { bucket_id: Uuid, force: bool },
    RemoteSet { path: String },
}

fn action_args(root: &Path, action: Action) -> Result<(PathBuf, Vec<String>), String> {
    let bucket = |id: Uuid| repo::find_bucket(root, id);
    let s = |v: &str| v.to_string();
    Ok(match action {
        Action::Commit { bucket_id, message } => {
            if message.trim().is_empty() {
                return Err("Write a commit message first".into());
            }
            (bucket(bucket_id)?.abs, vec![s("commit"), message])
        }
        Action::Lock { bucket_id, paths } => {
            let mut args = vec![s("lock")];
            args.extend(paths);
            (bucket(bucket_id)?.abs, args)
        }
        Action::Unlock { bucket_id, paths, force } => {
            let mut args = vec![s("unlock")];
            if force {
                args.push(s("--force"));
            }
            args.extend(paths);
            (bucket(bucket_id)?.abs, args)
        }
        Action::Finalize { bucket_id, pebble_ids } => {
            let b = bucket(bucket_id)?;
            let mut args = vec![s("finalize"), b.name];
            if pebble_ids.is_empty() {
                args.push(s("--all"));
            }
            for id in pebble_ids {
                args.push(s("--pebble"));
                args.push(id.to_string());
            }
            (root.to_path_buf(), args)
        }
        Action::Expect { bucket_id, from_bucket_id, description } => {
            if description.trim().is_empty() {
                return Err("Describe what is expected".into());
            }
            let mut args = vec![s("expect"), description, s("--in-bucket"), bucket(bucket_id)?.name];
            if let Some(from) = from_bucket_id {
                args.push(s("--from"));
                args.push(bucket(from)?.name);
            }
            (root.to_path_buf(), args)
        }
        Action::Revert { bucket_id, path, commit_id } => {
            let mut args = vec![s("revert"), path];
            if let Some(id) = commit_id {
                args.push(s("--commit"));
                args.push(id.to_string());
            }
            (bucket(bucket_id)?.abs, args)
        }
        Action::Stash { bucket_id, pop, message } => {
            let mut args = vec![s("stash")];
            if pop {
                args.push(s("pop"));
            } else {
                args.push(s("save"));
                if let Some(m) = message.filter(|m| !m.trim().is_empty()) {
                    args.push(s("--message"));
                    args.push(m);
                }
            }
            (bucket(bucket_id)?.abs, args)
        }
        Action::Push => (root.to_path_buf(), vec![s("push")]),
        Action::Pull { bucket_id, force } => {
            let mut args = vec![s("pull")];
            if force {
                args.push(s("--force"));
            }
            (bucket(bucket_id)?.abs, args)
        }
        Action::RemoteSet { path } => (root.to_path_buf(), vec![s("remote"), s("set"), path]),
    })
}

#[tauri::command]
async fn run_action(state: State<'_, AppState>, action: Action) -> Result<Value, String> {
    let root = state.root()?;
    let settings = state.settings();
    blocking(move || {
        let cli_path = cli::locate(settings.cli_path.as_deref()).ok_or(
            "The buckets CLI was not found. Install it or set its path in Settings.",
        )?;
        let (cwd, args) = action_args(&root, action)?;
        cli::run(&cli_path, &cwd, &args, settings.similarity_check)
    })
    .await
}

#[tauri::command]
async fn open_file(
    app: AppHandle,
    state: State<'_, AppState>,
    bucket_id: Uuid,
    path: Option<String>,
    reveal: bool,
) -> Result<(), String> {
    let root = state.root()?;
    let bucket = blocking(move || repo::find_bucket(&root, bucket_id)).await?;
    let target = match &path {
        Some(p) if buckets::utils::object_store::is_safe_relative(p) => bucket.abs.join(p),
        Some(p) => return Err(format!("Refusing unsafe path '{}'", p)),
        None => bucket.abs.clone(),
    };
    if reveal && path.is_some() {
        app.opener()
            .reveal_item_in_dir(&target)
            .map_err(|e| e.to_string())
    } else {
        app.opener()
            .open_path(target.display().to_string(), None::<&str>)
            .map_err(|e| e.to_string())
    }
}

/// Pick a folder and open it as the repository (restarts the app).
#[tauri::command]
async fn choose_repository(app: AppHandle) -> Result<(), String> {
    let picked = tauri::async_runtime::spawn_blocking({
        let app = app.clone();
        move || app.dialog().file().set_title("Open a buckets repository").blocking_pick_folder()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(folder) = picked else {
        return Ok(());
    };
    let path = folder.into_path().map_err(|e| e.to_string())?;
    open_repository(app, path.display().to_string()).await
}

#[tauri::command]
async fn open_repository(app: AppHandle, path: String) -> Result<(), String> {
    let root = repo_root_for(Path::new(&path))
        .ok_or_else(|| format!("{} is not inside a buckets repository (no .buckets folder)", path))?;
    let state = app.state::<AppState>();
    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.remember(&root);
        let same = state.session.as_ref().map(|s| s.root == root).unwrap_or(false);
        // Launch arguments and BUCKETS_REPO survive the restart, so record the
        // choice as a one-shot override that startup reads first.
        settings.open_next = (!same).then(|| root.clone());
        settings.save()?;
        if same {
            return Ok(());
        }
    }
    app.restart();
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPatch {
    cli_path: Option<String>,
    similarity_check: Option<bool>,
    theme: Option<String>,
}

#[tauri::command]
fn update_settings(state: State<'_, AppState>, patch: SettingsPatch) -> Result<Settings, String> {
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    if let Some(p) = patch.cli_path {
        settings.cli_path = (!p.trim().is_empty()).then(|| PathBuf::from(p.trim()));
    }
    if let Some(v) = patch.similarity_check {
        settings.similarity_check = v;
    }
    if let Some(t) = patch.theme {
        settings.theme = t;
    }
    settings.save()?;
    Ok(settings.clone())
}

fn startup_repository(settings: &mut Settings) -> Option<PathBuf> {
    if let Some(next) = settings.open_next.take() {
        let _ = settings.save();
        if let Some(root) = repo_root_for(&next) {
            return Some(root);
        }
    }
    let from_args = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .find(|p| p.is_dir());
    let from_env = std::env::var_os("BUCKETS_REPO").map(PathBuf::from);
    from_args
        .or(from_env)
        .or_else(|| settings.recent.iter().find(|p| p.is_dir()).cloned())
        .and_then(|p| repo_root_for(&p.canonicalize().unwrap_or(p)))
}

pub fn run() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).try_init();

    let mut settings = Settings::load();
    let (session, error) = match startup_repository(&mut settings) {
        Some(root) => match Session::open(&root) {
            Ok(session) => {
                settings.remember(&root);
                let _ = settings.save();
                (Some(session), None)
            }
            Err(e) => (None, Some(format!("Could not open {}: {}", root.display(), e))),
        },
        None => (None, None),
    };

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            session,
            error,
            settings: Mutex::new(settings),
        })
        .invoke_handler(tauri::generate_handler![
            app_status,
            board,
            bucket_files,
            bucket_history,
            commit_changes,
            activity,
            file_preview,
            file_diff,
            run_action,
            open_file,
            choose_repository,
            open_repository,
            update_settings,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("buckets-desktop failed to start: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_actions_from_the_web_view() {
        let a: Action = serde_json::from_str(
            r#"{"type":"unlock","bucketId":"00000000-0000-0000-0000-000000000001","paths":["a.blend"],"force":true}"#,
        )
        .expect("parse");
        assert!(matches!(a, Action::Unlock { force: true, .. }));
        let p: Action = serde_json::from_str(r#"{"type":"push"}"#).expect("parse");
        assert!(matches!(p, Action::Push));
    }
}
