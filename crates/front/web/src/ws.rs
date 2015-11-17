use std::collections::HashMap;
use std::sync::{mpsc, Mutex};
use std::{thread, str};
use comms::CmdFrom;
use super::config;
pub use websocket::{Sender, Receiver, Message};
use websocket::message::Type as MsgType;
use websocket::{server, header, stream, Server, Client};
use url::Host;

lazy_static! {
    pub static ref RPC_SENDERS: Mutex<HashMap<usize, mpsc::Sender<String>>>                         = Mutex::new(HashMap::new());
    pub static ref WS_SENDERS:  Mutex<Vec<server::sender::Sender<stream::WebSocketStream>>> = Mutex::new(Vec::new());
}

pub fn send(wsid: usize, msg: String) {
    let mut locked_senders = WS_SENDERS.lock().unwrap();

    locked_senders[wsid].send_message(&Message::text(msg)).unwrap();
}

pub fn rpc<T, F: Fn(String) -> Result<T, String>>(wsid: usize, prompt: String, validator: F) -> T {
    let go = |prompt: &str| -> String {
        let (tx, rx) = mpsc::channel();
        println!("Waiting on RPC from WSID {}", wsid);
        {
            let mut locked_senders = WS_SENDERS.lock().unwrap();
            let mut locked_rpcs = RPC_SENDERS.lock().unwrap();
            locked_rpcs.insert(wsid, tx);
            locked_senders[wsid].send_message(&Message::text(prompt.to_owned())).unwrap();
        }
        rx.recv().unwrap()
    };

    let mut answer = go(&prompt);
    loop {
        match validator(answer) {
            Ok(ret) => return ret,
            Err(admonish) => {
                answer = go(&format!("{} {}", admonish, prompt));
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

        'listener: for connection in ws {
            let request = connection.unwrap().read_request().unwrap(); // Get the request
            let headers = request.headers.clone(); // Keep the headers so we can check them

            request.validate().unwrap(); // Validate the request

            let mut response = request.accept(); // Form a response

            if let Some(&header::WebSocketProtocol(ref protocols)) = headers.get() {
                if protocols.contains(&"ouroboros".to_owned()) {
                    // Shutdown signal
                    println!("Websocket listener received shutdown signal");
                    break 'listener;
                } else if protocols.contains(&"rust-websocket".to_owned()) {
                    // We have a protocol we want to use
                    response.headers.set(header::WebSocketProtocol(vec!["rust-websocket".to_owned()]));

                    let mut client = response.send().unwrap(); // Send the response

                    let ip = client.get_mut_sender()
                        .get_mut()
                        .peer_addr()
                        .unwrap();

                    println!("Websocket connection from {}", ip);

                    let mut locked_senders = WS_SENDERS.lock().unwrap();
                    let wsid = locked_senders.len();

                    client.send_message(&Message::text(format!("hello {}", wsid))).unwrap();

                    let (sender, mut receiver) = client.split();
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
                                        let id = text[3..space].parse::<usize>().unwrap();
                                        let msg = text[space+1..].to_owned();

                                        let mut locked_senders = WS_SENDERS.lock().unwrap();
                                        let locked_rpcs = RPC_SENDERS.lock().unwrap();
                                        println!("Received RPC for WSID {}: {}", wsid, msg);
                                        if let Some(rpc) = locked_rpcs.get(&id) {
                                            rpc.send(msg).unwrap();
                                        } else {
                                            locked_senders[id].send_message(&Message::text("RPC ERROR: nobody listening".to_owned())).unwrap();
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
            } else {
                println!("Websocket connection with no protocols!");
            }
        }

        println!("joining marshal");
        marshal.join().unwrap();
        println!("joined marshal");
    })
}

pub fn ouroboros() {
    let mut req = Client::connect((Host::Domain("0.0.0.0".to_owned()), config::WS_PORT, "", false)).unwrap();
    req.headers.set(header::WebSocketProtocol(vec!["ouroboros".to_owned()]));
    let _ = req.send();
}

