use shakmaty::uci::UciMove;
use shakmaty::{Chess, Position};
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;

mod engine;
use engine::next_move;

mod engine_hyperparams;

/// Holds the engine's state, primarily the current board position.
struct EngineState {
    pos: Chess,
    is_thinking: Arc<AtomicBool>,
    thinking_thread: Option<thread::JoinHandle<()>>,
}

impl EngineState {
    /// Creates a new EngineState with the default starting chess position.
    fn new() -> Self {
        EngineState {
            pos: Chess::default(),
            is_thinking: Arc::new(AtomicBool::new(false)),
            thinking_thread: None,
        }
    }

    /// The main command handler. Parses a line from the GUI and acts on it.
    fn handle_command(&mut self, line: &str) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if let Some(&command) = tokens.first() {
            match command {
                "uci" => self.handle_uci(),
                "isready" => self.handle_isready(),
                "position" => self.handle_position(&tokens[1..]),
                "go" => self.handle_go(),
                "quit" => self.handle_quit(),
                "stop" => self.handle_stop(),
                "ucinewgame" => self.handle_ucinewgame(),
                // Other commands can be implemented as needed.
                // The spec says to ignore unknown commands.
                _ => {}
            }
        }
    }

    /// Responds to the "uci" command by identifying the engine and sending supported options.
    fn handle_uci(&self) {
        println!("id name AllRustBot");
        println!("id author All");
        // Example of sending an option. A real engine would list all its options here.
        // println!("option name Hash type spin default 16 min 1 max 1024");
        println!("uciok");
    }

    /// Responds to "isready" to synchronize with the GUI.
    fn handle_isready(&mut self) {
        // If a thinking thread is finished, join it to clean up resources.
        if let Some(handle) = self.thinking_thread.take() {
            if handle.is_finished() {
                handle.join().expect("Failed to join thinking thread");
            } else {
                // If not finished, put it back.
                self.thinking_thread = Some(handle);
            }
        }
        println!("readyok");
    }

    /// Sets up the board based on a FEN string or startpos, and a series of moves.
    fn handle_position(&mut self, tokens: &[&str]) {
        let mut current_pos: Chess;
        let moves_start_index;

        if tokens.first() == Some(&"startpos") {
            current_pos = Chess::default();
            moves_start_index = tokens.iter().position(|&r| r == "moves");
        } else if tokens.first() == Some(&"fen") {
            moves_start_index = tokens.iter().position(|&r| r == "moves");
            let fen_tokens = if let Some(msi) = moves_start_index {
                &tokens[1..msi]
            } else {
                &tokens[1..]
            };
            let fen_str = fen_tokens.join(" ");
            let fen: shakmaty::fen::Fen = fen_str.parse().expect("Failed to parse FEN");
            current_pos = fen
                .into_position(shakmaty::CastlingMode::Standard)
                .expect("Invalid FEN");
        } else {
            // Invalid position command
            return;
        }

        if let Some(msi) = moves_start_index {
            for move_str in &tokens[msi + 1..] {
                let uci_move: UciMove = move_str.parse().expect("Invalid UCI move");
                if let Ok(m) = uci_move.to_move(&current_pos) {
                    current_pos.play_unchecked(m);
                }
            }
        }

        self.pos = current_pos;
    }

    /// Starts calculating the best move for the current position.
    fn handle_go(&mut self) {
        if self.is_thinking.load(Ordering::SeqCst) {
            // Ignore 'go' if already thinking, as per UCI spec.
            return;
        }

        self.is_thinking.store(true, Ordering::SeqCst);

        // Clone necessary state for the thinking thread
        let position_to_search = self.pos.clone();
        let is_thinking_clone = self.is_thinking.clone();

        let handle = thread::spawn(move || {
            // This is where the engine "thinks". It calls our core logic function.
            let best_move = next_move(&position_to_search, 4);

            // Report the result back to the GUI.
            // A real engine might also send a ponder move.
            println!(
                "bestmove {}",
                best_move.to_uci(shakmaty::CastlingMode::Standard)
            );

            // Signal that thinking is finished.
            is_thinking_clone.store(false, Ordering::SeqCst);
        });

        self.thinking_thread = Some(handle);
    }

    /// Prepares the engine for a new game.
    fn handle_ucinewgame(&mut self) {
        // For a simple engine, we can just reset the position to the start.
        // A more complex engine might clear hash tables or other game-specific data.
        self.pos = Chess::default();
    }

    /// Handles the "stop" command.
    fn handle_stop(&mut self) {
        // NOTE: The provided `next_move` function is a black box and blocking.
        // Therefore, we can't truly interrupt it. A real engine would have an
        // iterative search that checks an atomic 'stop_requested' flag in its main loop.
        // When our thinking thread finishes, it will print 'bestmove' on its own.
        // This handler is here to conform to the protocol; the GUI will see 'bestmove'
        // as the acknowledgement that the search has stopped.
        // In a more complex engine, you would set a flag here:
        // self.stop_search.store(true, Ordering::SeqCst);
    }

    /// Handles the "quit" command.
    fn handle_quit(&self) {
        // We could send a final message or clean up, but exiting is sufficient.
        std::process::exit(0);
    }
}

/// A utility function to ensure messages are sent immediately to the GUI.
fn flush_stdout() {
    io::stdout().flush().expect("Failed to flush stdout");
}

fn log_to_file(message: &str) -> io::Result<()> {
    // Open the file with options to create it if it doesn't exist,
    // to append to it, and to write to it.
    let mut file = OpenOptions::new()
        .create(true) // Create the file if it doesn't exist.
        .append(true) // Append to the end of the file.
        .open("log.txt")?; // The file to open. The '?' operator will propagate any errors.

    // The writeln! macro writes the formatted string to the file,
    // automatically appending a newline.
    writeln!(file, "{}", message)?;

    // If both operations succeed, return Ok.
    Ok(())
}

fn main() {
    let mut engine_state = EngineState::new();
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = line
            .expect("Failed to read line from stdin")
            .trim()
            .to_string();
        if line.is_empty() {
            continue;
        }

        // Optional: log commands to a file for debugging
        log_to_file(&format!("GUI -> Engine: {}", line)).unwrap();

        engine_state.handle_command(&line);

        // Ensure every command response is sent immediately.
        flush_stdout();
    }
}
