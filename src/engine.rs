use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
};

use shakmaty::{
    Chess, Color, Move, Outcome, Position, Role,
    zobrist::{Zobrist64, ZobristHash},
};

use crate::engine_hyperparams::{self, NEGATIVE_INFINITY, POSITIVE_INFINITY};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranspositionHashType {
    Exact,
    Alpha,
    Beta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranspositionInformation {
    depth: u64,
    value: i64,
    best_move: Option<Move>,
    transposition_type: TranspositionHashType,
}

pub struct Searcher<'a> {
    position: &'a Chess,
    target_depth: u64,
    is_thinking: &'a Arc<AtomicBool>,
    last_best_move: Option<&'a Move>,
    transposition_table: &'a mut HashMap<Zobrist64, TranspositionInformation>,
    searched_nodes: u64,
}

impl<'a> Searcher<'a> {
    pub fn new(
        position: &'a Chess,
        target_depth: u64,
        is_thinking: &'a Arc<AtomicBool>,
        last_best_move: Option<&'a Move>,
        transposition_table: &'a mut HashMap<Zobrist64, TranspositionInformation>,
    ) -> Self {
        Self {
            position,
            target_depth,
            is_thinking,
            last_best_move,
            transposition_table,
            searched_nodes: 0,
        }
    }

    /// Entry point for the chess engine to search for the best move.
    pub fn next_move(&mut self) -> Move {
        let mut legal_moves = self.position.legal_moves();
        legal_moves.sort_by_key(|move_to_score| {
            quick_score_move_for_sort(move_to_score, self.position, self.last_best_move)
        });

        // Find the move that maximizes the evaluation (piece count)
        let mut best_move = None;
        let mut alpha = NEGATIVE_INFINITY;
        let beta = POSITIVE_INFINITY;

        for legal_move in &legal_moves {
            let mut new_position = self.position.clone();
            new_position.play_unchecked(*legal_move);
            let score = -self.negamax(&new_position, self.target_depth - 1, -beta, -alpha);
            if score > alpha {
                alpha = score;
                best_move = Some(*legal_move);
            }
            if !self.is_thinking.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
        }

        println!(
            "info depth {} score cp {alpha} nodes {}",
            self.target_depth, self.searched_nodes
        );
        best_move.expect("No legal moves found")
    }

    fn negamax(&mut self, position: &Chess, depth: u64, mut alpha: i64, beta: i64) -> i64 {
        let mut transposition_type = TranspositionHashType::Alpha;
        let zobrist_hash = position.zobrist_hash::<Zobrist64>(shakmaty::EnPassantMode::Legal);
        let mut best_cached_move = None;

        match probe_hash(self.transposition_table, zobrist_hash, depth, alpha, beta) {
            HashProbeOption::Some(val) => {
                return val;
            }
            HashProbeOption::Move(mv) => {
                best_cached_move = Some(mv);
            }
            _ => {}
        }

        self.searched_nodes += 1;

        if depth == 0
            || position.is_game_over()
            || !self.is_thinking.load(std::sync::atomic::Ordering::SeqCst)
        {
            let val = self.quiesce(position, alpha, beta);
            record_hash(
                self.transposition_table,
                zobrist_hash,
                depth,
                val,
                TranspositionHashType::Exact,
                None,
            );
            return val;
        }

        if depth >= 3
            && !position.checkers().any()
            && let Ok(null_pos) = position.clone().swap_turn()
        {
            // Search with reduced depth (typically depth - 3)
            let null_score = -self.negamax(&null_pos, depth - 3, -beta, -beta + 1);

            // If even doing nothing beats beta, we can prune
            if null_score >= beta {
                return beta;
            }
        }

        let mut legal_moves = position.legal_moves();
        legal_moves.sort_by_key(|move_to_score| {
            quick_score_move_for_sort(move_to_score, position, best_cached_move.as_ref())
        });
        let mut best_move = None;

        for m in legal_moves {
            let mut new_pos = position.clone();
            new_pos.play_unchecked(m);

            let score = -self.negamax(&new_pos, depth - 1, -beta, -alpha);

            if score >= beta {
                record_hash(
                    self.transposition_table,
                    zobrist_hash,
                    depth,
                    beta,
                    TranspositionHashType::Beta,
                    Some(m),
                );
                return beta;
            }
            if score > alpha {
                transposition_type = TranspositionHashType::Exact;
                alpha = score;
                best_move = Some(m);
            }
        }

        record_hash(
            self.transposition_table,
            zobrist_hash,
            depth,
            alpha,
            transposition_type,
            best_move,
        );
        alpha
    }

