use crate::pow::POW;
use anyhow::{anyhow, Result};
use fs_extra;
use log::{error, info, warn};
use std::net::SocketAddr;
use std::pin::Pin;
use std::{env, fs::create_dir, path::Path, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;
use tokio::time::{self, Duration};
use uuid::Uuid;
pub struct Handler {
    pub port: String,
    pub compose_dir: String,
    pub pow_difficulty: usize,
    pub pow_timeout: u64,
    pub service_timeout: u64,
}
struct Client {
    socket: Pin<Box<TcpStream>>,
    addr: Pin<Box<SocketAddr>>,
    pass_pow: bool,
    service_name: Option<String>,
    temp_dir: Option<String>,
}
impl Handler {
    pub async fn handle(self: Arc<Self>) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;
        info!("Listening on port {}", self.port);
        loop {
            let (socket, addr) = listener.accept().await?;
            info!("New connection: {}", addr);
            let self_clone = self.clone();
            let mut client = Client {
                socket: Box::pin(socket),
                addr: Box::pin(addr),
                pass_pow: false,
                service_name: None,
                temp_dir: None,
            };

            tokio::spawn(async move {
                if let Err(e) = self_clone.handle_connect(&mut client).await {
                    warn!("Connection {}: {}", *client.addr, e);
                    if let Err(disconnect_error) = self_clone.handle_disconnect(&client).await {
                        error!(
                            "Failed to disconnect {}: {}",
                            *client.addr, disconnect_error
                        );
                    }
                }
            });
        }
    }
    async fn handle_connect(&self, client: &mut Client) -> Result<()> {
        let mut buf = vec![0; 1024];

        let pow = POW::init(self.pow_difficulty);
        client
            .socket
            .write_all(
                format!(
                    "Welcome to the proof of work challenge\n\
                    You have {} seconds to solve the PoW\n\
                    assert sha256('{}' + ?).hexdigest().startswith('0' * {}) == True\n\
                    ? = ",
                    self.pow_timeout.to_string(),
                    pow.nonce_str.clone().unwrap(),
                    self.pow_difficulty.to_string()
                )
                .as_bytes(),
            )
            .await?;
        loop {
            match time::timeout(
                Duration::from_secs(self.pow_timeout),
                client.socket.read(&mut buf),
            )
            .await
            {
                Ok(Ok(n)) => loop {
                    let input = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                    if !client.pass_pow {
                        if pow.verify(input) {
                            info!("Pow accepted for {}", *client.addr);
                            client.pass_pow = true;
                            client
                                .socket
                                .write_all(
                                    format!(
                                        "\nPoW accepted, starting service, you have {} seconds\n",
                                        self.service_timeout.to_string()
                                    )
                                    .as_bytes(),
                                )
                                .await?;
                            let port = self.start_service(client).await?;
                            client
                                .socket
                                .write_all(format!("Service started on port {}\n", port).as_bytes())
                                .await?;
                            let mut buf = vec![0; 1024];
                            loop {
                                // let client_clone = Arc::clone(&mut Client);
                                // let mut client = client_clone.lock().unwrap();
                                match time::timeout(
                                    Duration::from_secs(self.service_timeout),
                                    client.socket.read(&mut buf),
                                )
                                .await
                                {
                                    Ok(Ok(n)) => {
                                        if n == 0 {
                                            return Err(anyhow!("Client closes the connection",));
                                        }
                                    }
                                    Ok(Err(e)) => {
                                        // ???
                                        return Err(e.into());
                                    }
                                    Err(_) => {
                                        // no input for service_timeout
                                        return Err(anyhow!("Service timeout"));
                                    }
                                }
                            }
                        } else {
                            client.socket.write_all("Invalid PoW\n".as_bytes()).await?;
                            break;
                        }
                    }
                },
                Ok(Err(e)) => {
                    // Client closes the connection
                    return Err(e.into());
                }
                Err(_) => {
                    // PoW timeout
                    return Err(anyhow!("PoW timeout"));
                }
            }
        }
    }
    async fn handle_disconnect(&self, client: &Client) -> Result<()> {
        if let Some(temp_dir) = &client.temp_dir {
            fs_extra::remove_items(&vec![temp_dir])?;
            warn!("removed directory {}", temp_dir);
        }
        self.remove_service(client).await?;
        Ok(())
    }
    async fn start_service(&self, client: &mut Client) -> Result<String> {
        // 1. Create a temporary directory
        // /tmp/pow-compose-[uuid]
        let service_name = format!("pow-compose-{}", Uuid::new_v4().to_string());
        client.service_name = Some(service_name.clone());
        let mut temp_dir_path = Path::new(env::temp_dir().as_path()).join(&service_name);
        create_dir(&temp_dir_path)?;
        // dbg!(&temp_dir_path);
        client.temp_dir = Some(temp_dir_path.clone().to_string_lossy().to_string());
        // 2. Copy the compose file to the temporary directory
        fs_extra::copy_items(
            &vec![&self.compose_dir],
            &temp_dir_path,
            &fs_extra::dir::CopyOptions::new(),
        )?;
        // now we have a temporary compose directory /tmp/pow-compose-[uuid]/[compose_dir]
        temp_dir_path = temp_dir_path.join(&self.compose_dir);
        // 3. find available port
        let port: String;
        match TcpListener::bind("127.0.0.1:0")
            .await
            .ok()
            .and_then(|listener| listener.local_addr().ok().map(|addr| addr.port()))
        {
            Some(p) => port = p.to_string(),
            None => return Err(anyhow!("No available port")),
        };
        info!("Open service on port {port}");
        // dbg!(&port);
        // 4. replace port in docker-compose.tpl and write to docker-compose.yml
        let compose_file_path = temp_dir_path.join("docker-compose.tpl");
        let mut compose_file = std::fs::read_to_string(&compose_file_path)?;
        compose_file = compose_file.replace("{{port}}", &port);
        std::fs::write(temp_dir_path.join("docker-compose.yml"), compose_file)?;
        // 5. start the service
        Command::new("docker")
            .args(&["compose", "-p", &service_name, "up", "-d"])
            .current_dir(&temp_dir_path)
            .output()
            .await?;
        // dbg!(output);
        Ok(port)
    }
    async fn remove_service(&self, client: &Client) -> Result<()> {
        if let Some(service_name) = &client.service_name {
            Command::new("docker")
                .args(&["compose", "-p", &service_name, "down"])
                .output()
                .await?;
            // dbg!(output);
        }
        Ok(())
    }
}
#[tokio::test]
async fn test_handle() {
    let handler = Arc::new(Handler {
        port: "1337".to_string(),
        compose_dir: "./example/".to_string(),
        pow_difficulty: 6,
        pow_timeout: 10,
        service_timeout: 10,
    });
    handler.handle().await.unwrap();
}
