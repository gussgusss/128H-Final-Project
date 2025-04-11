use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::broadcast,
};

#[tokio::main]
async fn main() {
    // echo server
    let listener = TcpListener::bind("localhost:4040").await.unwrap();
    let (tx, _) = broadcast::channel(10);

    // ensures multiple users can connect
    loop {
        let (mut socket, address) = listener.accept().await.unwrap();
        let tx = tx.clone();
        let mut rx = tx.subscribe();

        // spawns an async thread for each user
        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            // for more than 1 message
            loop {
                // returns first branch that finishes - user sends msg / receiving msg from other user
                tokio::select! {
                    user_msg = reader.read_line(&mut line) => {
                        // loop breaks when user disconnects
                        if (user_msg.unwrap() == 0) {
                            break;
                        }

                        tx.send((line.clone(), address)).unwrap();
                        line.clear();
                    }
                    other_msg = rx.recv() => {
                        let (mut msg, mut other_address) = other_msg.unwrap();

                        // doesn't show the message if it has the user's address
                        if (other_address != address) {
                            writer.write_all(msg.as_bytes()).await.unwrap();
                        }
                    }
                }
            }
        });
    }
}
