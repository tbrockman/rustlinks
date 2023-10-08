use std::{collections::HashMap, io::Write, sync::Arc, time::Duration};

use etcd_rs::{Error, KeyRange, KeyValueOp, WatchCanceler, WatchInbound, WatchOp, WatchStream};
use tokio::{sync::Mutex, time::sleep};

use crate::{rustlink::Rustlink, state::AppState, util, RustlinkAlias};

#[derive(Clone)]
pub struct Worker {
    pub state: actix_web::web::Data<AppState>,
    pub cancel: Arc<Mutex<Option<WatchCanceler>>>,
    pub sleep: Arc<Mutex<Option<()>>>,
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
            let mut local_links = self.state.rustlinks.write().await;
            local_links.extend(remote_links.unwrap());
            self.persist(local_links.clone())
                .await
                .expect("Failed persisting links");
        } else {
            eprintln!(
                "Failed to retrieve any remote links to initialize with: {:?}",
                remote_links.err()
            );
        }

        // TODO: fix this
        let mut stream: WatchStream;
        let mut backoff = 1;

        loop {
            let watch = self.state.client.watch(KeyRange::prefix(NAMESPACE)).await;

            match watch {
                Ok((s, c)) => {
                    stream = s;
                    *self.cancel.lock().await = Some(c);
                    break;
                }
                Err(e) => {
                    eprint!("Failed to start etcd watch: {:?}, sleeping for {:?} seconds before retrying", e, backoff);
                    *self.sleep.lock().await = Some(sleep(Duration::from_secs(backoff)).await);
                    backoff = std::cmp::min(backoff * 2, 60);
                }
            }
        }

        loop {
            match stream.inbound().await {
                WatchInbound::Ready(resp) => {
                    println!("receive event: {:?}", resp);

                    let futs = resp.events.into_iter().map(|event| async move {
                        let alias = util::key_to_alias(event.kv.key_str());

                        match event.event_type {
                            etcd_rs::EventType::Put => {
                                let value = event.kv.value;
                                match serde_json::from_slice(&value) {
                                    Ok(rustlink) => {
                                        let mut rustlinks = self.state.rustlinks.write().await;
                                        rustlinks.insert(alias, rustlink);
                                        Ok(())
                                    }
                                    Err(err) => Err(err),
                                }
                            }
                            etcd_rs::EventType::Delete => {
                                let mut rustlinks = self.state.rustlinks.write().await;
                                rustlinks.remove(&alias);
                                Ok(())
                            }
                        }
                    });
                    let results = futures::future::join_all(futs).await;
                    let acc: Result<Vec<()>, serde_json::Error> = results.into_iter().collect();

                    if acc.is_err() {
                        eprintln!("failed to update links: {:?}", acc.err());
                    }

                    let rustlinks = self.state.rustlinks.write().await;
                    let result = self.persist(rustlinks.clone()).await;

                    if result.is_err() {
                        eprintln!("Failed to persist links to disk: {:?}", result.err());
                    }
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
        let mut cancel = self.cancel.lock().await;
        if let Some(canceler) = cancel.take() {
            canceler.cancel().await.unwrap()
        } else {
            println!("nothing to cancel");
        }

        if let Some(sleep) = self.sleep.lock().await.take() {
            drop(sleep);
        }

        Ok(())
    }

    async fn persist(
        &self,
        rustlinks: HashMap<RustlinkAlias, Rustlink>,
    ) -> Result<(), std::io::Error> {
        match self.state.links_file.write().await.take() {
            Some(mut f) => {
                let serde_state =
                    serde_json::to_string(&rustlinks.clone()).unwrap_or("{}".to_string());
                f.write_all(serde_state.as_bytes())
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No links file to write to",
            )),
        }
    }

    async fn get_links(&self) -> Result<HashMap<RustlinkAlias, Rustlink>, Error> {
        let key_range = KeyRange::prefix(NAMESPACE);
        // TODO: only get links since last seen revision
        // order by last modified descending
        // limit to `n` links at most
        // update links stored in memory
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
