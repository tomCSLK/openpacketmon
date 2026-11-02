//! The door of the window: a listener on `127.0.0.1`, open only while the user allows it.

use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::APP_NAME;
use crate::engine::Engine;
use crate::mcp::privacy::hide_in_values;
use crate::mcp::protocol::{
    AccessFile, Answer, Challenge, Hello, MAX_REQUEST_BYTES, Request, STEP_TIMEOUT, client_proof,
    is_nonce, random_hex, read_line, same, window_proof, write_line,
};
use crate::mcp::tools;
use crate::view::filter_index::FilterIndex;

/// How often the listener looks for connections and for the end of the access
const POLL: Duration = Duration::from_millis(50);
/// How long a client may take to read the answer
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);
/// Questions answered at the same time; more connections are closed
const MAX_CLIENTS: usize = 4;

/// Where the window leaves its port and token, next to the settings
pub fn access_path() -> Option<PathBuf> {
    confy::get_configuration_file_path(APP_NAME, "mcp-access")
        .ok()
        .map(|path| path.with_extension("json"))
}

/// The state of the access, for the settings
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Off,
    Active { calls: u64 },
    Failed,
}

struct Running {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

/// Opens and closes the door; a clone is the same door
#[derive(Clone)]
pub struct Access {
    engine: Engine,
    path: Option<PathBuf>,
    running: Arc<Mutex<Option<Running>>>,
    failed: Arc<AtomicBool>,
    calls: Arc<AtomicU64>,
    /// Whether assistants may read the content of packets; never stored, and off with every
    /// opening of the door
    content: Arc<AtomicBool>,
}

impl Access {
    /// A closed door; `path` is where its file goes
    pub fn new(engine: Engine, path: Option<PathBuf>) -> Self {
        Self {
            engine,
            path,
            running: Arc::default(),
            failed: Arc::default(),
            calls: Arc::default(),
            content: Arc::default(),
        }
    }

