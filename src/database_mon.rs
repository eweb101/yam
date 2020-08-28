use sqlx::postgres::PgPoolOptions;
use async_std::task;
use std::{
        time::{
            Duration,
            Instant,
        },
        sync::Arc,
    };
use crate::configuration::Configuration;
use crate::slack::post_to_slack;

struct DbQuery {
    query_name: String,
    query_string: String,
    db_value: Option<i64>,
}

pub async fn database_mon_start(config: Arc<Configuration>) -> Result<(),String> {
    let database_url = match &config.database_url {
        None => {
            log::error!("database_mon got passed a configuration where database_url has not been set");
            return Err("database url is not set".to_string())
        },
        Some(d) => d
    };

    let config_db_queries = match &config.db_queries {
        None => {
            log::error!("database_mon got passed a configuration where db_queries has not been set");
            return Err("db_queries is not set".to_string())
        },
        Some(d) => d
    };

    // Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url).await;

    let pool = match pool {
        Err(e) => {
            return Err(format!("Could not create database pool:{}", e.to_string()))
        },
        Ok(p) => p,
    };
        
    //from here on never return
    let mut now = Instant::now(); 
    let mut do_slack = true;
    let mut db_queries: Vec<DbQuery> = config_db_queries.iter().map(|q| {
        DbQuery {
            query_name: q.0.clone(),
            query_string: q.1.clone(),
            db_value: None, //first time around the current value isn't set
        }
    }).collect();

    loop {
        if config.is_slack_configured() {
            do_slack = now.elapsed().as_secs() >= config.resend_status_minutes*60;
                if do_slack {
                    log::info!("database_mon is resending its status");
                    now = Instant::now()
                }
        }

        for db_query in &mut db_queries {
            let current_db_value: (i64,) = match sqlx::query_as(&db_query.query_string)
            .fetch_one(&pool) 
            .await {
                Err(e) => {
                    log::warn!("Could not fetch {} from database: {}",db_query.query_name,e.to_string());
                    continue; //goes to the next iteration of the for loop
                },
                Ok(p) => p
            };
            let current_db_value = current_db_value.0;
    
            log::info!("Current value in database for {}: {}",db_query.query_name,current_db_value);
    
            if config.is_slack_configured() {
                //only log to slack if this is the first time, the value has changed, or do_slack is true
                if db_query.db_value.is_none() || (current_db_value != db_query.db_value.unwrap()) || do_slack {
                    db_query.db_value = Some(current_db_value);
        
                    post_to_slack(&config, &format!("{}:{}", db_query.query_name, current_db_value)).await;
                }
            }
        }
        task::sleep(Duration::from_secs(config.sleep_seconds)).await;
    }
    //assert_eq!(res.status(), 200);
    //Ok(())
}
