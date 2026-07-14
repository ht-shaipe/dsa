#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dsa_core::config::AppConfig;
use std::path::Path;
use std::sync::{Arc, Mutex};

const APP_SERVER_PORT: u16 = 18080;

static SERVER_SHUTDOWN: Mutex<Option<Arc<tokio::sync::Notify>>> = Mutex::new(None);

fn main() {
    let (conf_path_str, conf) = load_config();
    let static_dir = find_static_dir();
    let shutdown_notify = Arc::new(tokio::sync::Notify::new());
    {
        let mut guard = SERVER_SHUTDOWN.lock().unwrap();
        *guard = Some(shutdown_notify.clone());
    }

    let conf_clone = conf.clone();
    let conf_path_clone = conf_path_str.clone();
    let static_dir_clone = static_dir.clone();
    let notify = shutdown_notify.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime for Actix-web");

        rt.block_on(async {
            tube_web::logs::initialize_logging("");
            dsa_server::setup_database(&conf_clone);

            let server_config = dsa_server::ServerConfig {
                host: "127.0.0.1".to_string(),
                port: APP_SERVER_PORT,
                cors_origins: vec![],
                static_dir: Some(static_dir_clone),
            };

            let server = dsa_server::start_server(conf_clone, conf_path_clone, server_config);
            tokio::pin!(server);

            tokio::select! {
                result = &mut server => {
                    if let Err(e) = result {
                        eprintln!("Actix-web server error: {}", e);
                    }
                }
                _ = notify.notified() => {
                    tube::log!("Actix-web server shutting down...");
                }
            }
        });
    });

    wait_for_server(APP_SERVER_PORT);

    dsa_app::run(APP_SERVER_PORT);
}

fn load_config() -> (String, AppConfig) {
    let config_arg = std::env::args().nth(1).unwrap_or_default();

    let candidates = if !config_arg.is_empty() {
        vec![tube_web::utils::get_abs_path(&config_arg)]
    } else {
        let cwd = std::env::current_dir().unwrap_or_default();
        vec![
            tube_web::utils::get_abs_path("conf/config.toml"),
            cwd.join("../../conf/config.toml").canonicalize().unwrap_or_else(|_| cwd.join("../../conf/config.toml")).to_string_lossy().to_string(),
            cwd.join("../../../conf/config.toml").canonicalize().unwrap_or_else(|_| cwd.join("../../../conf/config.toml")).to_string_lossy().to_string(),
        ]
    };

    for conf_path_str in &candidates {
        let conf_path = Path::new(conf_path_str);
        if conf_path.exists() {
            println!("Loading config from: {:?}", conf_path);
            if let Ok(c) = AppConfig::load(conf_path) {
                return (conf_path_str.clone(), c);
            }
        }
    }

    eprintln!("Warning: No config file found, using defaults (SQLite)");
    let mut conf = AppConfig::default();
    conf.database.db_type = "sqlite".to_string();
    ("".to_string(), conf)
}

fn find_static_dir() -> String {
    if cfg!(debug_assertions) {
        let dev_path = tube_web::utils::get_abs_path("./web/dist");
        if Path::new(&dev_path).exists() {
            return dev_path;
        }
        let cwd = std::env::current_dir().unwrap_or_default();
        let alt_path = cwd.join("../../web/dist").canonicalize().unwrap_or_else(|_| cwd.join("../../web/dist"));
        if alt_path.exists() {
            return alt_path.to_string_lossy().to_string();
        }
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    if let Some(ref dir) = exe_dir {
        let candidates: Vec<std::path::PathBuf> = if cfg!(target_os = "macos") {
            vec![
                dir.join("../Resources/web/dist"),
                dir.join("../../web/dist"),
            ]
        } else {
            vec![
                dir.join("web/dist"),
                dir.join("../../web/dist"),
            ]
        };

        for candidate in candidates {
            let canonical = candidate.canonicalize().unwrap_or_else(|_| candidate.clone());
            if canonical.exists() {
                tube::log!("Found static files at: {:?}", canonical);
                return canonical.to_string_lossy().to_string();
            }
        }
    }

    let fallback = "./web/dist".to_string();
    tube::log!("Static dir fallback to: {}", fallback);
    fallback
}

fn wait_for_server(port: u16) {
    let url = format!("http://127.0.0.1:{}/health", port);
    for i in 0..50 {
        if reqwest::blocking::get(&url).is_ok() {
            println!("Backend server ready on port {}", port);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        if i % 10 == 0 {
            println!("Waiting for backend server on port {}...", port);
        }
    }
    eprintln!("Warning: Backend server did not start within 10s, launching window anyway");
}
