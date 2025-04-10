use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};

#[tokio::main]
async fn main() {
    // echo server
    let listener = TcpListener::bind("localhost:4040").await.unwrap();

    // ensures multiple clients can connect
    loop {
        let (mut socket, address) = listener.accept().await.unwrap();

        // spawns an async thread for each client
        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            // for more than 1 message
            loop {
                let bytes_read = reader.read_line(&mut line).await.unwrap();
                // loop breaks when client disconnects
                if (bytes_read == 0) {
                    break;
                }
                writer.write_all(line.as_bytes()).await.unwrap();
                line.clear();
            }
        });
    }
}
