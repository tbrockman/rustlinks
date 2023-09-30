use std::sync::{Arc, Mutex};

use etcd_rs::{KeyRange, WatchOp, WatchInbound};

use crate::AppState;

pub struct Worker {
    pub state: actix_web::web::Data<AppState>,
    pub client: etcd_rs::Client,
    pub cancel: Arc<Mutex<Option<etcd_rs::WatchCanceler>>>
}

impl Worker {
    pub async fn start(&self) -> std::io::Result<()> {
        let (mut stream, cancel) = self.client.watch(KeyRange::prefix("rustlinks")).await.expect("watch by prefix");
        {
            *self.cancel.lock().expect("mutex lock") = Some(cancel);
        }
        loop {
            match stream.inbound().await {
                WatchInbound::Ready(resp) => {
                    println!("receive event: {:?}", resp);
                }
                WatchInbound::Interrupted(e) => {
                    eprintln!("encounter error: {:?}", e);
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
}