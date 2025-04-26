use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    sync::broadcast::{self, Receiver, Sender},
};

#[derive(Clone)]
//arc can share ownership
//mutex makes sure only one thread can change data at a time
struct Names(Arc<Mutex<HashSet<String>>>);

impl Names {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(HashSet::new())))
    }

    // inserts name. returns false if dupe
    fn insert(&self, name: String) -> bool {
        self.0.lock().unwrap().insert(name)
    }

    fn remove(&self, name: &str) -> bool {
        self.0.lock().unwrap().remove(name)
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("localhost:4040").await.unwrap();
    let (tx, _) = broadcast::channel(10);
    let names = Names::new();

    accept_loop(listener, tx, names).await;
}

// ensures multiple users can connect
async fn accept_loop(listener: TcpListener, tx: Sender<(String, SocketAddr)>, names: Names) {
    loop {
        let (socket, address) = listener.accept().await.unwrap();
        let tx_clone = tx.clone();
        let rx = tx.subscribe();

        // spawns an async thread for each user
        tokio::spawn(handle_client(socket, address, tx_clone, rx, names.clone()));
    }
}

async fn handle_client(
    mut socket: TcpStream,
    address: SocketAddr,
    tx: Sender<(String, SocketAddr)>,
    mut rx: Receiver<(String, SocketAddr)>,
    names: Names,
) {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut name: String = address.to_string();

    writer
        .write_all(format!("❗: Your name is {name}\n").as_bytes())
        .await
        .unwrap();
    writer.flush().await.unwrap();

    // loop for 1< message
    loop {
        // returns first branch that finishes - user sends msg / receiving msg from other user
        tokio::select! {
            // when this user sends msg
            user_msg = reader.read_line(&mut line) => {
                if user_msg.unwrap() == 0 {
                    //disconnect
                    break;
                } else if (line.starts_with("/name ")) {

                    let new_name = line[6..].trim().to_string();
                    let name_changed = names.insert(new_name.clone());
                    if (name_changed) {
                        names.remove(&name);
                        tx.send((format!("❗: {name} is now {new_name}\n"), address)).unwrap();
                        name = new_name;
                        writer.write_all(format!("❗: Your name is now {name}\n").as_bytes()).await.unwrap();
                        writer.flush().await.unwrap();
                    } else {
                        writer.write_all(format!("❗: The name {new_name} has already been taken, please choose a new one.\n").as_bytes()).await.unwrap();
                        writer.flush().await.unwrap();
                    }
                    line.clear();
                } else if (line.trim_end() == "/quit"){
                    break;

                } else {
                    tx.send((format!("({name}): {line}"), address)).unwrap();
                    line.clear();
                }
            }

            // when any broadcast arrives
            other_msg = rx.recv() => {
                let (msg, other_address) = other_msg.unwrap();
                // doesn't show the message if it has the user's address
                if other_address != address {
                    writer.write_all(msg.as_bytes()).await.unwrap();
                }
            }
        }
    }
    names.remove(&name);
}
