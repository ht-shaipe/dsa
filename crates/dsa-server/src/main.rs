//! DSA 服务端启动入口

use clap::Parser;
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

    tube_web::logs::initialize_logging("");
    dsa_server::setup_database(&conf);

    let server_config = dsa_server::ServerConfig {
        host: conf.server.host.clone(),
        port: conf.server.port as u16,
        cors_origins: conf.server.cors_origins.clone(),
        static_dir: None,
    };

    dsa_server::start_server(conf, conf_path_str, server_config).await
}