    fn quiesce(&mut self, position: &Chess, mut alpha: i64, beta: i64) -> i64 {
        self.searched_nodes += 1;

        let static_eval = evaluate(position);

        // Stand Pat
        let mut best_value = static_eval;
        if best_value >= beta {
            return best_value;
        }
        if best_value > alpha {
            alpha = best_value;
        }

        // Only consider capture moves for quiescence
        let mut capture_moves: Vec<Move> = position
            .legal_moves()
            .into_iter()
            .filter(|m| m.capture().is_some())
            .collect();

        // Optionally, sort captures by MVV-LVA or similar
        capture_moves.sort_by_key(|m| {
            // Most Valuable Victim - Least Valuable Attacker
            -get_piece_base_score(m.capture().unwrap()) + get_piece_base_score(m.role())
        });

        for m in capture_moves {
            let mut new_pos = position.clone();
            new_pos.play_unchecked(m);

            let score = -self.quiesce(&new_pos, -beta, -alpha);

            if score >= beta {
                return score;
            }
            if score > best_value {
                best_value = score;
            }
            if score > alpha {
                alpha = score;
            }
        }

        best_value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HashProbeOption {
    Some(i64),
    Move(Move),
    None,
}

fn probe_hash(
    transposition_table: &HashMap<Zobrist64, TranspositionInformation>,
    zobrist_hash: Zobrist64,
    depth: u64,
    alpha: i64,
    beta: i64,
) -> HashProbeOption {
    let info_option = transposition_table.get(&zobrist_hash);

    if let Some(info) = info_option {
        if info.depth >= depth {
            if info.transposition_type == TranspositionHashType::Exact {
                return HashProbeOption::Some(info.value);
            }
            if (info.transposition_type == TranspositionHashType::Alpha) && (info.value <= alpha) {
                return HashProbeOption::Some(alpha);
            }
            if (info.transposition_type == TranspositionHashType::Beta) && (info.value >= beta) {
                return HashProbeOption::Some(beta);
            }
        }
        //  Tell move sort to search best move from last gen first
        if let Some(best_move) = info.best_move {
            return HashProbeOption::Move(best_move);
        }
    }

    HashProbeOption::None
}

fn record_hash(
    transposition_table: &mut HashMap<Zobrist64, TranspositionInformation>,
    zobrist_hash: Zobrist64,
    depth: u64,
    value: i64,
    transposition_type: TranspositionHashType,
    best_move: Option<Move>,
) {
    transposition_table.insert(
        zobrist_hash,
        TranspositionInformation {
            depth,
            value,
            transposition_type,
            best_move,
        },
    );
}

/// Higher result is a better move
fn quick_score_move_for_sort(
    move_to_score: &Move,
    position: &Chess,
    last_best_move: Option<&Move>,
) -> i64 {
    let mut score = 0;

    if let Some(last_move) = last_best_move {
        // If the move is the same as the last best move, give it a higher score
        if move_to_score == last_move {
            score += 10000; // Arbitrary high value to prioritize this move
        }
    }

    // Prioritize moves that capture high value pieces with low value pieces
    if let Some(captured_piece) = move_to_score.capture() {
        score += 10 * get_piece_base_score(captured_piece);
    }

    // Filter up Promotions
    if let Some(new_piece) = move_to_score.promotion() {
        score += get_piece_base_score(new_piece);
    }

    if position
        .board()
        .attacks_to(
            move_to_score.to(),
            position.turn().other(),
            position.board().occupied(),
        )
        .any()
    {
        score -= get_piece_base_score(move_to_score.role());
    }

    // Reverse order since rust sorts moves from lowest score to highest score
    -score
}

/// Calculates a chess position's score from the players's perspective.
/// A positive score means the player is ahead; a negative score means the opponent is ahead.
fn evaluate(position: &Chess) -> i64 {
    let mut total_score = 0;
    let current_player_color = position.turn();

    if position.is_game_over() {
        return match position.outcome() {
            Some(Outcome::Decisive { winner }) => {
                if winner == current_player_color {
                    engine_hyperparams::MATE_SCORE
                } else {
                    -engine_hyperparams::MATE_SCORE // Being checkmated is the worst outcome
                }
            }
            _ => 0, // Any other outcome (stalemate, etc.) is neutral
        };
    }

    let board = position.board();

    let piece_count = board.iter().len();
    for (square, piece) in board {
        let mut tmp_score = get_piece_base_score(piece.role);

        let piece_pos = if piece.color == Color::White {
            square.flip_vertical().to_usize()
        } else {
            square.to_usize()
        };
        tmp_score += match piece.role {
            Role::Pawn => engine_hyperparams::PAWN_PST[piece_pos],
            Role::Knight => engine_hyperparams::KNIGHT_PST[piece_pos],
            Role::Bishop => engine_hyperparams::BISHOP_PST[piece_pos],
            Role::Rook => engine_hyperparams::ROOK_PST[piece_pos],
            Role::Queen => engine_hyperparams::QUEEN_PST[piece_pos],
            Role::King => {
                if piece_count > 10 {
                    engine_hyperparams::KING_MG_PST[piece_pos]
                } else {
                    engine_hyperparams::KING_EG_PST[piece_pos]
                }
            }
        };

        total_score += tmp_score
            * if piece.color == current_player_color {
                1
            } else {
                -1
            };
    }
    if piece_count <= 10 {
        total_score += end_game_king_bonuses(position);
    }

    total_score
}

fn end_game_king_bonuses(position: &Chess) -> i64 {
    let board = position.board();
    let player_king_square = board.king_of(position.turn()).unwrap();
    let opponent_king_square = board.king_of(position.turn().other()).unwrap();

    // Calculate the distance between the two kings
    let kings_distance = (player_king_square.file() as i64 - opponent_king_square.file() as i64)
        .abs()
        + (player_king_square.rank() as i64 - opponent_king_square.rank() as i64).abs();

    // Calculate a secondary score based on opponent king distance from center
    let opponent_king_center_distance = i64::max(
        3 - opponent_king_square.file() as i64,
        opponent_king_square.file() as i64 - 4,
    ) + i64::max(
        3 - opponent_king_square.rank() as i64,
        opponent_king_square.rank() as i64 - 4,
    );

    ((14 - kings_distance) + opponent_king_center_distance) * 10
}

// fn get_material_advantage(position: &Chess) -> i64 {
//     let board = position.board();

//     let player_material = board.material_side(position.turn()).zip_role().iter().map(|(role, count)| {
//         get_piece_base_score(*role) * *count as i64
//     }).sum::<i64>();

//     let opponent_material = board.material_side(position.turn().other()).zip_role().iter().map(|(role, count)| {
//         get_piece_base_score(*role) * *count as i64
//     }).sum::<i64>();

//     player_material - opponent_material
// }

// fn end_game_weight(position: &Chess) -> f64 {
//     get_material_advantage(position).abs() as f64 / TOTAL_POSSIBLE_MATERIAL as f64
// }

/// Returns the score of a piece based on its role.
fn get_piece_base_score(role: Role) -> i64 {
    match role {
        Role::Pawn => engine_hyperparams::PAWN_VALUE,
        Role::Knight => engine_hyperparams::KNIGHT_VALUE,
        Role::Bishop => engine_hyperparams::BISHOP_VALUE,
        Role::Rook => engine_hyperparams::ROOK_VALUE,
        Role::Queen => engine_hyperparams::QUEEN_VALUE,
        Role::King => engine_hyperparams::KING_VALUE,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    // use rand::prelude::*;
    use shakmaty::{CastlingMode, fen};

    #[test]
    fn test_evaluate() {
        let position = Chess::default();
        let evaluation = evaluate(&position);
        assert_eq!(evaluation, 0);
    }

    #[test]
    fn test_obvious_score_advantage() {
        let positions_to_move_advantage = vec!["3k4/8/8/8/8/8/8/QQQKQQQQ w - - 0 1"];

        let positions_not_to_move_advantage = vec!["3k4/8/8/8/8/8/8/QQQKQQQQ b - - 0 1"];

        for position in positions_to_move_advantage {
            let fen_position: fen::Fen = position.parse().unwrap();
            let pos: Chess = fen_position.into_position(CastlingMode::Standard).unwrap();
            assert!(evaluate(&pos) > 0);
        }

        for position in positions_not_to_move_advantage {
            let fen_position: fen::Fen = position.parse().unwrap();
            let pos: Chess = fen_position.into_position(CastlingMode::Standard).unwrap();
            assert!(evaluate(&pos) < 0);
        }
    }

    // #[test]
    // fn test_evaluations_are_equivalent() {
    //     // 1. Initialize dependencies
    //     let mut rng = thread_rng();
    //     for i in 0..10_000_000 {
    //         let mut pos = Chess::default();

    //         // 2. Generate a random position by playing a series of random legal moves
    //         // We play 40 half-moves (20 full moves) to get a reasonably complex position.
    //         for _ in 0..=rng.gen_range(40..=80) {
    //             let moves = pos.legal_moves();
    //             if moves.is_empty() {
    //                 // Game is over (checkmate or stalemate), stop making moves.
    //                 break;
    //             }

    //             // Select a random move from the list of legal moves
    //             if let Some(random_move) = moves.choose(&mut rng) {
    //                 pos.play_unchecked(*random_move);
    //             }
    //         }

    //         // 3. Get the FEN representation for debugging purposes.
    //         // If the test fails, this will be printed, allowing you to replicate the exact position.

    //         // 4. Evaluate the final random position with both functions
    //         let score_original = evaluate(&pos);
    //         let score_simple = evaluate_new(&pos);

    //         // 5. Assert that the scores are identical
    //         assert_eq!(
    //             score_original, score_simple,
    //             "\n{} Evaluation functions returned different scores for the same position.\n  - FEN: {}\n  - Original Score: {}\n  - Simple Score: {}",
    //             i, fen::Fen::from_position(&pos, shakmaty::EnPassantMode::Legal).to_string(), score_original, score_simple
    //         );
    //     }
    // }
}
