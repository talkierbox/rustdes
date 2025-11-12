use std::collections::HashMap;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex}; // Will ensure that concurrent accesses will properly work
use std::thread;
use std::time::SystemTime;
use std::vec::Vec;

use crate::commands::defs::{execute, match_command};
use crate::server::util;

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Active,
    Disconnected,
} // Connection statuses

#[derive(Debug)]
pub struct ConnectionInfo {
    status: ConnectionStatus,
    address: String,
    connected_at: SystemTime,
    last_activity: SystemTime,
}

pub fn start_server(port: i32) {
    println!("Starting server on port {port}");

    let listener =
        TcpListener::bind(format!("127.0.0.1:{port}")).expect("Failed to bind on the port");

    // Arc allows for multiple ownership, Mutex allows for safe mutation across threads.
    let connections: Arc<Mutex<HashMap<u64, ConnectionInfo>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let mut next_id: u64 = 0u64;

    for stream in listener.incoming() {
        let stream = stream.expect("Stream error!");
        let id = next_id;
        next_id += 1;

        // Get the peer address
        let addr = stream
            .peer_addr()
            .expect("Error with the peer address")
            .to_string();

        let connections_clone = Arc::clone(&connections);

        // Add to the connections pool
        {
            let mut conns = connections_clone.lock().unwrap();
            conns.insert(
                id,
                ConnectionInfo {
                    status: ConnectionStatus::Active,
                    address: addr.clone(),
                    connected_at: SystemTime::now(),
                    last_activity: SystemTime::now(),
                },
            );
        }

        println!("New connection {}: {}", id, addr);

        thread::spawn(move || {
            let result = handle_client(id, stream, &connections_clone);

            // Clean up when done
            // Curly braces ensure the lock goes away after this block
            {
                let mut conns = connections_clone.lock().unwrap();
                conns.remove(&id);
            }

            println!("Connection {} closed: {:?}", id, result);
        });
    }
}

pub fn handle_client(
    id: u64,
    mut stream: TcpStream,
    connections: &Arc<Mutex<HashMap<u64, ConnectionInfo>>>,
) -> std::io::Result<()> {
    println!("Handling the client {}", id);

    let mut buffer = [0; 1024]; // 1 kb buffer

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("Client {} disconnected", id);
                {
                    let mut conns = connections.lock().unwrap();
                    if let Some(info) = conns.get_mut(&id) {
                        info.status = ConnectionStatus::Disconnected;
                    }
                }
                break;
            }
            Ok(n) => {
                // Got n bytes of data
                let received = String::from_utf8_lossy(&buffer[..n]);
                println!("Client {} sent: {}", id, received.trim());

                // Handle the input - convert errors to error messages
                let output = match handle_input(received.to_string()) {
                    Ok(result) => result,
                    Err(e) => format!("Error: {}", e),
                };

                // Send the result (or error message) back to the client
                util::send(&output, &mut stream)?;

                // Update last activity
                {
                    let mut conns = connections.lock().unwrap();
                    if let Some(info) = conns.get_mut(&id) {
                        info.last_activity = SystemTime::now();
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading from client {} -- {}", id, e);
                return Err(e);
            }
        }
    }

    Ok(())
}

fn handle_input(input: String) -> std::io::Result<String> {
    // Split up the input string, match the first word block
    let parts: Vec<&str> = input.split(" ").collect();

    let cmd_type = match_command(parts[0])?;

    // Convert &[&str] to Vec<&str> (or keep as slice)
    let args: &[&str] = &parts[1..];

    let output = execute(&cmd_type, args)?;

    Ok(output)
}
