use async_std::task;
use std::{
        time::{
            Duration,
            Instant,
        },
        sync::{
            Arc,
            mpsc::{
                Sender,
            }
        },
};
use crate::configuration::Configuration;
use surf::http::StatusCode;

pub async fn web_mon_start(config: Arc<Configuration>, slack_tx: Sender<String>) -> Result<(),String> {
    if config.monitor_urls.is_none() {
        log::warn!("web monitoring is not configured");
        return Err("Web monitoring is not configured".to_string())
    }

    let monitor_urls = config.monitor_urls.as_ref().unwrap();
    //this is to send the config to slack every config_resend_status_minutes minutes even if 
    //the configuration is good
    let mut now = Instant::now();

    loop {
        //let res = surf::get("https://www.yachtlogger.com/heartbeat").await;

        //set this flag to true if something goes wrong with one of the urls. Only send
        //messages to slack if it is true
        let mut bad_result = false;
        let mut results: Vec<String> = Vec::new();
        for url in monitor_urls.iter() {
            let res = surf::get(url).await;
            match res {
                Err(e) => {
                    bad_result = true;
                    let s = format!("Error connecting to {}. {}",url, e.to_string());
                    log::error!("{}",s);
                    results.push(s);
                },
                Ok(result) => {
                    let s = format!("http status for {} is {}",url, result.status());
                    log::info!("{}",s);
                    if result.status() != StatusCode::OK { bad_result = true;}
                    results.push(s)
                }
            }
        }
        
        if config.is_slack_configured() {
            let do_slack = now.elapsed().as_secs() >= config.resend_status_minutes*60;
            if do_slack {
                log::info!("web_mon is resending its status");
                now = Instant::now();
            }

            if bad_result || do_slack {
                for res in results.iter() {
                    if let Err(e) = slack_tx.send(res.to_string()) {
                        log::error!("Could not send to slack:{}",e.to_string());
                        continue;
                    }
                }
            }
        }
        task::sleep(Duration::from_secs(config.sleep_seconds)).await;
    }
}
