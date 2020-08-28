use async_std::fs;
use async_std;
use toml::{Value};

#[derive(Clone)]
pub struct Configuration {
    pub sleep_seconds: u64,
    pub resend_status_minutes: u64,
    pub slack_url: Option<String>,
    pub database_url: Option<String>,
    pub monitor_urls: Option<Vec<String>>,
    pub db_queries: Option<Vec<(String,String)>>,
}

impl Configuration {
    pub async fn from_filename(filename: &str) -> Result<Configuration,String>  {
        let s = fs::read_to_string(filename).await;
        let s = match s { 
            Err(e) => return Err(format!("Could not open config file: {}",e.to_string())),
            Ok(s) => s
        };
        Configuration::from_string(&s).await
    }

    pub async fn from_string(s: &str) -> Result<Configuration, String> {
        let config: Value = match toml::from_str(s) {
            Err(e) => return Err(format!("Could  not parse config file:{}",e)),
            Ok(c) => c
        };

        let database_url = match config.get("DATABASE_URL") {
            None => None,
            Some(s) => {
                match s.as_str() {
                    None => None,
                    Some(s) => Some(s.to_string())
                }
            }
        };

        let slack_url = match config.get("SLACK_URL") {
            None => None,
            Some(s) => {
                match s.as_str() {
                    None => None,
                    Some(s) => Some(s.to_string())
                }
            }
        };

        
        let resend_status_minutes = match config.get("RESEND_MINUTES") {
            None => return Err("RESEND_MINUTES is not set in config file".to_string()),
            Some(s) => s
        };

        let resend_status_minutes = match resend_status_minutes.as_integer() {
            None => return Err("RESEND_MINUTES is not an integer".to_string()),
            Some(s) => s
        };
        let sleep_seconds = match config.get("SLEEP_SECONDS") {
            None => return Err("SLEEP_SECONDS is not set in config file".to_string()),
            Some(s) => s
        };

        let sleep_seconds = match sleep_seconds.as_integer() {
            None => return Err("SLEEP_SECONDS is not an integer".to_string()),
            Some(s) => s
        };
            
        let monitor_urls = match config.get("MONITOR_URLS") {
            None => {
                log::info!("MONITOR_URLS not found. Web Monitoring not configured");
                None
            },
            Some(yaml) => { //monitor_urls is set in the file but we don't know if it's an array yet
                match yaml.as_array() {
                    None => { 
                        log::info!("MONITOR_URLS is not an array. Web Monitoring is not configured");
                        None
                    },
                    Some(yaml_vec) => { //this is a vector of yaml objects. we have to convert to strings
                        let ret_vec = yaml_vec.iter().filter_map(|yam| {
                            match yam.as_str() {
                                None => {
                                    log::warn!("Invalid monitor url {:?}", yam);
                                    None
                                }
                                Some(y_str) => Some(y_str.to_string())
                            }
                        }).collect();
                        Some(ret_vec)
                    }
                }
            }
        };

        let db_queries = match config.get("DB_QUERIES") {
            None => {
                log::info!("DB_QUERIES not found. Database Monitoring not configured");
                None
            },
            Some(yaml) => { //db_queries is set in the file but we don't know if it's an array yet
                match yaml.as_array() {
                    None => return Err(format!("DB_QUERIES is not an array. Reading configuration file failed")),
                    Some(toml_vec) => { //this is a vector of toml Values. we have to convert to strings
                        let ret_vec: Result<Vec<_>, _> = toml_vec.iter().map(|yam| {
                            match yam.as_array() {
                                None => Err(format!("Could not read db_query:{},",yam)),
                                Some(v) => Ok((v[0].as_str().unwrap().to_string(),v[1].as_str().unwrap().to_string()))
                            }
                        }).collect();
                        match ret_vec {
                            Ok(rv) => Some(rv),
                            Err(e) => return Err(e)
                        }
                    }
                }
            }
        };

        let config = Configuration {
            sleep_seconds: sleep_seconds as u64,
            resend_status_minutes: resend_status_minutes as u64,
            slack_url: slack_url,
            database_url: database_url,
            monitor_urls: monitor_urls,
            db_queries: db_queries,
        };

        log::info!("Monitoring will be performed every {} seconds.",sleep_seconds);
        match config.is_db_configured() {
            true => log::info!("Database monitoring is configured"),
            false => log::info!("Database monitoring is not configured")
        }

        match config.is_web_configured() {
            true => log::info!("URL monitoring is configured"),
            false => log::info!("URL monitoring is not configured")
        }
        match config.is_slack_configured() {
            true => {
                log::info!("Slack is configured");
                log::info!("Results will be sent to slack when an error occurs and every {} minutes",resend_status_minutes);
            },
            false => log::info!("Slack is not configured")
        }

        Ok(config)
    }

    pub(crate) fn is_slack_configured(&self) -> bool {
        self.slack_url.is_some()
    }

    fn is_db_configured(&self) -> bool {
        self.database_url.is_some() && self.db_queries.is_some()
    }

    fn is_web_configured(&self) -> bool {
        self.monitor_urls.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::Configuration;
    #[async_std::test]
    async fn test_config1() {
        let t1 = r#"
DATABASE_URL = "postgres://"
SLEEP_SECONDS = 300
SLACK_URL = "https://hooks.slack.com/services/"
RESEND_MINUTES = 60
MONITOR_URLS = [
    "https://www.example.com/heartbeat",
    "https://www.example.com/heartbeat2"
]
DB_QUERIES = [
    ["users","select * from users"],
    ["cars","select * from cars"]
]
        "#;
        let v = Configuration::from_string(t1).await;
        assert!(v.is_ok());
        let v = v.unwrap();
        let mu = v.monitor_urls.unwrap();
        let dq = v.db_queries.unwrap();
        assert_eq!(v.sleep_seconds,300);
        assert_eq!(v.resend_status_minutes,60);
        assert_eq!(mu.len(),2);
        assert_eq!(mu[0],"https://www.example.com/heartbeat");
        assert_eq!(mu[1],"https://www.example.com/heartbeat2");
        assert_eq!(dq[0].1,"select * from users");
    }

    #[async_std::test]
    async fn config_without_sleep_seconds() {
        let t1 = r#"
DATABASE_URL = "postgres://"
SLACK_URL = "https://hooks.slack.com/services/"
RESEND_MINUTES = 60
MONITOR_URLS = [
    "https://www.example.com/heartbeat",
    "https://www.example.com/heartbeat2"
]
        "#;
        let v = Configuration::from_string(t1).await;
        match v {
            Err(e) => assert_eq!(e,"SLEEP_SECONDS is not set in config file"),
            Ok(_c) => panic!("SLEEP_SECONDS isn't set so should have failed")
        }
    }

    #[async_std::test]
    async fn config_without_resend_minutes() {
        let t1 = r#"
DATABASE_URL = "postgres://"
SLEEP_SECONDS = 300
SLACK_URL = "https://hooks.slack.com/services/"
MONITOR_URLS = [
    "https://www.example.com/heartbeat",
    "https://www.example.com/heartbeat2"
]
        "#;
        let v = Configuration::from_string(t1).await;
        match v {
            Err(e) => assert_eq!(e,"RESEND_MINUTES is not set in config file"),
            Ok(_c) => panic!("RESEND_MINUTES isn't set so should have failed")
        }
    }
}

