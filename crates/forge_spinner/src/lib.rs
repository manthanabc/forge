use std::io::{self, Write};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use rand::seq::IndexedRandom;

/// Commands for the spinner background thread
enum Cmd {
    Write(String),
    Flush(mpsc::Sender<()>),
    Stop,
}

/// Manages spinner functionality for the UI
#[derive(Default)]
pub struct SpinnerManager {
    tx: Option<mpsc::Sender<Cmd>>,  // channel to spinner thread
    handle: Option<JoinHandle<()>>, // spinner thread handle
    message: Option<String>,        // current status text
    running: bool,
}

impl SpinnerManager {
    pub fn new() -> Self {
        Self::default()
    }
    /// Start the spinner with a message (API preserved).
    /// Behavior mirrors markdown_renderer: draws an in-place spinner line,
    /// writes print above it, hides cursor, and handles Ctrl-C.
    pub fn start(&mut self, message: Option<&str>) -> Result<()> {
        // Stop any existing spinner first (preserves API behavior)
        self.stop(None)?;
        enable_raw_mode()?;

        // Choose default word if none provided (keeps prior semantics)
        let words = [
            "Thinking",
            "Processing",
            "Analyzing",
            "Forging",
            "Researching",
            "Synthesizing",
            "Reasoning",
            "Contemplating",
        ];
        let word = match message {
            None => words.choose(&mut rand::rng()).unwrap_or(&words[0]),
            Some(msg) => msg,
        };
        let status_text = word.to_string();
        self.message = Some(status_text.clone());

        let (tx, rx) = mpsc::channel::<Cmd>();

        let handle = thread::spawn(move || {
            let spinner_frames: [&str; 8] = ["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"];
            let mut idx: usize = 0;
            let tick = Duration::from_millis(80);
            let mut last = std::time::Instant::now();

            // Hide cursor and draw initial spinner line
            print!("\x1b[?25l");
            print!("\r{}  {}", spinner_frames[idx], status_text);
            let _ = io::stdout().flush();

            let mut keep_running = true;
            let mut ctrl_c = false;
            while keep_running {
                // Handle incoming commands quickly
                match rx.recv_timeout(Duration::from_millis(5)) {
                    Ok(Cmd::Write(s)) => {
                        // Print above spinner then redraw spinner line
                        // execute!(self.wwriter, MoveToColumn(0));
                        print!("\r\x1b[2K");
                        if !s.ends_with('\n') {
                            print!("{}\n", s);
                        } else {
                            print!("{}", s);
                        }
                        print!("\r{}  {}", spinner_frames[idx], status_text);
                        let _ = io::stdout().flush();
                    }
                    Ok(Cmd::Flush(ack)) => {
                        let _ = ack.send(());
                    }
                    Ok(Cmd::Stop) => {
                        keep_running = false;
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        keep_running = false;
                        continue;
                    }
                }

                // Drain input; capture Ctrl-C and request shutdown
                while event::poll(Duration::from_millis(0)).unwrap_or(false) {
                    match event::read() {
                        Ok(Event::Key(key)) => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                if let KeyCode::Char('c') | KeyCode::Char('C') = key.code {
                                    ctrl_c = true;
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    }
                }

                if keep_running && last.elapsed() >= tick {
                    idx = (idx + 1) % spinner_frames.len();
                    print!("\r{}  {}", spinner_frames[idx], status_text);
                    let _ = io::stdout().flush();
                    last = std::time::Instant::now();
                }

                if ctrl_c {
                    keep_running = false;
                }
            }

            // Cleanup: clear line and show cursor
            print!("\r\x1b[2K");
            print!("\x1b[?25h");
            let _ = io::stdout().flush();

            if ctrl_c {
                // Exit with 130 to emulate SIGINT after cleanup
                std::process::exit(130);
            }
        });

        self.tx = Some(tx);
        self.handle = Some(handle);
        self.running = true;
        Ok(())
    }

    /// Stop the active spinner if any (API preserved).
    pub fn stop(&mut self, message: Option<String>) -> Result<()> {
        // Signal spinner thread to stop and join it
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(Cmd::Stop);
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        // Restore terminal mode after spinner thread cleanup
        let _ = disable_raw_mode();

        // Print trailing message if provided
        if let Some(msg) = message {
            println!("{}", msg);
        }

        self.running = false;
        self.message = None;
        Ok(())
    }

    pub fn write_ln(&mut self, message: impl ToString) -> Result<()> {
        let s = message.to_string();
        if let Some(tx) = &self.tx {
            // Write above spinner while it continues running
            let _ = tx.send(Cmd::Write(s));
        } else {
            println!("b{}", s);
        }
        Ok(())
    }
}
