//use futures::join;
use std::{
        sync::{
            Arc,
            mpsc::{
                channel,
            }
        },
};
use async_std::task;
use yam_lib::configuration::Configuration;
use yam_lib::mysql_mon::mysql_mon_start;
use yam_lib::web_mon::web_mon_start;
//use yam_lib::log_mon::log_mon_start;
use yam_lib::slack::start_slack_poster;



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
    let (slack_tx, slack_rx) = channel();

    let mut handles = Vec::new();

    if config_arc.is_db_configured() {
        let ca = config_arc.clone();
        let tx = slack_tx.clone();
        let handle = task::spawn(async move {
            mysql_mon_start(ca,tx).await});
        handles.push(handle);
    }
    if config_arc.is_slack_configured() {
        let su = config_arc.slack_url.as_ref().unwrap().clone();
        let handle = task::spawn(async move {
            start_slack_poster(su, slack_rx).await});
        handles.push(handle);
    }

    let tx = slack_tx.clone();
    let ca = config_arc.clone();
    let handle = task::spawn(async move {
        web_mon_start(ca,tx).await});
    handles.push(handle);

    /*let ca = config_arc.clone();
    let tx = slack_tx.clone();
    let mut log_handles = Vec::new();
    match log_mon_start(ca,tx) {
        Err(_) => log::info!("log monitor not started"),
        Ok(h) => log_handles.extend(h)
    };*/
    
    for h in handles {
        let result = h.await;
        if let Err(e) = result {
            log::error!("{}",&e);
        }
    }
}
