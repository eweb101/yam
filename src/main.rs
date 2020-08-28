use futures::try_join;
use std::sync::Arc;
use async_std::task;
use yam_lib::configuration::Configuration;
use yam_lib::database_mon::database_mon_start;
use yam_lib::web_mon::web_mon_start;



#[async_std::main]
//async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
async fn main() {
    env_logger::init();

    let filename = dotenv::var("YAM_CONFIG_FILE");
    let filename = match filename {
        Err(_e) => {
            log::error!("YAM_CONFIG_FILE environment variable not set.");
            return;
        },
        Ok(s) => s
    };

    let config = Configuration::from_filename(&filename).await;
    let config = match config {
        Err(e) => {
            log::error!("Config file parsing failed: {}", e);
            return;
        },
        Ok(c) => c
    };

    let config_arc = Arc::new(config);
    let config_arc2 = config_arc.clone();

    let handle1 = task::spawn(async move {
        database_mon_start(config_arc).await });
    let handle2 = task::spawn(async move {
        web_mon_start(config_arc2).await});
    let res = try_join!(handle1,handle2);
    if let Err(e) = res {
        log::error!("{}",&e);
    }
}
