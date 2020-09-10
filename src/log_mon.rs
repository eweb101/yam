use regex::Regex;
use std::{
    thread::{
        JoinHandle,
    },
    time,
};
use std::{
    sync::{
        Arc,
        mpsc::{
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
};

use crate::configuration::Configuration;
//use crate::slack::post_to_slack;
//use surf::http::StatusCode;

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
        let process = match Command::new("docker")
            .arg("logs")
            .arg("-f")
            .arg(&self.container_name)
            .stderr(Stdio::piped())
            .stdout(Stdio::null())
            .spawn() {
                Err(why) => panic!("couldn't spawn docker logs: {}", why),
                Ok(process) => process,
        };
        Box::new(process.stderr.unwrap())
    }
        
    fn start_docker_thread(&mut self) {
        let mut reader = self.get_reader();

        let mut buf = Vec::new();
        let mut byte = [0u8];
        loop {
            match reader.read(&mut byte) {
                Ok(0) => {
                    log::error!("docker log process ended");
                    let fifteen_secs = time::Duration::from_secs(15);
                    std::thread::sleep(fifteen_secs);
                    reader = self.get_reader();
                    continue;
                 },
                 Ok(_) => {
                    if byte[0] == 0x0A {
                        let log_msg = String::from_utf8(buf.clone());
                        match log_msg {
                            Err(e) => {
                                log::error!("Error turning log message into string:{}",e.to_string());
                                continue;
                            },
                            Ok(lm) if self.regex.is_match(&lm) => {
                                log::debug!("Matched this entry: {}",lm.to_string());
                                self.handle_slack();
                            },
                            Ok(_) => {} //regex not matched
                        }
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
    }
}
