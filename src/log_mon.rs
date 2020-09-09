use std::{
    sync::{
        Arc,
        mpsc::{
            channel,
            Sender,
        }
    },
    process::{
        Command,
        Stdio,
    },
    io::{   
        Read,
    },
    thread::{
        spawn,
    },
    collections::{
        HashMap,
    }
};

use crate::configuration::Configuration;
//use crate::slack::post_to_slack;
//use surf::http::StatusCode;

struct LogFile {
    log_name: String,
    regex: String,
    tx: Sender<LogEntry>,
    reader: Box<dyn Read + Send>,
}

//pub fn log_mon_start(config: Arc<Configuration>) -> Result<FuturesUnordered<()>,String> {
pub async fn log_mon_start(config: Arc<Configuration>) -> Result<(),String> {
    let config_log_files = match &config.log_files {
        None => {
            log::error!("log_mon got passed a configuration where log_files has not been set");
            return Err("log_files is not set".to_string())
        },
        Some(d) => d
    };

    let (tx, rx) = channel();
    let mut log_files: HashMap<String,i32> = config_log_files.iter().map(|q| {
        let process = match Command::new("tail")
            .arg("-f")
            .arg(q.1.clone())
            .stdout(Stdio::piped())
            .spawn() {
                Err(why) => panic!("couldn't spawn tail: {}", why),
                Ok(process) => process,
        };
        let stdout = process.stdout.unwrap();
        let log_file = LogFile {
            log_name: q.0.clone(),
            regex: q.2.clone(),
            tx: tx.clone(),
            reader: Box::new(stdout),
        };
        start_tail_thread(log_file);
        (q.0.clone(), 0)
    }).collect();
    for log_name in rx {
        let log_name = match log_name {
            LogEntry::EOF => {
                log::error!("tail process ended.");
                continue;
            },
            LogEntry::LogName(log_name) => log_name
        };
        let num_errors = log_files.get_mut(&log_name);
        match  num_errors {
            None => {
                log::error!("could not find entry for: {}", log_name);
                continue;
            },
            Some(ne) => {
                *ne += 1;
                if *ne > 10 {
                    *ne = 0;
                    println!("got 10 messages for {}",log_name);
                }
                println!("got {} messages for {}",ne,log_name);
            }
        }
    }
        
    Ok(())
}

#[derive(Debug)]
enum LogEntry {
    LogName(String),
    EOF,
}

fn start_tail_thread(mut log_file: LogFile) {
    spawn(move || {
        let mut buf = Vec::new();
        let mut byte = [0u8];
        loop {
            match log_file.reader.read(&mut byte) {
                Ok(0) => {
                    let _ = log_file.tx.send(LogEntry::EOF); //ejw fix this
                    break;
                 },
                 Ok(_) => {
                    if byte[0] == 0x0A {
                        log_file.tx.send(match String::from_utf8(buf.clone()) {
                            Ok(_) => LogEntry::LogName(log_file.log_name.clone()),
                            Err(err) => {
                                log::error!("Error reading character. {}",err.to_string());
                                continue
                            }
                         })
                         .unwrap();
                         buf.clear()
                     } 
                     else {
                        buf.push(byte[0])
                     }
                 },
                 Err(error) => {
                    log::error!("Error reading character. {}",error.to_string());
                    continue
                 }
           }
        }
    });
}
