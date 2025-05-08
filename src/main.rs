mod server;

use eframe::{
    egui::{self, FontId, Key, TopBottomPanel, Vec2, ViewportBuilder},
    App, Frame, NativeOptions,
};
use std::net::SocketAddr;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::TcpStream,
    runtime::Runtime,
    sync::mpsc::{self, Receiver, Sender},
};

fn main() -> Result<(), eframe::Error> {
    //  the chat server in the background
    let mut rt_main = Runtime::new().unwrap();
    rt_main.spawn(async {
        if let Err(e) = server::run_server().await {
            eprintln!("Server error: {}", e);
            std::process::exit(1);
        }
    });
    // launch egui window
    let addr: SocketAddr = "127.0.0.1:4040".parse().unwrap();

    eframe::run_native(
        "Chat Server",
        NativeOptions::default(),
        Box::new(move |cc| Box::new(ChatApp::new(cc, addr))),
    )
}

struct ChatApp {
    messages: Vec<String>,
    input: String,
    tx_write: Sender<String>,
    rx_read: Receiver<String>,
    rt: Runtime,
}

impl ChatApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, addr: SocketAddr) -> Self {
        let rt = Runtime::new().unwrap();
        //dark theme
        _cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let (tx_read, rx_read) = mpsc::channel::<String>(100);

        let (tx_write, mut rx_write) = mpsc::channel::<String>(100);

        // spawns an async block to manage the socket
        rt.spawn(async move {
            let stream = TcpStream::connect(addr).await.unwrap();
            let (r, w) = stream.into_split();
            let mut reader = BufReader::new(r);
            let mut writer = BufWriter::new(w);

            // read messages from server
            let mut buf = String::new();
            let tx_read_clone = tx_read.clone();
            tokio::spawn(async move {
                loop {
                    buf.clear();
                    let n = reader.read_line(&mut buf).await.unwrap_or(0);
                    if (n == 0) {
                        break;
                    }
                    let line = buf.trim_end().to_owned();
                    let _ = tx_read_clone.send(line).await;
                }
            });

            // send messages to server
            tokio::spawn(async move {
                while let Some(msg) = rx_write.recv().await {
                    if (writer.write_all(msg.as_bytes()).await.is_err()) {
                        break;
                    }
                    let _ = writer.write_all(b"\n").await;
                    let _ = writer.flush().await;
                }
            });
        });

        Self {
            messages: Vec::new(),
            input: String::new(),
            tx_write,
            rx_read,
            rt,
        }
    }
}

impl App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // keeps checking for messages and appending to array
        while let Ok(msg) = self.rx_read.try_recv() {
            self.messages.push(msg);
        }

        // build UI
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // scrollable chat
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for m in &self.messages {
                            ui.label(
                                egui::RichText::new(m)
                                    .font(FontId::new(20.0, egui::FontFamily::Monospace)),
                            );
                        }
                    });

                ui.add_space(10.0);

                // text input at the bottom
                TopBottomPanel::bottom("input_panel").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        // single line text edit
                        let text_edit = egui::TextEdit::singleline(&mut self.input)
                            .hint_text("Type a message…")
                            .frame(true)
                            .desired_width(f32::INFINITY)
                            .font(FontId::new(20.0, egui::FontFamily::Monospace))
                            .lock_focus(false);
                        let response = ui.add(text_edit);

                        // sends the message to server when enter is pressed
                        if ui.input(|i| i.key_pressed(Key::Enter)) {
                            if !self.input.trim().is_empty() {
                                let msg = self.input.trim().to_owned();
                                self.input.clear();
                                let mut tx = self.tx_write.clone();
                                self.rt.spawn(async move {
                                    let _ = tx.send(msg).await;
                                });
                            }

                            // refocusing onto text edit
                            response.request_focus();
                        }
                    });
                });
            });
        });

        // keeps UI responsive
        ctx.request_repaint();
    }
}
