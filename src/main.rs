use futures::try_join;
use std::sync::Arc;
use async_std::task;
use yam_lib::configuration::Configuration;
use yam_lib::mysql_mon::mysql_mon_start;
use yam_lib::web_mon::web_mon_start;
use yam_lib::log_mon::log_mon_start;



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
    let config_arc3 = config_arc.clone();

    let handle1 = task::spawn(async move {
        mysql_mon_start(config_arc).await });
    let handle2 = task::spawn(async move {
        web_mon_start(config_arc2).await});
    let handle3 = task::spawn(async move {
        log_mon_start(config_arc3).await});
    
    /*let log_tasks = match log_mon_start(config_arc3) {
        Err(_) => return,
        Ok(t) => t
    };*/
    
    let res = try_join!(handle1,handle2,handle3);
    if let Err(e) = res {
        log::error!("{}",&e);
    }
}
