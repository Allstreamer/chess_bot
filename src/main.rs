use shakmaty::uci::UciMove;
use shakmaty::zobrist::Zobrist64;
use shakmaty::{Chess, Color, Position};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

mod engine;
use engine::Searcher;

use crate::engine::TranspositionInformation;

#[rustfmt::skip]
mod engine_hyperparams;

/// Holds the engine's state, primarily the current board position.
struct EngineState {
    pos: Chess,
    is_thinking: Arc<AtomicBool>,
    thinking_thread: Option<thread::JoinHandle<()>>,
    nickname: String,
}

impl EngineState {
    fn new() -> Self {
        Self {
            pos: Chess::default(),
            is_thinking: Arc::new(AtomicBool::new(false)),
            thinking_thread: None,
            nickname: "AllRustBot".to_owned(),
        }
    }

    /// Parses a line from the GUI and acts on it.
    fn handle_command(&mut self, line: &str) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if let Some(&command) = tokens.first() {
            match command {
                "position" => self.handle_position(&tokens[1..]),
                "go" => self.handle_go(&tokens[1..]),
                "isready" => self.handle_isready(),
                "uci" => self.handle_uci(),
                "quit" => self.handle_quit(),
                "stop" => self.handle_stop(),
                "ucinewgame" => self.handle_ucinewgame(),
                "setoption" => self.handle_setoption(&tokens[1..]),
                // The spec says to ignore unknown commands.
                _ => {}
            }
        }
    }

    /// Handles the "setoption" command to change engine parameters.
    fn handle_setoption(&mut self, tokens: &[&str]) {
        // tokens slice starts after "setoption", e.g., ["name", "nick", "value", "new_name"]
        if tokens.first() != Some(&"name") {
            return; // Invalid format
        }

        let value_pos = tokens.iter().position(|&s| s == "value");

        if let Some(value_idx) = value_pos {
            // Option with a value
            if value_idx > 1 {
                // Ensure there is a name between "name" and "value"
                let option_name = tokens[1..value_idx].join(" ");
                let option_value = tokens[value_idx + 1..].join(" ");

                if option_name.eq_ignore_ascii_case("nick") {
                    self.nickname = option_value;
                }
                // Handle other options with values here
            }
        }
        // No "else" branch needed for button types yet, as we only have "nick".
    }

    /// Responds to the "uci" command by identifying the engine and sending supported options.
    fn handle_uci(&self) {
        println!("id name {}", self.nickname);
        println!("id author All");
        println!("option name nick type string default {}", self.nickname);
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
    fn handle_go(&mut self, tokens: &[&str]) {
        if self.is_thinking.load(Ordering::SeqCst) {
            // Ignore 'go' if already thinking, as per UCI spec.
            return;
        }
        self.is_thinking.store(true, Ordering::SeqCst);

        let thinking_start_time = Instant::now();

        let mut wtime: Option<u64> = None;
        let mut btime: Option<u64> = None;

        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                "wtime" => {
                    if let Some(val_str) = tokens.get(i + 1) {
                        if let Ok(time) = val_str.parse::<u64>() {
                            wtime = Some(time);
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "btime" => {
                    if let Some(val_str) = tokens.get(i + 1) {
                        if let Ok(time) = val_str.parse::<u64>() {
                            btime = Some(time);
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                // TODO: Parse other parameters like "depth", "nodes", "movetime", "infinite"
                _ => {
                    // Ignore unknown or unhandled tokens
                    i += 1;
                }
            }
        }

        // Clone necessary state for the thinking thread
        let position_to_search = self.pos.clone();
        let is_thinking_clone = Arc::clone(&self.is_thinking);
        let is_thinking_clone_b = Arc::clone(&self.is_thinking);

        let time = if position_to_search.turn() == Color::White {
            wtime
        } else {
            btime
        };

        let target_think_time = Duration::from_millis(match time {
            Some(available_time) => available_time / 20,
            None => 100,
        });

        let handle = thread::spawn(move || {
            let mut transposition_table: HashMap<Zobrist64, TranspositionInformation> =
                HashMap::new();
            let mut searcher = Searcher::new(
                &position_to_search,
                1,
                &is_thinking_clone_b,
                None,
                &mut transposition_table,
            );
            let mut best_move = searcher.next_move();
            let mut depth: u64 = 2;
            loop {
                if !is_thinking_clone_b.load(Ordering::SeqCst) {
                    break;
                }
                let mut searcher = Searcher::new(
                    &position_to_search,
                    depth,
                    &is_thinking_clone_b,
                    Some(&best_move),
                    &mut transposition_table,
                );
                best_move = searcher.next_move();
                depth += 1;
            }

            let time_taken = thinking_start_time.elapsed();
            println!("info time {}", time_taken.as_millis());

            // A real engine might also send a ponder move.
            let best_move_response = format!(
                "bestmove {}",
                best_move.to_uci(shakmaty::CastlingMode::Standard)
            );
            println!("{best_move_response}");
        });

        let _timer_handle = thread::spawn(move || {
            thread::sleep(target_think_time);
            is_thinking_clone.store(false, Ordering::SeqCst);
        });

        self.thinking_thread = Some(handle);
    }

    /// Prepares the engine for a new game.
    fn handle_ucinewgame(&mut self) {
        self.pos = Chess::default();
    }

    /// Handles the "stop" command.
    fn handle_stop(&self) {
        self.is_thinking.store(false, Ordering::SeqCst);
    }

    /// Handles the "quit" command.
    fn handle_quit(&self) {
        std::process::exit(0);
    }
}

fn main() {
    let mut engine_state = EngineState::new();
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let trimed_line = line
            .expect("Failed to read line from stdin")
            .trim()
            .to_owned();
        if trimed_line.is_empty() {
            continue;
        }

        engine_state.handle_command(&trimed_line);

        // Ensure every command response is sent immediately.
        io::stdout().flush().expect("Failed to flush stdout");
    }
}
