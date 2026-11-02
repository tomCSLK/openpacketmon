//! What `openpacketmon mcp` and the window say to each other, one line of JSON at a time.
//!
//! Both show that they know the token without sending it: `openpacketmon mcp` says [`Hello`]
//! with a random nonce, the window answers with a [`Challenge`] that proves the token over this
//! nonce and brings a nonce of its own, and the [`Request`] proves the token over that one. So
//! a program that took the port of a closed window learns no token and gets no question, and a
//! program that knocks at the door learns nothing it could use.

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::time::{Duration, Instant};

use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Largest question the window reads
pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
/// Largest answer `openpacketmon mcp` reads
const MAX_ANSWER_BYTES: usize = 16 * 1024 * 1024;
/// How long the window may take to connect
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// How long an answer may take
const ANSWER_TIMEOUT: Duration = Duration::from_secs(60);
/// How long each step before the question may take, on both sides
pub const STEP_TIMEOUT: Duration = Duration::from_secs(5);
/// Largest line before the question
const MAX_STEP_BYTES: usize = 1024;
/// Random bytes in a token or a nonce
const RANDOM_BYTES: usize = 32;

/// Content of the file that tells `openpacketmon mcp` where the window is
#[derive(Serialize, Deserialize, Debug)]
pub struct AccessFile {
    pub port: u16,
    pub token: String,
}

/// The first line of `openpacketmon mcp`
#[derive(Serialize, Deserialize, Debug)]
pub struct Hello {
    pub nonce: String,
}

/// The window proves that it knows the token, and asks the same in return
#[derive(Serialize, Deserialize, Debug)]
pub struct Challenge {
    /// Nonce of the window, for the proof of the question
    pub nonce: String,
    /// [`window_proof`] over the nonce of the hello
    pub proof: String,
}

/// A question to the window
#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    /// [`client_proof`] over the nonce of the challenge
    pub proof: String,
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
    /// The window took the question but no answer came: busy, or closed in the meantime
    NoAnswer,
    /// The window turned the question down, or its answer made no sense
    Refused,
    /// Something else listens at the port of the window and does not know the token
    Stranger,
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
    stream
        .set_write_timeout(Some(CONNECT_TIMEOUT))
        .map_err(|_| AskError::NotAvailable)?;
    let nonce = random_hex();
    write_line(
        &mut stream,
        &Hello {
            nonce: nonce.clone(),
        },
    )
    .map_err(|_| AskError::NotAvailable)?;
    let line = read_line(&mut stream, MAX_STEP_BYTES, STEP_TIMEOUT).ok_or(AskError::NoAnswer)?;
    let challenge = match serde_json::from_slice::<Challenge>(&line) {
        Ok(challenge) => challenge,
        Err(_) if matches!(serde_json::from_slice(&line), Ok(Answer::Refused)) => {
            return Err(AskError::Refused);
        }
        Err(_) => return Err(AskError::Stranger),
    };
    // whoever does not know the token gets neither the question nor a proof
    if !same(&challenge.proof, &window_proof(&file.token, &nonce)) || !is_nonce(&challenge.nonce) {
        return Err(AskError::Stranger);
    }
    let request = Request {
        proof: client_proof(&file.token, &challenge.nonce),
        tool: tool.to_owned(),
        arguments,
    };
    write_line(&mut stream, &request).map_err(|_| AskError::NoAnswer)?;
    let line =
        read_line(&mut stream, MAX_ANSWER_BYTES, ANSWER_TIMEOUT).ok_or(AskError::NoAnswer)?;
    match serde_json::from_slice(&line) {
        Ok(Answer::Ok(value)) => Ok(value),
        Ok(Answer::Error(text)) => Err(AskError::Tool(text)),
        Ok(Answer::Refused) | Err(_) => Err(AskError::Refused),
    }
}

/// A new token or nonce: random bytes as hex
pub fn random_hex() -> String {
    let mut bytes = [0_u8; RANDOM_BYTES];
    rand::rng().fill(&mut bytes);
    hex(&bytes)
}

/// Whether a text looks like a nonce of [`random_hex`]
pub fn is_nonce(text: &str) -> bool {
    text.len() == RANDOM_BYTES * 2 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// What the window shows for the nonce of `openpacketmon mcp`
pub fn window_proof(token: &str, nonce: &str) -> String {
    hex(&hmac_sha256(
        token.as_bytes(),
        format!("window\n{nonce}").as_bytes(),
    ))
}

/// What `openpacketmon mcp` shows for the nonce of the window; another text than the proof of
/// the window, so that the window cannot be made to answer its own challenge
pub fn client_proof(token: &str, nonce: &str) -> String {
    hex(&hmac_sha256(
        token.as_bytes(),
        format!("client\n{nonce}").as_bytes(),
    ))
}

/// Whether two texts are the same, in a time that does not depend on where they differ
pub fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0, |d, (x, y)| d | (x ^ y)) == 0
}

/// HMAC with SHA-256 (RFC 2104)
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut block = [0_u8; BLOCK];
    if key.len() > BLOCK {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let pad = |byte: u8| block.map(|b| b ^ byte);
    let inner = Sha256::new()
        .chain_update(pad(0x36))
        .chain_update(message)
        .finalize();
    Sha256::new()
        .chain_update(pad(0x5c))
        .chain_update(inner)
        .finalize()
        .into()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_matches_rfc_4231() {
        // test cases 1, 2 and 6
        let cases: [(&[u8], &[u8], &str); 3] = [
            (
                &[0x0b; 20],
                b"Hi There",
                "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
            ),
            (
                b"Jefe",
                b"what do ya want for nothing?",
                "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
            ),
            (
                &[0xaa; 131],
                b"Test Using Larger Than Block-Size Key - Hash Key First",
                "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
            ),
        ];
        for (key, message, expected) in cases {
            assert_eq!(hex(&hmac_sha256(key, message)), expected);
        }
    }

    #[test]
    fn the_proofs_differ_by_side_and_nonce() {
        let token = random_hex();
        let nonce = random_hex();
        assert!(is_nonce(&nonce) && is_nonce(&token));
        assert_ne!(nonce, token);
        assert_ne!(window_proof(&token, &nonce), client_proof(&token, &nonce));
        assert_ne!(
            window_proof(&token, &nonce),
            window_proof(&token, &random_hex())
        );
        assert_ne!(
            window_proof(&token, &nonce),
            window_proof(&random_hex(), &nonce)
        );
        assert!(!is_nonce("") && !is_nonce(&nonce[1..]) && !is_nonce(&"g".repeat(64)));
    }

    #[test]
    fn same_compares_whole_texts() {
        assert!(same("abc", "abc") && same("", ""));
        assert!(!same("abc", "abd") && !same("abc", "ab") && !same("", "a"));
    }
}
