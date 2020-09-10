use serde::{Deserialize, Serialize};
use surf::http::status::StatusCode;
use std::{
    sync::{
        mpsc::{
            Receiver,
        }
    },
};

#[derive(Deserialize, Serialize)]
struct SlackPost {
    text: String,
}

pub async fn start_slack_poster(slack_url: String, slack_rx: Receiver<String>) -> Result<(),String> {
    log::trace!("entering start_slack_poster");
    log::debug!("this is the slack url:{}",slack_url);

    for msg in slack_rx {
        let data = SlackPost { 
            text: msg.to_string(),
        };

        log::debug!("Trying to send this message to slack:{}",msg);
        let res = surf::post(&slack_url).body_json(&data);
        let res = match res {
            Err(e) => {
                log::error!("Could not construct slack url: {}",e.to_string());
                continue
            },
            Ok(p) => p,
        };
            
        let res = res.await;
        let res = match res {
            Err(e) => {
                log::error!("Could not connect to slack: {}",e.to_string());
                continue
            },
            Ok(p) => p,
        };

        if res.status() != StatusCode::OK {
            log::warn!("slack call returned: {}",res.status().to_string());
        }
    }
    Err("slack thread is exiting. This should never happen".to_string())
}
