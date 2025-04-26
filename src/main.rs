use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
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

struct Room {
    tx: Sender<String>,
}

impl Room {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(10);
        Self { tx }
    }
}

const MAIN: &str = "main";

#[derive(Clone)]
// rwlock allows multiple readers or one writer
struct Rooms(Arc<RwLock<HashMap<String, Room>>>);

impl Rooms {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    fn join(&self, room_name: &str) -> Sender<String> {
        // get read access
        let read_guard = self.0.read().unwrap();
        // check if room already exists. if yes, returns its sender
        if let Some(room) = read_guard.get(room_name) {
            return room.tx.clone();
        }
        // must drop read before acquiring write
        drop(read_guard);

        // get write access and create new room, returning sender
        let mut write_guard = self.0.write().unwrap();
        write_guard.insert(room_name.to_owned(), Room::new());
        let room = write_guard.get(room_name).unwrap();
        room.tx.clone()
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("localhost:4040").await.unwrap();
    let (tx, _) = broadcast::channel(10);
    let names = Names::new();
    let rooms = Rooms::new();

    accept_loop(listener, tx, names, rooms).await;
}

// ensures multiple users can connect
async fn accept_loop(
    listener: TcpListener,
    tx: Sender<(String, SocketAddr)>,
    names: Names,
    rooms: Rooms,
) {
    loop {
        let (socket, address) = listener.accept().await.unwrap();
        let tx_clone = tx.clone();
        let rx = tx.subscribe();

        // spawns an async thread for each user
        tokio::spawn(handle_user(
            socket,
            address,
            tx_clone,
            rx,
            names.clone(),
            rooms.clone(),
        ));
    }
}

async fn handle_user(
    mut socket: TcpStream,
    address: SocketAddr,
    tx: Sender<(String, SocketAddr)>,
    mut rx: Receiver<(String, SocketAddr)>,
    names: Names,
    rooms: Rooms,
) {
    let (reader, mut writer) = socket.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut name: String = "Guest".to_string();
    let mut room_name = "main".to_owned();
    let mut room_tx = rooms.join(&room_name);
    let mut room_rx = room_tx.subscribe();

    writer
        .write_all(format!("📢: Your name is {name}\n").as_bytes())
        .await
        .unwrap();
    writer.flush().await.unwrap();

    let _ = room_tx.send(format!("📢: {name} has joined {room_name}\n"));

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
                        tx.send((format!("📢: {name} is now {new_name}\n"), address)).unwrap();
                        name = new_name;
                        writer.write_all(format!("📢: Your name is now {name}\n").as_bytes()).await.unwrap();
                        writer.flush().await.unwrap();
                    } else {
                        writer.write_all(format!("📢: The name {new_name} has already been taken, please choose a new one.\n").as_bytes()).await.unwrap();
                        writer.flush().await.unwrap();
                    }
                    line.clear();
                } else if (line.starts_with("/join")) {
                    let new_room = line[6..].trim().to_string().to_owned();
                    if (new_room != room_name) {
                        let _ = room_tx.send(format!("📢: {name} has left {room_name}\n"));
                        room_name = new_room;
                        room_tx = rooms.join(&room_name);
                        room_rx = room_tx.subscribe();
                        let _ = room_tx.send(format!("📢: {name} has joined {room_name}\n"));
                    } else {
                        writer.write_all(format!("📢: You are already in {room_name}\n").as_bytes()).await.unwrap();
                        writer.flush().await.unwrap();
                    }

                } else if (line.trim_end() == "/quit"){
                    break;

                } else {
                    room_tx.send(format!("({name}): {line}")).unwrap();
                    line.clear();
                }
            }

            // when any broadcast arrives
            other_msg = room_rx.recv() => {
                let msg = other_msg.unwrap();
                // doesn't show the message if it has the user's address

                    writer.write_all(msg.as_bytes()).await.unwrap();

            }
        }
    }
    names.remove(&name);
}
