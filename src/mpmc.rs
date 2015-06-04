// thanks "panicbit" on irc.mozilla.org #rust

use std::sync::{Mutex,Arc};
use std::sync::mpsc::{channel,Sender,Receiver,SendError};

#[derive(Clone)]
pub struct MultiSender<T: Send> {
    clients: Arc<Mutex<Vec<Sender<T>>>>
}

unsafe impl<T: Send> Sync for MultiSender<T> {}

impl<T: Send+Clone> MultiSender<T> {
    pub fn new() -> MultiSender<T> {
        MultiSender::<T> {
            clients: Arc::new(Mutex::new(Vec::new()))
        }
    }
    
    pub fn receiver(&mut self) -> Receiver<T> {
        let (cast_tx, cast_rx) = channel();
        let mut clients = self.clients.lock().unwrap();
        clients.push(cast_tx);
        cast_rx
    }

    pub fn send(&self, msg: T) {
        let clients = self.clients.lock().unwrap();
        for client in clients.iter() {
            client.send(msg.clone());
        }
    }

    pub fn send_one(&self, i: usize, msg: T) -> Result<(), SendError<T>> {
        let clients = self.clients.lock().unwrap();
        clients[i].send(msg.clone())
    }

    #[allow(dead_code)]
    pub fn disconnect_all(&mut self) {
        let mut clients = self.clients.lock().unwrap();
        *clients = Vec::new();
    }
}
