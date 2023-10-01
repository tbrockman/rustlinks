use std::{sync::{Arc, Mutex}, collections::HashMap};

use etcd_rs::{KeyRange, WatchOp, WatchInbound, KeyValueOp, Error};

use crate::{AppState, golink::Golink, util, GolinkAlias};

#[derive(Clone)]
pub struct Worker {
    pub state: actix_web::web::Data<AppState>,
    pub client: etcd_rs::Client,
    pub cancel: Arc<Mutex<Option<etcd_rs::WatchCanceler>>>
}

const NAMESPACE: &str = "rustlinks";

impl Worker {
    pub async fn start(&self) -> std::io::Result<()> {
        let remote_golinks = self.get_links().await;

        if remote_golinks.is_ok() {
            if let Ok(mut local_golinks) = self.state.golinks.write() {
                local_golinks.extend(remote_golinks.unwrap())
            }
        }
        else {
            eprintln!("Failed to retrieve any remote Golinks to initialize with: {:?}", remote_golinks.err());
        }

        let (mut stream, cancel) = self.client.watch(KeyRange::prefix(NAMESPACE)).await.expect("watch by prefix");
        {
            *self.cancel.lock().expect("mutex lock") = Some(cancel);
        }
        loop {
            match stream.inbound().await {
                WatchInbound::Ready(resp) => {
                    println!("receive event: {:?}", resp);
                    
                    resp.events.into_iter().for_each(|event| {         
                        let alias = util::key_to_alias(event.kv.key_str());

                        match event.event_type {
                            etcd_rs::EventType::Put => {
                                let value = event.kv.value.clone();
                                
                                if let Ok(golink) = serde_json::from_slice(&value) && let Ok(mut golinks) = self.state.golinks.write() {
                                    golinks.insert(alias, golink);
                                } else {
                                    eprintln!("Failed to deserialize and insert Golink: {:?}", value);
                                }
                            },
                            etcd_rs::EventType::Delete => {
                                if let Ok(mut golinks) = self.state.golinks.write() {
                                    golinks.remove(&alias);
                                }
                            },
                        }
                    })
                }
                WatchInbound::Interrupted(e) => {
                    match e {
                        etcd_rs::Error::WatchEventExhausted => {
                            println!("watch event exhausted");
                        }
                        etcd_rs::Error::IOError(_) => todo!(),
                        etcd_rs::Error::Transport(_) => todo!(),
                        etcd_rs::Error::ChannelClosed => todo!(),
                        _ => todo!(),
                    }
                    eprintln!("encounter error: {:?}", e);
                    break;
                }
                WatchInbound::Closed => {
                    println!("watch stream closed");
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), etcd_rs::Error>{
        let mut cancel = self.cancel.lock().expect("mutex lock");
        if let Some(canceler) = cancel.take() {
            canceler.cancel().await
        }
        else {
            println!("nothing to cancel");
            Ok(())
        }
    }
    
    async fn get_links(&self) -> Result<HashMap<GolinkAlias, Golink>, Error> {
        let result = self.client.get_by_prefix(NAMESPACE).await;

        match result {
            Ok(response) => {
                let mut golinks: HashMap<GolinkAlias, Golink> = HashMap::new();
                response.kvs.into_iter().for_each(|kv| {
                    let mut split = kv.key_str().split("/");
                    split.next();
                    let alias = split.remainder().unwrap().to_string();
                    let value = kv.value.clone();
                    let golink: Golink = serde_json::from_slice(&value).unwrap();
                    golinks.insert(alias, golink);
               });
               Ok(golinks)
            },
            Err(e) => Err(e)
        }
    }
}