use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use etcd_rs::{Error, KeyRange, KeyValueOp, WatchInbound, WatchOp};

use crate::{rustlink::Rustlink, state::AppState, util, RustlinkAlias};

#[derive(Clone)]
pub struct Worker {
    pub state: actix_web::web::Data<AppState>,
    pub cancel: Arc<Mutex<Option<etcd_rs::WatchCanceler>>>,
}

const NAMESPACE: &str = "rustlinks";

// TODO:
// At some point, it might make sense to re-write this to be more generic
// We can say that the Worker struct is generic over a type that implements the
// `Datastore` trait, which would have a `start` method, a `stop` method, a
// `configure` method, and most importantly a `watch` method (which returns a
// stream of upsert/delete events) This would allow us to swap out the etcd
// implementation for a different implementation, like a Postgres
// implementation, a Redis implementation, etc.

impl Worker {
    pub async fn start(&self) -> std::io::Result<()> {
        let remote_links = self.get_links().await;

        if remote_links.is_ok() {
            if let Ok(mut local_links) = self.state.rustlinks.write() {
                local_links.extend(remote_links.unwrap())
            }
        } else {
            eprintln!(
                "Failed to retrieve any remote links to initialize with: {:?}",
                remote_links.err()
            );
        }

        let (mut stream, cancel) = self
            .state
            .client
            .watch(KeyRange::prefix(NAMESPACE))
            .await
            .expect("watch by prefix");
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
                                let value = event.kv.value;

                                if let Ok(rustlink) = serde_json::from_slice(&value) && let Ok(mut rustlinks) = self.state.rustlinks.write() {
                                    rustlinks.insert(alias, rustlink);
                                } else {
                                    eprintln!("Failed to deserialize and insert Rustlink: {:?}", value);
                                }
                            },
                            etcd_rs::EventType::Delete => {
                                if let Ok(mut rustlinks) = self.state.rustlinks.write() {
                                    rustlinks.remove(&alias);
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

    pub async fn stop(&self) -> Result<(), etcd_rs::Error> {
        let mut cancel = self.cancel.lock().expect("mutex lock");
        if let Some(canceler) = cancel.take() {
            canceler.cancel().await
        } else {
            println!("nothing to cancel");
            Ok(())
        }
    }

    async fn get_links(&self) -> Result<HashMap<RustlinkAlias, Rustlink>, Error> {
        let key_range = KeyRange::prefix(NAMESPACE);
        // TODO: only get links since last seen revision
        let proto = etcd_rs::proto::etcdserverpb::RangeRequest {
            key: key_range.key,
            range_end: key_range.range_end,
            limit: 0,
            revision: 0,
            sort_order: 0,
            sort_target: 0,
            serializable: true,
            keys_only: false,
            count_only: false,
            min_mod_revision: 0,
            max_mod_revision: 0,
            min_create_revision: 0,
            max_create_revision: 0,
        };
        let request = etcd_rs::RangeRequest { proto: proto };
        let result = self.state.client.get(request).await;

        match result {
            Ok(response) => {
                let mut rustlinks: HashMap<RustlinkAlias, Rustlink> = HashMap::new();
                response.kvs.into_iter().for_each(|kv| {
                    let mut split = kv.key_str().split('/');
                    split.next();
                    let alias = split.remainder().unwrap().to_string();
                    let value = kv.value.clone();
                    let rustlink: Rustlink = serde_json::from_slice(&value).unwrap();
                    rustlinks.insert(alias, rustlink);
                });
                Ok(rustlinks)
            }
            Err(e) => Err(e),
        }
    }

    async fn configure(&self) {}
}
