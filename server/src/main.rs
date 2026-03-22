use colored::Colorize;
use log::{error, info};
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
};

async fn handle_client(mut stream: TcpStream) {
    'main_loop: loop {
        let mut len_buf = [0u8; protocol::PREFIX_LENGTH_BYTES];
        match stream.read_exact(&mut len_buf) {
            Ok(()) => {
                let pack_len = u32::from_be_bytes(len_buf) as usize;

                let mut content = vec![0u8; pack_len];
                if let Err(err) = stream.read_exact(&mut content) {
                    match err.kind() {
                        std::io::ErrorKind::UnexpectedEof => {
                            // 对方关闭连接, 退出循环
                            error!("Read UnexpectedEof, disconnecting...");
                            break 'main_loop;
                        }
                        _ => {
                            error!("Failed to read from client: {}, disconnecting...", err);
                            break 'main_loop;
                        }
                    }
                }

                match serde_json::from_slice::<protocol::ClientRequest>(&content) {
                    Ok(data) => {
                        info!("Received: {:?}", data);
                    }
                    Err(err) => {
                        error!(
                            "Failed to parse request from client: {}\nContent: {}",
                            err,
                            unsafe { String::from_utf8_unchecked(content.clone()) }
                        );
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // 对方关闭连接, 退出循环
                error!("Read UnexpectedEof, disconnecting...");
                break 'main_loop;
            }
            Err(err) => {
                error!("Failed to read from client: {}, disconnecting...", err);
                break 'main_loop;
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    {
        info!("{:30}", "".on_truecolor(220, 220, 220));
        #[rustfmt::skip]
        info!("{:30}", " Baker-Dx Online Server".black().on_truecolor(220, 220, 220));
        #[rustfmt::skip]
        info!("{:30}", format!(" Protocol Version {}", protocol::PROTOCOL_VERSION).black().on_truecolor(220, 220, 220));
        info!("{:30}", "".on_truecolor(220, 220, 220));
        #[rustfmt::skip]
        info!("{:30}", " Endfield Industries".black().on_truecolor(255, 255, 0));
        info!("");
        info!("Copyright (c) 2026 Wanye_7300. Licensed under MIT License.");
        info!("");
    }

    info!("Starting Server");

    let listener = TcpListener::bind("127.0.0.1:7300")?;

    for stream in listener.incoming() {
        tokio::spawn(handle_client(stream?));
    }

    Ok(())
}
