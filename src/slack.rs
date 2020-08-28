use serde::{Deserialize, Serialize};
use surf::http::status::StatusCode;
use crate::configuration::Configuration;

#[derive(Deserialize, Serialize)]
struct SlackPost {
    text: String,
}

pub(crate) async fn post_to_slack(config: &Configuration, message: &str) {
    log::trace!("entering post_to_slack");
    let data = SlackPost { 
        text: message.to_string(),
    };
    
    let slack_url = match &config.slack_url {
        None => {
            log::error!("post_to_slack received config with slack_url set to none");
            return;
        },
        Some(p) => p
    };

    let res = surf::post(slack_url).body_json(&data);
    let res = match res {
        Err(e) => {
            log::error!("Could not construct slack url: {}",e.to_string());
            return
        },
        Ok(p) => p,
    };
            
    let res = res.await;
    let res = match res {
        Err(e) => {
            log::error!("Could not connect to slack: {}",e.to_string());
            return
        },
        Ok(p) => p,
    };
    if res.status() != StatusCode::OK {
        log::warn!("slack call returned: {}",res.status().to_string());
    }
}