    /// Opens or closes the door; every opening has a new token
    #[allow(clippy::print_stderr)]
    pub fn apply(&self, enabled: bool) {
        let mut running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
        if enabled == running.is_some() {
            return;
        }
        self.failed.store(false, Ordering::Relaxed);
        self.content.store(false, Ordering::Relaxed);
        if let Some(open) = running.take() {
            open.stop.store(true, Ordering::Relaxed);
            let _ = open.thread.join();
            if let Some(path) = &self.path {
                remove_access_file(path, open.port);
            }
            return;
        }
        self.calls.store(0, Ordering::Relaxed);
        // the settings show this; `log_err` would end a debug build
        match self.open() {
            Ok(open) => *running = Some(open),
            Err(e) => {
                eprintln!("Could not set up the access for AI assistants: {e}");
                self.failed.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Allows the content of packets, as long as the door is open
    pub fn allow_content(&self, allowed: bool) {
        let running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
        self.content
            .store(allowed && running.is_some(), Ordering::Relaxed);
    }

    pub fn content_allowed(&self) -> bool {
        self.content.load(Ordering::Relaxed)
    }

    pub fn status(&self) -> Status {
        let open = self
            .running
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some();
        if open {
            Status::Active {
                calls: self.calls.load(Ordering::Relaxed),
            }
        } else if self.failed.load(Ordering::Relaxed) {
            Status::Failed
        } else {
            Status::Off
        }
    }

    fn open(&self) -> io::Result<Running> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| io::Error::from(ErrorKind::NotFound))?;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let token = random_hex();
        write_access_file(
            path,
            &AccessFile {
                port,
                token: token.clone(),
            },
        )?;
        let stop = Arc::new(AtomicBool::new(false));
        let door = Door {
            engine: self.engine.clone(),
            token,
            stop: stop.clone(),
            calls: self.calls.clone(),
            content: self.content.clone(),
            clients: Arc::default(),
            index: Arc::new(FilterIndex::with_values(hide_in_values)),
        };
        let thread = thread::Builder::new()
            .name("thread_mcp_access".to_string())
            .spawn(move || door.listen(&listener))?;
        Ok(Running { port, stop, thread })
    }
}

/// Writes the file for this user alone; it never shows half a content
fn write_access_file(path: &Path, content: &AccessFile) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let draft = path.with_extension("draft");
    let _ = fs::remove_file(&draft);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    // under Windows the folder of the settings belongs to the user
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(&draft)?;
    file.write_all(&serde_json::to_vec(content)?)?;
    drop(file);
    fs::rename(&draft, path)
}

/// Removes the file unless a window opened later has taken it over: the newest one answers
fn remove_access_file(path: &Path, port: u16) {
    let ours = fs::read(path)
        .ok()
        .and_then(|file| serde_json::from_slice::<AccessFile>(&file).ok())
        .is_some_and(|file| file.port == port);
    if ours {
        let _ = fs::remove_file(path);
    }
}

#[derive(Clone)]
struct Door {
    engine: Engine,
    token: String,
    stop: Arc<AtomicBool>,
    calls: Arc<AtomicU64>,
    content: Arc<AtomicBool>,
    clients: Arc<AtomicUsize>,
    /// The filters of the assistants, apart from those of the window
    index: Arc<FilterIndex>,
}

impl Door {
    fn listen(&self, listener: &TcpListener) {
        while !self.stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => self.welcome(stream),
                Err(_) => thread::sleep(POLL),
            }
        }
    }

    fn welcome(&self, stream: TcpStream) {
        if self.clients.fetch_add(1, Ordering::Relaxed) >= MAX_CLIENTS {
            self.clients.fetch_sub(1, Ordering::Relaxed);
            return;
        }
        let door = self.clone();
        let started = thread::Builder::new()
            .name("thread_mcp_client".to_string())
            .spawn(move || {
                door.serve(stream);
                door.clients.fetch_sub(1, Ordering::Relaxed);
            });
        if started.is_err() {
            self.clients.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Hello, challenge, one question, one answer
    fn serve(&self, mut stream: TcpStream) {
        // on some systems a connection inherits this from the listener
        if stream.set_nonblocking(false).is_err()
            || stream.set_write_timeout(Some(WRITE_TIMEOUT)).is_err()
        {
            return;
        }
        let Some(line) = read_line(&mut stream, MAX_REQUEST_BYTES, STEP_TIMEOUT) else {
            return;
        };
        let hello = serde_json::from_slice::<Hello>(&line).ok();
        let Some(hello) = hello.filter(|hello| is_nonce(&hello.nonce)) else {
            let _ = write_line(&mut stream, &Answer::Refused);
            return;
        };
        let nonce = random_hex();
        let challenge = Challenge {
            proof: window_proof(&self.token, &hello.nonce),
            nonce: nonce.clone(),
        };
        if write_line(&mut stream, &challenge).is_err() {
            return;
        }
        let Some(line) = read_line(&mut stream, MAX_REQUEST_BYTES, STEP_TIMEOUT) else {
            return;
        };
        let answer = self.answer(&line, &nonce);
        if !self.stop.load(Ordering::Relaxed) {
            let _ = write_line(&mut stream, &answer);
        }
    }

    fn answer(&self, line: &[u8], nonce: &str) -> Answer {
        let Ok(request) = serde_json::from_slice::<Request>(line) else {
            return Answer::Refused;
        };
        if !same(&request.proof, &client_proof(&self.token, nonce)) {
            return Answer::Refused;
        }
        self.calls.fetch_add(1, Ordering::Relaxed);
        let content = self.content.load(Ordering::Relaxed);
        tools::call(
            &self.engine,
            &self.index,
            content,
            &request.tool,
            &request.arguments,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use serde_json::{Value, json};

    use super::*;
    use crate::config::conf::Conf;
    use crate::engine::tests::{frame, write_pcap};
    use crate::mcp::protocol::{AskError, ask};
    use crate::mcp::tools::{CAPTURE_STATUS, FILTER_HELP, FOLLOW_STREAM, PACKETS};
    use crate::networking::types::capture_context::CaptureSourcePicklist;
    use std::net::Shutdown;

    struct Fixture {
        _runtime: tokio::runtime::Runtime,
        engine: Engine,
        dir: PathBuf,
        access: Access,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let engine = Engine::new(Conf::default(), runtime.handle().clone());
            let dir = std::env::temp_dir()
                .join(format!("openpacketmon-mcp-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            let access = Access::new(engine.clone(), Some(dir.join("access.json")));
            Self {
                _runtime: runtime,
                engine,
                dir,
                access,
            }
        }

        fn path(&self) -> PathBuf {
            self.dir.join("access.json")
        }

        fn file(&self) -> AccessFile {
            serde_json::from_slice(&fs::read(self.path()).unwrap()).unwrap()
        }

        fn connect(&self) -> TcpStream {
            TcpStream::connect((Ipv4Addr::LOCALHOST, self.file().port)).unwrap()
        }

        fn status(&self) -> Result<Value, AskError> {
            ask(&self.path(), CAPTURE_STATUS, json!({}))
        }

        /// What the window says to a first line, `None` if it just closes the connection
        fn answer_to(&self, line: &[u8]) -> Option<Answer> {
            let mut stream = self.connect();
            stream.write_all(line).unwrap();
            stream.write_all(b"\n").unwrap();
            let mut answer = Vec::new();
            stream.read_to_end(&mut answer).ok()?;
            serde_json::from_slice(&answer).ok()
        }

        /// Says hello and checks the challenge of the window
        fn hello(&self) -> (TcpStream, Challenge) {
            let mut stream = self.connect();
            let nonce = random_hex();
            write_line(
                &mut stream,
                &Hello {
                    nonce: nonce.clone(),
                },
            )
            .unwrap();
            let line = read_line(&mut stream, 1024, STEP_TIMEOUT).unwrap();
            let challenge: Challenge = serde_json::from_slice(&line).unwrap();
            assert_eq!(challenge.proof, window_proof(&self.file().token, &nonce));
            (stream, challenge)
        }

        /// What the window says to a question with this proof
        fn answer_with(&self, proof: impl FnOnce(&Challenge) -> String, tool: &str) -> Answer {
            let (mut stream, challenge) = self.hello();
            let request = Request {
                proof: proof(&challenge),
                tool: tool.to_owned(),
                arguments: Value::Null,
            };
            write_line(&mut stream, &request).unwrap();
            let line = read_line(&mut stream, MAX_REQUEST_BYTES, STEP_TIMEOUT).unwrap();
            serde_json::from_slice(&line).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            self.access.apply(false);
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn the_access_is_off_until_the_user_turns_it_on() {
        assert!(!Conf::default().mcp.enabled);
        let fixture = Fixture::new("off");
        assert_eq!(fixture.access.status(), Status::Off);
        assert!(!fixture.path().exists());
        assert_eq!(fixture.status(), Err(AskError::NotAvailable));
    }

    #[test]
    fn the_open_door_answers_and_counts() {
        let fixture = Fixture::new("open");
        fixture.access.apply(true);
        assert_eq!(fixture.access.status(), Status::Active { calls: 0 });
        assert_eq!(fixture.status().unwrap(), json!({ "capture": "none" }));
        assert_eq!(fixture.access.status(), Status::Active { calls: 1 });
    }

    #[test]
    #[cfg(unix)]
    fn only_the_user_can_read_the_token() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = Fixture::new("mode");
        fixture.access.apply(true);
        let mode = fs::metadata(fixture.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn a_closed_door_leaves_nothing_and_a_new_one_has_a_new_token() {
        let fixture = Fixture::new("closed");
        fixture.access.apply(true);
        let first = fixture.file();
        fixture.access.apply(false);
        assert_eq!(fixture.access.status(), Status::Off);
        assert!(!fixture.path().exists());
        assert_eq!(fixture.status(), Err(AskError::NotAvailable));
        // the port itself can take a moment while a parallel test starts a process, which holds
        // on to it until it runs, but nobody answers there any more
        write_access_file(&fixture.path(), &first).unwrap();
        assert!(matches!(
            fixture.status(),
            Err(AskError::NotAvailable | AskError::NoAnswer)
        ));
        fs::remove_file(fixture.path()).unwrap();

        fixture.access.apply(true);
        assert_ne!(fixture.file().token, first.token);
        assert!(fixture.status().is_ok());
    }

    #[test]
    fn a_window_leaves_the_file_of_a_newer_one_alone() {
        let fixture = Fixture::new("second");
        fixture.access.apply(true);
        let newer = Access::new(fixture.engine.clone(), Some(fixture.path()));
        newer.apply(true);
        let file = fixture.file();
        fixture.access.apply(false);
        assert_eq!(fixture.file().token, file.token);
        assert!(fixture.status().is_ok());
        newer.apply(false);
        assert!(!fixture.path().exists());
    }

    #[test]
    fn the_token_decides_and_is_never_sent() {
        let fixture = Fixture::new("token");
        fixture.access.apply(true);
        let token = fixture.file().token;
        let mut wrong = token.clone();
        wrong.replace_range(..1, if token.starts_with('0') { "1" } else { "0" });
        let wrong_proofs: [&dyn Fn(&Challenge) -> String; 6] = [
            &|c| client_proof(&wrong, &c.nonce),
            &|_| client_proof(&token, &random_hex()),
            // the proof of the window itself, or one it would give for this nonce
            &|c| c.proof.clone(),
            &|c| window_proof(&token, &c.nonce),
            &|_| token.clone(),
            &|_| String::new(),
        ];
        for proof in wrong_proofs {
            assert_eq!(fixture.answer_with(proof, CAPTURE_STATUS), Answer::Refused);
        }
        // the question of earlier versions, with the token in it
        let old = json!({ "token": token, "tool": CAPTURE_STATUS }).to_string();
        assert_eq!(fixture.answer_to(old.as_bytes()), Some(Answer::Refused));
        assert_eq!(fixture.access.status(), Status::Active { calls: 0 });

        let right = fixture.answer_with(|c| client_proof(&token, &c.nonce), CAPTURE_STATUS);
        assert_eq!(right, Answer::Ok(json!({ "capture": "none" })));
        // every challenge has a nonce of its own
        assert_ne!(fixture.hello().1.nonce, fixture.hello().1.nonce);
    }

    /// Takes a port as if a window had left it, and notes every line it gets
    fn stranger(
        reply: impl FnOnce(&Hello) -> String + Send + 'static,
    ) -> (u16, JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let thread = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut lines = Vec::new();
            let line = read_line(&mut stream, MAX_REQUEST_BYTES, STEP_TIMEOUT).unwrap();
            let hello: Hello = serde_json::from_slice(&line).unwrap();
            lines.push(String::from_utf8(line).unwrap());
            stream.write_all(reply(&hello).as_bytes()).unwrap();
            stream.write_all(b"\n").unwrap();
            while let Some(line) = read_line(&mut stream, MAX_REQUEST_BYTES, STEP_TIMEOUT) {
                lines.push(String::from_utf8_lossy(&line).into_owned());
            }
            let _ = stream.shutdown(Shutdown::Both);
            lines
        });
        (port, thread)
    }

    #[test]
    fn a_stranger_at_the_port_gets_neither_token_nor_question() {
        let fixture = Fixture::new("stranger");
        let token = random_hex();
        let replies: [fn(&Hello) -> String; 4] = [
            // a proof with another token
            |hello| {
                let challenge = Challenge {
                    nonce: random_hex(),
                    proof: window_proof(&random_hex(), &hello.nonce),
                };
                serde_json::to_string(&challenge).unwrap()
            },
            // its own nonce sent back as proof
            |hello| json!({ "nonce": random_hex(), "proof": hello.nonce }).to_string(),
            |_| json!({ "ok": { "capture": "none" } }).to_string(),
            |_| "HTTP/1.1 400 Bad Request".to_owned(),
        ];
        for reply in replies {
            let (port, thread) = stranger(reply);
            write_access_file(
                &fixture.path(),
                &AccessFile {
                    port,
                    token: token.clone(),
                },
            )
            .unwrap();
            let arguments = json!({ "filter": "http.request.uri contains \"private\"" });
            assert_eq!(
                ask(&fixture.path(), PACKETS, arguments),
                Err(AskError::Stranger)
            );
            let lines = thread.join().unwrap();
            assert_eq!(lines.len(), 1, "{lines:?}");
            assert!(!lines[0].contains(&token) && !lines[0].contains("private"));
            fs::remove_file(fixture.path()).unwrap();
        }
    }

    #[test]
    fn wrong_questions_get_no_data() {
        let fixture = Fixture::new("wrong");
        fixture.access.apply(true);
        let short = json!({ "nonce": "00" }).to_string();
        for line in [
            &b"GET / HTTP/1.1"[..],
            b"{}",
            b"\xff\xfe",
            b"[[[[",
            short.as_bytes(),
        ] {
            assert_eq!(fixture.answer_to(line), Some(Answer::Refused));
        }
        let long = vec![b'x'; MAX_REQUEST_BYTES + 1];
        assert_eq!(fixture.answer_to(&long), None);

        let token = fixture.file().token;
        assert_eq!(
            fixture.answer_with(|c| client_proof(&token, &c.nonce), "no_such_tool"),
            Answer::Error("unknown tool".to_owned())
        );
        // a question that is no question, after the right hello
        let (mut stream, _) = fixture.hello();
        stream.write_all(b"{}\n").unwrap();
        let line = read_line(&mut stream, 1024, STEP_TIMEOUT).unwrap();
        assert_eq!(
            serde_json::from_slice::<Answer>(&line).unwrap(),
            Answer::Refused
        );
    }

    #[test]
    fn the_arguments_reach_the_tool() {
        let fixture = Fixture::new("arguments");
        fixture.access.apply(true);
        let help = ask(&fixture.path(), FILTER_HELP, json!({ "connections": true })).unwrap();
        assert!(help["examples"].to_string().contains("program contains"));
        let wrong = ask(&fixture.path(), FILTER_HELP, json!({ "connections": 1 }));
        assert!(matches!(wrong, Err(AskError::Tool(_))));
    }

    #[test]
    fn the_content_of_packets_needs_its_own_allowance_and_ends_with_the_door() {
        let fixture = Fixture::new("content");
        let stream =
            |fixture: &Fixture| ask(&fixture.path(), FOLLOW_STREAM, json!({ "number": 1 }));
        fixture.access.allow_content(true);
        // a closed door allows nothing
        assert!(!fixture.access.content_allowed());
        fixture.access.apply(true);
        assert!(!fixture.access.content_allowed());
        let Err(AskError::Tool(off)) = stream(&fixture) else {
            panic!("the content was not refused");
        };
        assert!(off.contains("Allow packet contents"), "{off}");

        fixture.access.allow_content(true);
        assert!(fixture.access.content_allowed());
        let Err(AskError::Tool(on)) = stream(&fixture) else {
            panic!("there is no capture");
        };
        assert!(on.contains("No capture"), "{on}");

        fixture.access.apply(false);
        fixture.access.apply(true);
        assert!(!fixture.access.content_allowed());
    }

    #[test]
    fn idle_clients_cannot_hold_the_door() {
        let fixture = Fixture::new("idle");
        fixture.access.apply(true);
        let idle: Vec<TcpStream> = (0..MAX_CLIENTS).map(|_| fixture.connect()).collect();
        thread::sleep(POLL * 3);
        let mut late = fixture.connect();
        let mut nothing = Vec::new();
        late.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        assert_eq!(late.read_to_end(&mut nothing).unwrap(), 0);
        drop(idle);
        thread::sleep(POLL * 3);
        assert!(fixture.status().is_ok());
    }

    #[test]
    fn a_flood_of_connections_passes_and_the_door_answers_again() {
        let fixture = Fixture::new("flood");
        fixture.access.apply(true);
        let port = fixture.file().port;
        let flood: Vec<_> = (0..8)
            .map(|_| {
                thread::spawn(move || {
                    for _ in 0..100 {
                        if let Ok(mut stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, port)) {
                            let _ = stream.write_all(b"{\"nonce\":");
                        }
                    }
                })
            })
            .collect();
        // questions in between get an answer or are turned away, nothing else
        for _ in 0..20 {
            assert!(matches!(fixture.status(), Ok(_) | Err(AskError::NoAnswer)));
        }
        for thread in flood {
            thread.join().unwrap();
        }
        thread::sleep(POLL * 3);
        assert!(fixture.status().is_ok());
    }

    #[test]
    fn a_slow_client_is_cut_off() {
        let fixture = Fixture::new("slow");
        fixture.access.apply(true);
        let mut stream = fixture.connect();
        let hello = serde_json::to_vec(&Hello {
            nonce: random_hex(),
        })
        .unwrap();
        let started = std::time::Instant::now();
        let mut cut = false;
        for byte in hello {
            if stream.write_all(&[byte]).is_err() {
                cut = true;
                break;
            }
            let mut probe = [0; 1];
            stream
                .set_read_timeout(Some(Duration::from_millis(250)))
                .unwrap();
            if matches!(stream.read(&mut probe), Ok(0) | Err(_)) && started.elapsed() > STEP_TIMEOUT
            {
                cut = true;
                break;
            }
        }
        assert!(cut, "the slow hello went through");
        assert!(started.elapsed() < STEP_TIMEOUT * 2);
        assert!(fixture.status().is_ok());
    }

    #[test]
    fn a_huge_question_after_the_hello_gets_nothing() {
        let fixture = Fixture::new("huge");
        fixture.access.apply(true);
        let (mut stream, _) = fixture.hello();
        // the window stops reading, so the end of this may not be taken any more
        let _ = stream.write_all(&vec![b'x'; MAX_REQUEST_BYTES + 1]);
        let _ = stream.write_all(b"\n");
        let mut answer = Vec::new();
        let _ = stream.read_to_end(&mut answer);
        assert!(answer.is_empty(), "{}", String::from_utf8_lossy(&answer));
        assert_eq!(fixture.access.status(), Status::Active { calls: 0 });
    }

    #[test]
    fn turning_the_access_off_and_on_during_questions() {
        let fixture = Fixture::new("toggle");
        fixture.access.apply(true);
        let path = fixture.path();
        let stop = Arc::new(AtomicBool::new(false));
        let askers: Vec<_> = (0..3)
            .map(|_| {
                let path = path.clone();
                let stop = stop.clone();
                thread::spawn(move || {
                    let mut answers = 0;
                    while !stop.load(Ordering::Relaxed) {
                        // a window closing under a question is no window, or no answer; a
                        // new window can get the port of the old one, then the old token fails
                        match ask(&path, CAPTURE_STATUS, json!({})) {
                            Ok(_) => answers += 1,
                            Err(
                                AskError::NotAvailable
                                | AskError::NoAnswer
                                | AskError::Refused
                                | AskError::Stranger,
                            ) => {}
                            Err(AskError::Tool(text)) => panic!("{text}"),
                        }
                    }
                    answers
                })
            })
            .collect();
        for _ in 0..20 {
            fixture.access.allow_content(true);
            thread::sleep(POLL);
            fixture.access.apply(false);
            assert!(!fixture.access.content_allowed());
            fixture.access.apply(true);
        }
        thread::sleep(POLL * 4);
        stop.store(true, Ordering::Relaxed);
        let answers: usize = askers.into_iter().map(|t| t.join().unwrap()).sum();
        assert!(answers > 0);
        assert!(!fixture.access.content_allowed());
        assert!(fixture.status().is_ok());
    }

    #[test]
    fn the_status_tells_the_source_without_its_folder() {
        let fixture = Fixture::new("capture");
        fs::create_dir_all(&fixture.dir).unwrap();
        let file = fixture.dir.join("sample.pcap");
        let packets: Vec<_> = (0..4)
            .map(|i| {
                let bytes = frame([192, 168, 1, 10], [192, 168, 1, 1], 50_000, 53, false, 30);
                (1_000 + i, bytes)
            })
            .collect();
        write_pcap(&file, &packets);
        {
            let mut state = fixture.engine.lock();
            state.set_capture_source(CaptureSourcePicklist::File);
            state.set_pcap_import(file.to_string_lossy().to_string());
        }
        assert!(fixture.engine.start());
        for _ in 0..500 {
            if fixture.engine.lock().traffic_history.no_more_packets() {
                break;
            }
            thread::sleep(POLL / 5);
        }
        fixture.access.apply(true);
        let status = fixture.status().unwrap();
        assert_eq!(status["capture"], "finished");
        assert_eq!(
            status["source"],
            json!({ "kind": "file", "name": "sample.pcap" })
        );
        assert_eq!(status["packets"], 4);
        assert!(status["bytes"].as_u64().unwrap() > 0);
        assert_eq!(status["stored_packets"]["count"], 4);
        assert!(!status.to_string().contains(&*fixture.dir.to_string_lossy()));
        fixture.engine.stop();
    }
}
