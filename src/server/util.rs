use std::io::Write;
use std::net::TcpStream;

pub fn send(message: &str, client_stream: &mut TcpStream) -> Result<(), std::io::Error> {
    let fixed_message: String = message.to_string() + "\n";
    client_stream.write_all(fixed_message.as_bytes())?;
    client_stream.flush()?;
    return Ok(());
}
