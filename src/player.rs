pub mod Player {
    use std::{process::{Command, Stdio}};
    #[cfg(feature = "remote")]
    use serde::ser::Error;
    #[cfg(feature = "remote")]
    use std::{net::TcpStream, sync::{atomic::{AtomicBool, Ordering}, Arc}, mpsc::{self, Sender}, thread::{sleep, spawn}, time::Duration, format, io::{Read, Write}, path::Path };
    #[cfg(feature = "remote")]
    use openssl::rand::{self, rand_bytes};
    #[cfg(feature = "remote")]
    use ssh2::Session;

use crate::{config::DeviceSettings, log};
    pub trait MangaPlayer {
        fn read();
    }
    pub trait AnimePlayer {
        fn play(&mut self, url: &str);
    }
    pub trait LocalPlayer {
        fn exec();
    }

    #[cfg(feature = "remote")]
    pub trait RemotePlayer {
        fn initConnection();
        fn ssh(&self) -> Result<Sender<VlcrRemoteMessage>, Box<dyn std::error::Error>>;
    } 

    pub struct Vlc {
        pub cmd: String,
        pub args: Vec<String>
    }
    impl AnimePlayer for Vlc {
        fn play(&mut self, url: &str) {
            Command::new(self.cmd.clone()).args(self.args.clone()).arg(url)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
    }
    #[cfg(feature = "remote")]
    pub struct Vlcr {
        pub dev: Option<DeviceSettings>,
        pub tx: Option<Sender<VlcrRemoteMessage>>
    }
    #[cfg(feature = "remote")]
    pub enum VlcrRemoteMessage {
        quit,
        add(String),
        play,
    }
    #[cfg(feature = "remote")]
    impl AnimePlayer for Vlcr {
        
        fn play(&mut self, url: &str) {
            log!("play url: {}",url);
            if self.tx.is_none() {
                log!("Tx none so ssh");
                match self.ssh() {
                    Ok(tx) => self.tx=Some(tx),
                    Err(err) => {
                        log!("Errore nella creazione di tx: {}", err);
                        return;
                    },
                }
            }

            if let Some(tx) = &self.tx {
                tx.send(VlcrRemoteMessage::add(url.to_owned()));
                tx.send(VlcrRemoteMessage::play);
            }
        }
    }
    #[cfg(feature = "remote")]
    impl RemotePlayer for Vlcr {
        fn initConnection() {}
        fn ssh(&self) -> Result<Sender<VlcrRemoteMessage>, Box<dyn std::error::Error>> {
            if let Some(dev) = &self.dev {
                let tcp = TcpStream::connect(&dev.host)?;
                let mut session = Session::new()?;
                session.set_tcp_stream(tcp);
                session.handshake()?;

                session.userauth_pubkey_file(&dev.user, None, Path::new(&dev.key), None)?;
                if !session.authenticated() {
                    return Err("SSH authentication failed".into());
                }

                let mut channel = session.channel_session()?;

                channel.request_pty(
                    "xterm-256color",
                    None,
                    Some((250, 80, 1920, 1080)),
                )?;
                let vlc_command = "pkill vlc; sleep 0.5; vlc --one-instance -f --extraintf rc";
                channel.exec(vlc_command)?;

                let (tx, rx) = mpsc::channel::<VlcrRemoteMessage>();
                let is_alive = Arc::new(AtomicBool::new(true));
                let is_alive_worker = Arc::clone(&is_alive);

                let worker_handle = spawn(move || {
                    while is_alive_worker.load(Ordering::Relaxed) {
                        if channel.eof() {
                            println!("\r\n[VLC Controller] Remote VLC process closed.");
                            break;
                        }

                        while let Ok(cmd) = rx.try_recv() {
                            // let sendCommand = {
                            //     if channel.write_all(formatted.as_bytes()).is_err() {
                            //         is_alive_worker.store(false, Ordering::Relaxed);
                            //         break;
                            //     }
                            // }
                            match cmd {
                                VlcrRemoteMessage::quit => {
                                    let _ = channel.write_all(b"quit\n");
                                    let _ = channel.flush();
                                    is_alive_worker.store(false, Ordering::Relaxed);
                                    break;
                                }
                                VlcrRemoteMessage::add(what) => {
                                    let formatted = format!("add {}\n",what);
                                    if channel.write_all(formatted.as_bytes()).is_err() {
                                        is_alive_worker.store(false, Ordering::Relaxed);
                                    }
                                }
                                VlcrRemoteMessage::play => {
                                    let formatted = format!("play\n");
                                    if channel.write_all(formatted.as_bytes()).is_err() {
                                        is_alive_worker.store(false, Ordering::Relaxed);
                                    }
                                },
                            }

                            // Otherwise write the arbitrary command line into VLC
                            
                            let _ = channel.flush();
                        }

                        if !is_alive_worker.load(Ordering::Relaxed) {
                            break;
                        }

                        // Read and stream VLC stdout to local terminal
                        // match channel.read(&mut buf) {
                        //     Ok(0) => {
                        //         // EOF received
                        //         break;
                        //     }
                        //     Ok(n) => {
                        //         let _ = std::io::stdout().write_all(&buf[..n]);
                        //         let _ = std::io::stdout().flush();
                        //     }
                        //     Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        //         // Channel read timeout: yield briefly to prevent 100% CPU loop
                        //         sleep(Duration::from_millis(20));
                        //     }
                        //     Err(_) => {
                        //         // Read error or network broken
                        //         break;
                        //     }
                        // }
                    }

                    // Cleanup & safe shutdown
                    is_alive_worker.store(false, Ordering::Relaxed);
                    let _ = channel.close();
                    let _ = channel.wait_close();
                });
                log!("ssh return smt");
                return Ok(tx);
            }
            Err("Nothing initialized".into())
        }
    }
}