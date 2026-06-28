use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
struct Registry {
    modules: Arc<Mutex<HashMap<String, Vec<ModuleEntry>>>>,
    data_path: PathBuf,
}

#[derive(Clone, Serialize, Deserialize)]
struct ModuleEntry {
    name: String,
    version: String,
    git_url: String,
    #[serde(default = "default_git_rev")]
    git_rev: String,
}

fn default_git_rev() -> String {
    "main".into()
}

#[derive(Deserialize)]
struct PublishBody {
    name: String,
    version: String,
    git_url: String,
    #[serde(default = "default_git_rev")]
    git_rev: String,
    token: Option<String>,
}

#[derive(Deserialize)]
struct ResolveQuery {
    req: Option<String>,
}

#[tokio::main]
async fn main() {
    let data_path = std::env::var("NYRA_REGISTRY_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".nyra/registry/index.json")
        });
    let reg = Registry {
        modules: Arc::new(Mutex::new(load_or_seed(&data_path))),
        data_path: data_path.clone(),
    };
    let app = Router::new()
        .route("/index", get(index_latest))
        .route("/index/:name", get(index_versions))
        .route("/resolve/:name", get(resolve))
        .route("/publish", post(publish))
        .with_state(reg);
    let addr = SocketAddr::from(([127, 0, 0, 1], 9470));
    println!("nyrapkg-registry listening on http://{addr}");
    println!("index file: {}", data_path.display());
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn load_or_seed(path: &PathBuf) -> HashMap<String, Vec<ModuleEntry>> {
    if path.is_file() {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, Vec<ModuleEntry>>>(&text) {
                return map;
            }
        }
    }
    let mut map = HashMap::new();
    seed_builtin(&mut map);
    let _ = persist(path, &map);
    map
}

fn seed_builtin(map: &mut HashMap<String, Vec<ModuleEntry>>) {
    let builtins = [
        ModuleEntry {
            name: "ny-sqlite".into(),
            version: "0.1.0".into(),
            git_url: "https://github.com/nyra-lang/nyra".into(),
            git_rev: "main".into(),
        },
        ModuleEntry {
            name: "ny-serde".into(),
            version: "0.1.0".into(),
            git_url: "https://github.com/nyra-lang/nyra".into(),
            git_rev: "main".into(),
        },
        ModuleEntry {
            name: "ny-toml".into(),
            version: "0.1.0".into(),
            git_url: "https://github.com/nyra-lang/nyra".into(),
            git_rev: "main".into(),
        },
        ModuleEntry {
            name: "ny-crypto".into(),
            version: "0.1.0".into(),
            git_url: "https://github.com/nyra-lang/nyra".into(),
            git_rev: "main".into(),
        },
        ModuleEntry {
            name: "ny-websocket".into(),
            version: "0.1.0".into(),
            git_url: "https://github.com/nyra-lang/nyra".into(),
            git_rev: "main".into(),
        },
        ModuleEntry {
            name: "ny-redis".into(),
            version: "0.1.0".into(),
            git_url: "https://github.com/nyra-lang/nyra".into(),
            git_rev: "main".into(),
        },
        ModuleEntry {
            name: "ny-postgres".into(),
            version: "0.1.0".into(),
            git_url: "https://github.com/nyra-lang/nyra".into(),
            git_rev: "main".into(),
        },
        ModuleEntry {
            name: "ny-mysql".into(),
            version: "0.1.0".into(),
            git_url: "https://github.com/nyra-lang/nyra".into(),
            git_rev: "main".into(),
        },
    ];
    for entry in builtins {
        map.entry(entry.name.clone())
            .or_default()
            .push(entry);
    }
}

fn persist(path: &PathBuf, map: &HashMap<String, Vec<ModuleEntry>>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(map).unwrap();
    std::fs::write(path, text)
}

fn read_token() -> Option<String> {
    dirs::home_dir()
        .map(|h| h.join(".nyra/credentials"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|text| {
            text.lines()
                .find_map(|l| l.strip_prefix("token=").map(str::trim))
                .map(str::to_string)
        })
}

async fn index_latest(State(reg): State<Registry>) -> Json<Vec<ModuleEntry>> {
    let map = reg.modules.lock().unwrap();
    let mut latest = Vec::new();
    for versions in map.values() {
        if let Some(entry) = highest_version(versions) {
            latest.push(entry.clone());
        }
    }
    latest.sort_by(|a, b| a.name.cmp(&b.name));
    Json(latest)
}

async fn index_versions(
    State(reg): State<Registry>,
    Path(name): Path<String>,
) -> Json<Vec<ModuleEntry>> {
    let map = reg.modules.lock().unwrap();
    Json(map.get(&name).cloned().unwrap_or_default())
}

async fn resolve(
    State(reg): State<Registry>,
    Path(name): Path<String>,
    Query(query): Query<ResolveQuery>,
) -> Json<Option<ModuleEntry>> {
    let map = reg.modules.lock().unwrap();
    let versions = match map.get(&name) {
        Some(v) => v,
        None => return Json(None),
    };
    if let Some(req) = query.req.as_deref() {
        if let Ok(parsed) = pkg::parse_req(req) {
            let candidates: Vec<pkg::Version> = versions
                .iter()
                .filter_map(|e| pkg::parse_version(&e.version).ok())
                .collect();
            if let Some(best) = pkg::best_match(&parsed, candidates.iter()) {
                let chosen = versions
                    .iter()
                    .find(|e| pkg::parse_version(&e.version).ok().as_ref() == Some(&best))
                    .cloned();
                return Json(chosen);
            }
            return Json(None);
        }
    }
    Json(highest_version(versions).cloned())
}

async fn publish(
    State(reg): State<Registry>,
    Json(body): Json<PublishBody>,
) -> Json<&'static str> {
    let expected = read_token().unwrap_or_else(|| "nyra-dev-token".into());
    if body.token.as_deref() != Some(expected.as_str()) {
        return Json("unauthorized");
    }
    let entry = ModuleEntry {
        name: body.name.clone(),
        version: body.version.clone(),
        git_url: body.git_url,
        git_rev: body.git_rev,
    };
    let mut map = reg.modules.lock().unwrap();
    let versions = map.entry(body.name).or_default();
    if let Some(idx) = versions.iter().position(|e| e.version == entry.version) {
        versions[idx] = entry;
    } else {
        versions.push(entry);
    }
    let _ = persist(&reg.data_path, &map);
    Json("ok")
}

fn highest_version(versions: &[ModuleEntry]) -> Option<&ModuleEntry> {
    versions.iter().max_by(|a, b| {
        let va = pkg::parse_version(&a.version).unwrap_or(pkg::Version {
            major: 0,
            minor: 0,
            patch: 0,
        });
        let vb = pkg::parse_version(&b.version).unwrap_or(pkg::Version {
            major: 0,
            minor: 0,
            patch: 0,
        });
        va.compare(&vb)
    })
}
