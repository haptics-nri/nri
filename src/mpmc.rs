//! Multi-producer/multi-consumer channels
//!
//! Written by "panicbit" on irc.mozilla.org #rust, slightly modified by me

use std::sync::{Mutex,Arc};
use std::sync::mpsc::{channel,Sender,Receiver,SendError};

/// Multi-producer, multi-consumer manager. Hands out channels and holds onto the sending ends, and
/// has methods for sending to one or all of them.
#[derive(Clone)]
pub struct MultiSender<T: Send> {
    /// All open client channels
    clients: Arc<Mutex<Vec<Sender<T>>>>
}

unsafe impl<T: Send> Sync for MultiSender<T> {}

impl<T: Send+Clone> MultiSender<T> {
    /// Create a new empty MultiSender with no clients
    pub fn new() -> MultiSender<T> {
        MultiSender::<T> {
            clients: Arc::new(Mutex::new(Vec::new()))
        }
    }
    
    /// Creates a channel, keeps the sending end and returns the receiving end
    ///
    /// The receiving ends are kept in order, so the index is predictable in case you want to call
    /// send_one()
    pub fn receiver(&mut self) -> Receiver<T> {
        let (cast_tx, cast_rx) = channel();
        let mut clients = self.clients.lock().unwrap();
        clients.push(cast_tx);
        cast_rx
    }

    /// Send a message to all clients
    ///
    /// Does not check for disconnected clients (changed from panicbit's version), because the
    /// indices need to be predictable so that send_one() is usable
    pub fn send(&self, msg: T) {
        let clients = self.clients.lock().unwrap();
        for client in clients.iter() {
            client.send(msg.clone());
        }
    }

    /// Send a message to one client
    ///
    /// Client identified by index (TODO some better way)
    pub fn send_one(&self, i: usize, msg: T) -> Result<(), SendError<T>> {
        let clients = self.clients.lock().unwrap();
        clients[i].send(msg.clone())
    }

    /// Forget about all the clients
    #[allow(dead_code)]
    pub fn disconnect_all(&mut self) {
        let mut clients = self.clients.lock().unwrap();
        *clients = Vec::new();
    }
}
