use regex::Regex;
use dockworker::{ContainerLogOptions, Docker};
use std::io::prelude::*;
use std::{
    sync::{
        Arc,
        mpsc::{
            Sender,
        }
    },
    thread::{
        spawn,
        JoinHandle,
    },
};

use crate::configuration::Configuration;
//use crate::slack::post_to_slack;
//use surf::Response;

struct DockerLogReader {
    log_name: String,
    container_name: String,
    regex: Regex,
    num_hits: i32,
    slack_tx: Sender<String>,
}

pub fn log_mon_start(config: Arc<Configuration>, slack_tx: Sender<String>) -> Result<Vec<JoinHandle<()>>,String> {
    let config_log_files = match &config.log_files {
        None => {
            log::error!("log_mon got passed a configuration where log_files has not been set");
            return Err("log_files is not set".to_string())
        },
        Some(d) => d
    };

    let mut handles = Vec::new();
    for log in config_log_files {
        let tx = slack_tx.clone();
        let regex = Regex::new(&log.2);
        let regex = match regex {
            Err(e) => {
                log::error!("Could not create regex from string:{}", log.2.clone());
                log::error!("regex error is:{}",e.to_string());
                return Err("Could not create regex".to_string());
            },
            Ok(r) => r
        };
        let mut lf = DockerLogReader {
            log_name: log.0.clone(),
            container_name: log.1.clone(),
            regex: regex,
            num_hits: 0,
            slack_tx: tx,    
        };

        handles.push(spawn(move || lf.start_docker_thread()))
    }
    Ok(handles)
}

impl DockerLogReader {
    fn handle_slack(&mut self) {
        self.num_hits += 1;
        if let Err(e) = self.slack_tx.send(format!("{} received {} hits.",self.log_name, self.num_hits)) {
            log::error!("DockerLogReader could not send to slack:{}",e.to_string());
        }
    }

    fn get_reader(&self) -> Box<dyn Read> {
        let docker = Docker::connect_with_defaults().unwrap();
        let log_options = ContainerLogOptions {
                stdout: true,
                stderr: true,
                follow: true,
                tail:   Some(100),
                ..ContainerLogOptions::default()
        };

        log::info!("Starting log watcher for:{}",&self.container_name);
        let res = docker.log_container(&self.container_name, &log_options).unwrap();
        //let mut line_reader = BufReader::new(res);
        Box::new(res)

        //let r = reader.write_all(b"http:/v1.40/containers/json");
        //let r = reader.write_all(b"GET /v1.40/containers/json HTTP/1.1\r\nHost: 1.40\r\nUser-Agent: curl/7.68.9\r\nnAccept: gzip, deflate\r\n\r\n");
       /* let r = reader.write_all(b"GET /v1.40/containers/mysql/logs?stderr=true HTTP/1.1\r\nHost: v1.40\r\n\r\n\r\n");
        if let Err(e) = r {
            println!("Error writing to docker sock:{}",e.to_string());
            panic!("error");
        }
        let mut response = String::new();*/
    }

    fn read_bytes(&mut reader: Box<dyn Read>,num_bytes_to_read: i32) -> Result<Vec<u8>,()> {
        let mut buf = Vec::new();
        let mut byte = [0u8];

        for _ in 0..num_bytes_to_read {
            match reader.read(&mut byte) {
                /*Ok(0) => {
                    log::error!("docker log process ended");
                    let fifteen_secs = time::Duration::from_secs(15);
                    std::thread::sleep(fifteen_secs);
                    reader = self.get_reader();
                    break;
                },*/
                Ok(_) => {
                    println!("byte value:{}",byte[0]);
                    buf.push(byte[0])
                },
                Err(e) => {
                    log::error!("Error reading byte:{}",e.to_string());
                    return Err(())
                }
            }
        }
        Ok(buf)
    }

    fn start_docker_thread(&mut self) {
        let reader = self.get_reader();

        loop {
            
            let header = DockerLogReader::read_bytes(&mut reader,8);
            let header = match header {
                Err(_) => {
                    log::error!("failure reading header. restart?");
                     continue;
                },
                Ok(s) => s
            };
            let log_entry_size = header[7] as i32;

            log::info!("number of data bytes to read:{}",log_entry_size);

            let log_entry = DockerLogReader::read_bytes(reader,log_entry_size);
            let log_entry = match log_entry {
                Err(_) => {
                    log::error!("failure reading. log entry?");
                    continue;
                },
                Ok(s) => s
            };
            let log_msg = String::from_utf8(log_entry.clone());
            match log_msg {
                Err(e) => {
                    log::error!("Error turning log message into string:{}",e.to_string());
                    continue;
                },
                Ok(lm) if self.regex.is_match(&lm) => {
                    log::debug!("Matched this entry: {}",lm.to_string());
                    self.handle_slack();
                },
                Ok(lm) => log::info!("regex not matched:{}",lm) //regex not matched
            }
        }
    }
}
