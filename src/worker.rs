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
    rustlink::Rustlink,
    state::AppState,
    util::{self, NAMESPACE},
};

#[derive(Clone)]
pub struct Worker {
    pub state: actix_web::web::Data<AppState>,
    pub cancel: Arc<Mutex<Option<WatchCanceler>>>,
    pub sleep: Arc<Mutex<Option<()>>>,
}

// TODO:
// At some point, it might make sense to re-write this to be generic over the
// backend to allow swapping the storage layer for a different implementation
// like Postgres, or MySQL, or Redis, or whatever

impl Worker {
    pub async fn start(&self) -> std::io::Result<()> {
        let mut stream: WatchStream;
        let mut backoff = 1;

        loop {
            let range = KeyRange::prefix(NAMESPACE);
            let request = WatchCreateRequest {
                proto: ProtoWatchCreateRequest {
                    key: range.key,
                    range_end: range.range_end,
                    start_revision: 0,
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
            println!("polling for etcd inbound events...");
            match stream.inbound().await {
                WatchInbound::Ready(resp) => {
                    println!("received event: {:?}", resp);

                    let futs = resp.events.into_iter().map(|event| async move {
                        if let Some(alias) = util::key_to_alias(event.kv.key_str()) {
                            match event.event_type {
                                etcd_rs::EventType::Put => {
                                    let value = event.kv.value;
                                    let rustlink = serde_json::from_slice::<Rustlink>(&value)?;
                                    self.state.rustlink_store.set(&alias, &rustlink)
                                }
                                etcd_rs::EventType::Delete => {
                                    // let mut revision =
                                    // self.state.revision.write().await;
                                    // *revision = event.kv.mod_revision;
                                    self.state.rustlink_store.delete(&alias)
                                }
                            }
                        } else {
                            Ok(())
                        }
                    });
                    let results = futures::future::join_all(futs).await;
                    let acc: Result<Vec<()>, RustlinksError> = results.into_iter().collect();

                    if acc.is_err() {
                        eprintln!("failed to process events: {:?}", acc.err());
                    }
                }
                WatchInbound::Interrupted(e) => {
                    match e {
                        etcd_rs::Error::WatchEventExhausted => {
                            println!("watch event exhausted");
                        }
                        etcd_rs::Error::IOError(_) => todo!(),
                        etcd_rs::Error::Transport(_) => todo!(),
                        etcd_rs::Error::ChannelClosed => todo!(), // TODO: handle issues on watch
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

    async fn configure(&self) {}
}
