//! DSA 服务端启动入口

use actix_cors::Cors;
use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};
use clap::Parser;
use deck_connector::Connector;
use std::path::Path;

const DEF_CONFIG_PATH: &str = "conf/config.toml";

#[derive(Parser, Debug)]
#[command(name = "dsa", about = "DSA - Daily Stock Analysis")]
struct Args {
    #[arg(short, long)]
    config: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let conf_path = tube_web::utils::get_abs_path(&args.config.unwrap_or(DEF_CONFIG_PATH.to_owned()));
    let conf_path_str = conf_path.clone();
    let conf_path = Path::new(&conf_path);

    let conf = match dsa_core::config::AppConfig::load(conf_path) {
        Ok(c) => c,
        Err(e) => {
            tube::log!("配置加载失败 {:?}: {:?}", conf_path, e);
            dsa_core::config::AppConfig::default()
        }
    };

    // 通过 Connector 构建数据库连接并注册到 deck 全局缓存
    // set_cache 内部调用 set_connector，key 格式: {db_type}_{name}_connector
    // 后续通过 get_connector("default", "mysql") 获取
    let password = conf.resolve_db_password();
    Connector::new("mysql")
        .server(&conf.database.host)
        .port(conf.database.port as u16)
        .user(&conf.database.user)
        .password(&password)
        .db(&conf.database.name)
        .set_cache("default");

    tube::log!("MySQL连接池已注册: mysql://{}@{}:{}/{}", conf.database.user, conf.database.host, conf.database.port, conf.database.name);

    // 运行数据库迁移
    if let Some(conn) = deck_connector::get_connector("default", "mysql") {
        dsa_core::db::run_migrations(&conn);
    } else {
        tube::log!("数据库连接未初始化，跳过迁移");
    }

    dsa_core::set_config_path(conf_path_str);
    dsa_core::set_global_config(conf.clone());

    tube_web::logs::initialize_logging("");

    let ip = format!("{}:{}", conf.server.host, conf.server.port);
    tube::log!("DSA server starting at {}", ip);
    tube::log!("LLM provider: {}, model: {}", conf.llm.provider, conf.llm.model);
    tube::log!("Database: {}:{}", conf.database.host, conf.database.port);

    let cors_origins = conf.server.cors_origins.clone();
    HttpServer::new(move || {
        let mut cors = Cors::default()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .max_age(3600);
        if cors_origins.is_empty() {
            cors = cors.allow_any_origin();
        } else {
            for origin in &cors_origins {
                cors = cors.allowed_origin(origin);
            }
        }
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .service(
                web::resource("/health")
                    .route(web::get().to(dsa_server::router::health_check)),
            )
            .service(
                web::scope("/api/v1")
                    .service(
                        web::resource("/agent/chat/stream")
                            .route(web::post().to(dsa_server::handler::stream::chat_stream)),
                    )
                    .service(
                        web::resource("/{cls}")
                            .route(web::to(dsa_server::router::api_handler)),
                    )
                    .service(
                        web::resource("/{cls}/{tail:.*}")
                            .route(web::to(dsa_server::router::api_handler)),
                    ),
            )
            .service(fs::Files::new("/", "./web/dist").index_file("index.html"))
    })
    .bind(ip)?
    .workers(12)
    .keep_alive(std::time::Duration::from_secs(5))
    .run()
    .await
}
