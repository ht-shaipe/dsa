//! DSA Server - HTTP 服务入口

#[macro_use]
extern crate tube;

pub use tube::Error;

pub mod handler;
pub mod strip_auth;
pub mod router;
pub mod state;

use actix_cors::Cors;
use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};

pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub static_dir: Option<String>,
}

async fn spa_fallback(req: actix_web::HttpRequest) -> actix_web::HttpResponse {
    let static_dir = req
        .app_data::<web::Data<String>>()
        .map(|d| d.as_str())
        .unwrap_or("./web/dist");
    let index_path = std::path::Path::new(static_dir).join("index.html");
    match actix_files::NamedFile::open_async(&index_path).await {
        Ok(f) => f.into_response(&req),
        Err(_) => actix_web::HttpResponse::NotFound().finish(),
    }
}

pub async fn start_server(
    conf: dsa_core::config::AppConfig,
    conf_path_str: String,
    server_config: ServerConfig,
) -> std::io::Result<()> {
    dsa_core::set_config_path(conf_path_str);
    dsa_core::set_global_config(conf.clone());

    strip_auth::set_local_mode(conf.server.is_local_mode());

    let ip = format!("{}:{}", server_config.host, server_config.port);
    tube::log!("DSA server starting at {}", ip);
    tube::log!(
        "LLM provider: {}, model: {}",
        conf.llm.provider,
        conf.llm.model
    );

    let static_dir = server_config
        .static_dir
        .clone()
        .unwrap_or_else(|| "./web/dist".to_string());
    tube::log!("Static files directory: {}", static_dir);

    let static_dir_data = web::Data::new(static_dir.clone());
    let is_embedded = server_config.cors_origins.is_empty();
    let cors_origins = server_config.cors_origins.clone();
    HttpServer::new(move || {
        let cors = if is_embedded {
            Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header()
                .max_age(3600)
        } else {
            let mut c = Cors::default()
                .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                .max_age(3600);
            for origin in &cors_origins {
                c = c.allowed_origin(origin);
            }
            c
        };
        App::new()
            .app_data(static_dir_data.clone())
            .wrap(strip_auth::StripAuth)
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .service(web::resource("/health").route(web::get().to(router::health_check)))
            .service(
                web::scope("/api/v1")
                    .service(
                        web::resource("/auth/login").route(web::post().to(handler::auth::login)),
                    )
                    .service(
                        web::resource("/auth/register")
                            .route(web::post().to(handler::auth::register)),
                    )
                    .service(
                        web::resource("/auth/profile")
                            .route(web::post().to(handler::auth::get_profile)),
                    )
                    .service(
                        web::resource("/auth/profile/update")
                            .route(web::post().to(handler::auth::update_profile)),
                    )
                    .service(
                        web::resource("/auth/change-password")
                            .route(web::post().to(handler::auth::change_password)),
                    )
                    .service(
                        web::resource("/proxy").route(web::post().to(handler::proxy::proxy_post)),
                    )
                    .service(
                        web::resource("/agent/chat/stream")
                            .route(web::post().to(handler::stream::chat_stream)),
                    )
                    .service(
                        web::resource("/analysis/stream")
                            .route(web::post().to(handler::analysis_stream::analysis_stream)),
                    )
                    .service(
                        web::resource("/task/progress/stream")
                            .route(web::get().to(handler::task_progress::task_progress_stream)),
                    )
                    .service(web::resource("/{cls}").route(web::to(router::api_handler)))
                    .service(web::resource("/{cls}/{tail:.*}").route(web::to(router::api_handler))),
            )
            .service(
                fs::Files::new("/", &static_dir)
                    .index_file("index.html")
                    .default_handler(web::to(spa_fallback)),
            )
    })
    .bind(ip)?
    .workers(4)
    .keep_alive(std::time::Duration::from_secs(5))
    .run()
    .await
}

pub fn setup_database(conf: &dsa_core::config::AppConfig) {
    let cwd = std::env::current_dir().unwrap_or_default();
    tube::log!("setup_database CWD: {:?}", cwd);
    let connector = conf.build_connector();
    let db_type = &conf.database.db_type;
    let cache_key = format!("{}_{}_connector", db_type.to_lowercase(), "default");
    tube::log!(
        "Registering connector: db_type={}, database={}, cache_key={}",
        db_type,
        connector.database,
        cache_key
    );
    connector.set_cache("default");

    if conf.database.is_sqlite() {
        tube::log!("SQLite连接已注册: {}", conf.database.name);
    } else {
        tube::log!(
            "MySQL连接池已注册: mysql://{}@{}:{}/{}",
            conf.database.user,
            conf.database.host,
            conf.database.port,
            conf.database.name
        );
    }

    if let Some(conn) = deck_connector::get_connector("default", db_type) {
        tube::log!(
            "数据库连接获取成功，执行迁移, db_type={}, conn_str={}",
            db_type,
            conn.get_conn_str()
        );
        dsa_core::db::run_migrations(&conn);
        tube::log!("迁移执行完毕");
    } else {
        tube::log!("数据库连接未初始化(key={})，尝试替代查找...", cache_key);
        for alt in &["sqlite", "Sqlite", "mysql", "Mysql"] {
            if let Some(conn) = deck_connector::get_connector("default", alt) {
                tube::log!("通过替代key({})找到连接，执行迁移", alt);
                dsa_core::db::run_migrations(&conn);
                return;
            }
        }
        tube::log!("所有替代查找均失败，跳过迁移");
    }
}
