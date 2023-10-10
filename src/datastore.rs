use std::{
    io::{Read, Seek, Write},
    sync::Arc,
    time::Duration,
};

use etcd_rs::{
    proto::etcdserverpb::WatchCreateRequest as ProtoWatchCreateRequest, KeyRange, WatchCanceler,
    WatchCreateRequest, WatchInbound, WatchOp, WatchStream,
};
use tokio::{sync::Mutex, time::sleep};

use crate::{
    errors::RustlinksError,
    state::{AppState, SerdeAppState},
    util::{self, NAMESPACE},
};

#[derive(Clone)]
pub struct Worker {
    pub state: actix_web::web::Data<AppState>,
    pub cancel: Arc<Mutex<Option<WatchCanceler>>>,
    pub sleep: Arc<Mutex<Option<()>>>,
}

// TODO:
// At some point, it might make sense to re-write this to be more generic
// to allow swapping the backend for a different storage implementation
// like Postgres, or MySQL, or Redis, or whatever

impl Worker {
    pub async fn start(&self) -> std::io::Result<()> {
        {
            let mut local_links_file = self.state.links_file.write().await;

            if let Some(links_file) = local_links_file.as_mut() {
                let mut buf: Vec<u8> = Vec::new();
                let result = links_file.read_to_end(buf.as_mut());

                if result.is_err() {
                    eprintln!("Failed to read bytes from links file: {:?}", result.err())
                } else {
                    let de: Result<SerdeAppState, serde_json::Error> = serde_json::from_slice(&buf);
                    match de {
                        Ok(disk_state) => {
                            let mut rustlinks = self.state.rustlinks.write().await;
                            rustlinks.extend(disk_state.rustlinks);
                            *self.state.revision.write().await = disk_state.revision;
                        }
                        Err(e) => {
                            eprintln!("Failed to deserialize links file: {:?}", e);
                        }
                    }
                }
            }
        }
        let mut stream: WatchStream;
        let mut backoff = 1;

        loop {
            // TODO: watch request since last seen revision
            let range = KeyRange::prefix(NAMESPACE);
            let request = WatchCreateRequest {
                proto: ProtoWatchCreateRequest {
                    key: range.key,
                    range_end: range.range_end,
                    start_revision: self.state.revision.read().await.clone(),
                    progress_notify: false,
                    filters: vec![],
                    prev_kv: false,
                    fragment: false,
                    watch_id: 0,
                },
            };
            let watch = self.state.etcd_client.watch(request).await;

            match watch {
                Ok((s, c)) => {
                    stream = s;
                    *self.cancel.lock().await = Some(c);
                    break;
                }
                Err(e) => {
                    eprint!("Failed to start etcd watch: {:?}, sleeping for {:?} seconds before retrying", e, backoff);
                    // Store the sleep future in the worker so that it can be cancelled
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

                                        let mut revision = self.state.revision.write().await;
                                        *revision = event.kv.mod_revision;
                                        Ok(())
                                    }
                                    Err(err) => Err(err),
                                }
                            }
                            etcd_rs::EventType::Delete => {
                                let mut rustlinks = self.state.rustlinks.write().await;
                                rustlinks.remove(&alias);

                                let mut revision = self.state.revision.write().await;
                                *revision = event.kv.mod_revision;
                                Ok(())
                            }
                        }
                    });
                    let results = futures::future::join_all(futs).await;
                    let acc: Result<Vec<()>, serde_json::Error> = results.into_iter().collect();

                    if acc.is_err() {
                        eprintln!("failed to update links: {:?}", acc.err());
                    }
                    let result = self.persist().await;

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

    pub async fn stop(&self) -> Result<(), RustlinksError> {
        // Cancel any pending sleeps
        if let Some(sleep) = self.sleep.lock().await.take() {
            drop(sleep);
        }

        if let Some(canceler) = self.cancel.lock().await.take() {
            canceler
                .cancel()
                .await
                .or_else(|e| Err(RustlinksError::EtcdError(e)))?
        } else {
            println!("nothing to cancel");
        }
        Ok(())
    }

    async fn persist(&self) -> Result<(), std::io::Error> {
        let serde_state = self.state.from().await;

        match self.state.links_file.write().await.as_ref() {
            Some(mut f) => {
                let string = serde_json::to_string(&serde_state).unwrap_or("{}".to_string());
                f.rewind()?;
                f.write_all(string.as_bytes())
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No links file to write to",
            )),
        }
    }

    async fn configure(&self) {}
}
