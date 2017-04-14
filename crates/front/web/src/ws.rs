use std::collections::HashMap;
use std::net::TcpStream;
use std::result::Result as StdResult;
use std::sync::{mpsc, Mutex};
use std::{thread, str};
use comms::CmdFrom;
use super::{config, flow};
pub use websocket::Message;
use websocket::message::Type as MsgType;
use websocket::client::{self, ClientBuilder};
use websocket::server::Server;
use uuid::Uuid;

use super::{Result, Error};

lazy_static! {
    pub static ref SERVER_ID: Uuid = Uuid::new_v4();
    pub static ref RPC_SENDERS: Mutex<HashMap<usize, mpsc::Sender<Option<String>>>> = Mutex::new(HashMap::new());
    pub static ref WS_SENDERS:  Mutex<Vec<client::Writer<TcpStream>>> = Mutex::new(Vec::new());
}

#[derive(Clone)]
pub struct Comms {
    wsid: usize
}

impl Comms {
    pub fn new(wsid: usize) -> Comms {
        Comms {
            wsid: wsid
        }
    }
}

impl flow::Comms for Comms {
    type Error = Error;

    fn print(&self, msg: String) -> Result<()> {
        println!("{}", msg);
        Ok(())
    }

    fn send(&self, msg: String) -> Result<()> {
        let mut locked_senders = WS_SENDERS.lock()?;
        if ::std::env::var("NRI_WS_FAIL").ok().map_or(false, |s| s == "1") { // FIXME remove this debugging gizmo
            Err("NoDataAvailable".into())
        } else {
            Ok(locked_senders[self.wsid].send_message(&Message::text(msg))?)
        }
    }

    fn rpc<T, F: Fn(String) -> StdResult<T, String>>(&self, prompt: String, validator: F) -> Result<Option<T>> {
        let go = |prompt: &str| -> Result<Option<String>> {
            let (tx, rx) = mpsc::channel();
            println!("Waiting on RPC from WSID {}", self.wsid);
            {
                let mut locked_senders = WS_SENDERS.lock()?;
                let mut locked_rpcs = RPC_SENDERS.lock()?;
                locked_rpcs.insert(self.wsid, tx);
                locked_senders[self.wsid].send_message(&Message::text(prompt.to_owned()))?;
            }
            Ok(rx.recv()?)
        };

        let mut maybe_answer = go(&prompt)?;
        loop {
            if let Some(answer) = maybe_answer {
                match validator(answer) {
                    Ok(ret) => return Ok(Some(ret)),
                    Err(admonish) => {
                        maybe_answer = go(&format!("{} {}", admonish, prompt))?;
                    }
                }
            } else {
                return Ok(None);
            }
        }
    }
}

pub fn spawn(ctx: mpsc::Sender<CmdFrom>, wsrx: mpsc::Receiver<Message<'static>>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let ws = Server::bind(("0.0.0.0", config::WS_PORT)).unwrap();

        let mut relays = Vec::new();

        let marshal = thread::spawn(move || {
            // relay messages from above to all WS threads
            while let Ok(msg) = wsrx.recv() {
                WS_SENDERS.lock().unwrap().iter_mut().map(|ref mut s| {
                    s.send_message(&Message::clone(&msg)).unwrap(); // FIXME why is the typechecker so confused here
                }).count();
            }

            println!("web: shutting down websocket servers");
            // kill all WS threads now
            WS_SENDERS.lock().unwrap().iter_mut().map(|ref mut s| {
                s.send_message(&Message::close()).unwrap();
            }).count();
            println!("web: finished shutting down websocket servers");
        });

        'listener: for request in ws.filter_map(StdResult::ok) {
            if request.protocols().contains(&"ouroboros".to_owned()) {
                // Shutdown signal
                
                println!("Websocket listener received shutdown signal");
                request.reject().unwrap();
                break 'listener;
            } else if request.protocols().contains(&"rust-websocket".to_owned()) {
                // We have a protocol we want to use

                let mut client = request.use_protocol("rust-websocket").accept().unwrap(); // Send the response

                let ip = client.peer_addr().unwrap();
                println!("Websocket connection from {}", ip);

                let mut locked_senders = WS_SENDERS.lock().unwrap();
                let wsid = locked_senders.len();

                client.send_message(&Message::text(format!("hello {}_{}", *SERVER_ID, wsid))).unwrap();

                let (mut receiver, sender) = client.split().unwrap();
                locked_senders.push(sender);
                let cctx = ctx.clone();
                relays.push(thread::spawn(move || {
                    for message in receiver.incoming_messages() {
                        let message = message.unwrap();

                        match message {
                            Message { opcode: MsgType::Close, .. } => {
                                println!("Websocket client {} disconnected", ip);
                                return;
                            },
                            Message { opcode: MsgType::Text, payload: text, .. } => {
                                println!("Received WS text {:?}", str::from_utf8(&text).unwrap_or(&*format!("{:?}", text)));
                                if text.starts_with(b"RPC") {
                                    let text = str::from_utf8(&text).unwrap();
                                    let space = text.find(' ').unwrap();
                                    let mut id_parts = text[3..space].split('_');
                                    let srvid = id_parts.next().unwrap().parse::<Uuid>().unwrap();
                                    let wsid = id_parts.next().unwrap().parse::<usize>().unwrap();
                                    let msg = text[space+1..].to_owned();

                                    if srvid == *SERVER_ID {
                                        let mut locked_senders = WS_SENDERS.lock().unwrap();
                                        let locked_rpcs = RPC_SENDERS.lock().unwrap();
                                        println!("Received RPC for WSID {}_{}: {}", srvid, wsid, msg);
                                        if let Some(rpc) = locked_rpcs.get(&wsid) {
                                            if msg == "ABORT" {
                                                rpc.send(None).unwrap();
                                            } else {
                                                rpc.send(Some(msg)).unwrap();
                                            }
                                        } else {
                                            locked_senders[wsid].send_message(&Message::text("RPC ERROR: nobody listening".to_owned())).unwrap();
                                        }
                                    }
                                } else {
                                    cctx.send(CmdFrom::Data(str::from_utf8(&text).unwrap().to_owned())).unwrap();
                                }
                            },
                            _ => ()
                        }
                    }
                }));
            } else {
                println!("Websocket connection with no suitable protocols!");
            }
        }

        println!("joining marshal");
        marshal.join().unwrap();
        println!("joined marshal");
    })
}

pub fn ouroboros() {
    let _ = ClientBuilder::new(&format!("ws://0.0.0.0:{}", config::WS_PORT)).unwrap()
        .add_protocol("ouroboros")
        .connect_insecure();
}

