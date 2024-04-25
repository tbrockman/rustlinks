use std::{sync::Arc, time::Duration};

use etcd_rs::{
    proto::etcdserverpb::WatchCreateRequest as ProtoWatchCreateRequest, KeyRange, KeyValueOp,
    RangeRequest, WatchCanceler, WatchCreateRequest, WatchInbound, WatchOp, WatchStream,
};
use tokio::{sync::Mutex, time::sleep};

use crate::{
    cli::RustlinksOpts,
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

            // first, get the last revision we know about.
            // a) if it's zero, retrieve all keys from etcd, and start a watch from the latest revision.
            // b) if it's not zero, start a watch from the last revision we know about if the watch fails due to the revision being
            // compacted, drop our database and start over
            let last_revision = self.state.rustlink_store.get_revision().unwrap_or(0);
            let mut range_request = RangeRequest::new(range.clone());
            range_request.proto.min_mod_revision = last_revision + 1;
            let range_response = self.state.etcd_client.get(range_request).await;

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
                    match e {
                        etcd_rs::Error::IOError(_) => todo!(),
                        etcd_rs::Error::InvalidURI(_) => todo!(),
                        etcd_rs::Error::Transport(_) => todo!(),
                        etcd_rs::Error::Response(_) => todo!(),
                        etcd_rs::Error::ChannelClosed => todo!(),
                        etcd_rs::Error::CreateWatch => todo!(),
                        etcd_rs::Error::WatchEvent(_) => todo!(),
                        etcd_rs::Error::KeepAliveLease => todo!(),
                        etcd_rs::Error::WatchChannelSend(_) => todo!(),
                        etcd_rs::Error::WatchEventExhausted => todo!(),
                    }

                    eprint!("Failed to start etcd watch: {:?}, sleeping for {:?} seconds before retrying", e, backoff);
                    // Store the sleep future in the worker so that it can be cancelled
                    *self.sleep.lock().await = Some(sleep(Duration::from_secs(backoff)).await);
                    backoff = std::cmp::min(backoff * 2, 300);
                }
            }
        }

        loop {
            println!("polling for etcd inbound events...");
            match stream.inbound().await {
                WatchInbound::Ready(resp) => {
                    println!("received event: {:?}", resp);

                    let results = resp.events.into_iter().map(|event| {
                        if let Some(alias) = util::key_to_alias(event.kv.key_str()) {
                            match event.event_type {
                                etcd_rs::EventType::Put => {
                                    let value = event.kv.value;
                                    let rustlink = serde_json::from_slice::<Rustlink>(&value)?;
                                    self.state.rustlink_store.set_rustlink(&alias, &rustlink)?;
                                }
                                etcd_rs::EventType::Delete => {
                                    // let mut revision =
                                    // self.state.revision.write().await;
                                    // *revision = event.kv.mod_revision;
                                    self.state.rustlink_store.delete_rustlink(&alias)?;
                                }
                            }
                            Ok(event.kv.mod_revision)
                        } else {
                            Ok(0)
                        }
                    });

                    match results
                        .into_iter()
                        .collect::<Result<Vec<i64>, RustlinksError>>()
                    {
                        Ok(revisions) => {
                            let max_revision = revisions.into_iter().max().unwrap_or(0);
                            if let Err(e) = self.state.rustlink_store.set_revision(max_revision) {
                                eprintln!("failed to set revision: {:?}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("failed to process events: {:?}", e);
                        }
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
