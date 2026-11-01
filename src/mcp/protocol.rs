//! What `openpacketmon mcp` and the window say to each other: one line of JSON each way.

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Largest question the window reads
pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
/// Largest answer `openpacketmon mcp` reads
const MAX_ANSWER_BYTES: usize = 16 * 1024 * 1024;
/// How long the window may take to connect
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// How long an answer may take
const ANSWER_TIMEOUT: Duration = Duration::from_secs(60);

/// Content of the file that tells `openpacketmon mcp` where the window is
#[derive(Serialize, Deserialize, Debug)]
pub struct AccessFile {
    pub port: u16,
    pub token: String,
}

/// A question to the window
#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub token: String,
    pub tool: String,
    #[serde(default)]
    pub arguments: Value,
}

/// The answer of the window
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    /// The result of the tool
    Ok(Value),
    /// The tool could not answer; the text is for the assistant
    Error(String),
    /// Not a valid question or the wrong token
    Refused,
}

/// Why a question did not reach an answer
#[derive(Debug, PartialEq, Eq)]
pub enum AskError {
    /// No window to ask: closed, or the access is off
    NotAvailable,
    /// The window turned the question down, or its answer made no sense
    Refused,
    /// The tool could not answer
    Tool(String),
}

/// Reads up to the end of the line; `None` if it is too long or does not arrive in time
pub fn read_line(stream: &mut TcpStream, max: usize, timeout: Duration) -> Option<Vec<u8>> {
    let deadline = Instant::now() + timeout;
    let mut line = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        // the timeout of a read starts anew with each byte, so that alone would not end
        let left = deadline.checked_duration_since(Instant::now())?;
        stream
            .set_read_timeout(Some(left.max(Duration::from_millis(1))))
            .ok()?;
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        if let Some(end) = chunk[..n].iter().position(|b| *b == b'\n') {
            line.extend_from_slice(&chunk[..end]);
            return (line.len() <= max).then_some(line);
        }
        line.extend_from_slice(&chunk[..n]);
        if line.len() > max {
            return None;
        }
    }
}

/// Writes one value as a line of JSON
pub fn write_line(stream: &mut TcpStream, value: &impl Serialize) -> io::Result<()> {
    let mut line = serde_json::to_vec(value)?;
    line.push(b'\n');
    stream.write_all(&line)
}

/// Asks the window that wrote the access file
pub fn ask(path: &Path, tool: &str, arguments: Value) -> Result<Value, AskError> {
    let file = std::fs::read(path).map_err(|_| AskError::NotAvailable)?;
    let file: AccessFile = serde_json::from_slice(&file).map_err(|_| AskError::NotAvailable)?;
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, file.port));
    let mut stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)
        .map_err(|_| AskError::NotAvailable)?;
    let request = Request {
        token: file.token,
        tool: tool.to_owned(),
        arguments,
    };
    stream
        .set_write_timeout(Some(CONNECT_TIMEOUT))
        .map_err(|_| AskError::NotAvailable)?;
    write_line(&mut stream, &request).map_err(|_| AskError::NotAvailable)?;
    let line =
        read_line(&mut stream, MAX_ANSWER_BYTES, ANSWER_TIMEOUT).ok_or(AskError::NotAvailable)?;
    match serde_json::from_slice(&line) {
        Ok(Answer::Ok(value)) => Ok(value),
        Ok(Answer::Error(text)) => Err(AskError::Tool(text)),
        Ok(Answer::Refused) | Err(_) => Err(AskError::Refused),
    }
}
