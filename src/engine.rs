use std::{
    collections::HashMap, sync::{atomic::AtomicBool, Arc}
};

use shakmaty::{zobrist::{Zobrist64, ZobristHash}, Chess, Color, Move, Outcome, Position, Role};

use crate::{
    engine_hyperparams::{self, NEGATIVE_INFINITY, POSITIVE_INFINITY},
    log_to_file,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranspositionHashType {
    Exact,
    Alpha,
    Beta,
}
struct TranspositionInformation {
    depth: u64,
    value: i64,
    // best_move: Move,
    transposition_type: TranspositionHashType,
}

/// Returns the best move for the current position using piece count evaluation.
pub fn next_move(position: &Chess, depth: u64, is_thinking: &Arc<AtomicBool>, last_best_move: Option<&Move>) -> Move {
    let mut legal_moves = position.legal_moves();
    legal_moves.sort_by_key(|move_to_score| quick_score_move_for_sort(move_to_score, position, last_best_move));

    let mut transposition_table: HashMap<Zobrist64, TranspositionInformation> = HashMap::new();

    // Find the move that maximizes the evaluation (piece count)
    let mut nodes = 0;
    let mut q_nodes = 0;
    let best_move = legal_moves
        .iter()
        .max_by_key(|legal_move| {
            if !is_thinking.load(std::sync::atomic::Ordering::SeqCst) {
                return NEGATIVE_INFINITY;
            }
            let mut new_position = position.clone();
            new_position.play_unchecked(**legal_move);
            -negamax(
                &new_position,
                depth - 1,
                &mut nodes,
                &mut q_nodes,
                NEGATIVE_INFINITY,
                POSITIVE_INFINITY,
                is_thinking,
                &mut transposition_table
            )
        })
        .expect("No legal moves found");
    log_to_file(&format!(
        "Target Depth: {} | Searched: {} Nodes & {} Q-Nodes",
        depth, nodes, q_nodes
    ))
    .unwrap();
    *best_move
}

fn negamax(
    position: &Chess,
    depth: u64,
    nodes: &mut u64,
    q_nodes: &mut u64,
    mut alpha: i64,
    beta: i64,
    is_thinking: &Arc<AtomicBool>,
    transposition_table: &mut HashMap<Zobrist64, TranspositionInformation>
) -> i64 {
    let mut transposition_type = TranspositionHashType::Alpha;
    let zobrist_hash = position.zobrist_hash::<Zobrist64>(shakmaty::EnPassantMode::Legal);

    if let Some(val) = probe_hash(transposition_table, zobrist_hash, depth, alpha, beta) {
        return val;
    }
    *nodes += 1;

    if depth == 0
        || position.is_game_over()
        || !is_thinking.load(std::sync::atomic::Ordering::SeqCst)
    {
        let val = evaluate(position);
        record_hash(transposition_table, zobrist_hash, depth, val, TranspositionHashType::Exact);
        return val;
    }

    let mut legal_moves = position.legal_moves();
    legal_moves.sort_by_key(|move_to_score| quick_score_move_for_sort(move_to_score, position, None));

    for m in legal_moves {
        let mut new_pos = position.clone();
        new_pos.play_unchecked(m);

        let score = -negamax(
            &new_pos,
            depth - 1,
            nodes,
            q_nodes,
            -beta,
            -alpha,
            is_thinking,
            transposition_table
        );

        if score >= beta {
            record_hash(transposition_table, zobrist_hash, depth, beta, TranspositionHashType::Beta);
            return beta;
        }
        if score > alpha {
            transposition_type = TranspositionHashType::Exact;
            alpha = score;
        }
    }

    record_hash(transposition_table, zobrist_hash, depth, alpha, transposition_type);
    alpha
}

fn probe_hash(transposition_table: &mut HashMap<Zobrist64, TranspositionInformation>, zobrist_hash: Zobrist64, depth: u64, alpha: i64, beta: i64) -> Option<i64> {
    let info_option = transposition_table.get(&zobrist_hash);

    if let Some(info) = info_option {
        if info.depth >= depth {
            if info.transposition_type == TranspositionHashType::Exact {
                return Some(info.value);
            }
            if (info.transposition_type == TranspositionHashType::Alpha) && (info.value <= alpha) {
                return Some(alpha);
            }
            if (info.transposition_type == TranspositionHashType::Beta) && (info.value >= beta) {
                return Some(beta);
            }
        }
        // Not sure how to implment yet so leaving out
        // remember_best_move(); <- Tell move sort to search best move from last gen first
    } 

    None
}

fn record_hash(transposition_table: &mut HashMap<Zobrist64, TranspositionInformation>, zobrist_hash: Zobrist64, depth: u64, value: i64, transposition_type: TranspositionHashType) {
    transposition_table.insert(zobrist_hash, TranspositionInformation { 
        depth: depth, 
        value: value, 
        transposition_type: transposition_type 
    });
}

/// Higher result is a better move
fn quick_score_move_for_sort(move_to_score: &Move, position: &Chess, last_best_move: Option<&Move>) -> i64 {
    let mut score = 0;
    
    if let Some(last_move) = last_best_move {
        // If the move is the same as the last best move, give it a higher score
        if move_to_score == last_move {
            score += 1000; // Arbitrary high value to prioritize this move
        }
    }

    // Prioritize moves that capture high value pieces with low value pieces
    if let Some(captured_piece) = move_to_score.capture() {
        score = (10 * get_piece_base_score(move_to_score.role()))
            - get_piece_base_score(captured_piece);
    }

    // Filter up Promotions
    if let Some(new_piece) = move_to_score.promotion() {
        score += get_piece_base_score(new_piece);
    }

    //
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

/// Calculates a chess position's material score from the players's perspective.
/// A positive score means the player is ahead; a negative score means the opponent is ahead.
fn evaluate(position: &Chess) -> i64 {
    // Initialize a mutable variable `total_score` to 0. This will be our accumulator.
    let mut total_score = 0;
    // Get the color of the player whose turn it is to move (e.g., White or Black).
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

    total_score
}

// fn end_game_king_bonuses(position: &Chess) -> i64 {
//     let board = position.board();
//     let player_king_square = board.king_of(position.turn()).unwrap();
//     let opponent_king_square = board.king_of(position.turn().other()).unwrap();

//     // Calculate the distance between the two kings
//     let kings_distance = (player_king_square.file() as i64 - opponent_king_square.file() as i64).abs()
//         + (player_king_square.rank() as i64 - opponent_king_square.rank() as i64).abs();

//     // Calculate a secondary score based on opponent king distance from center
//     let opponent_king_center_distance =
//     POSITIVE_INFINITY(3 - opponent_king_square.file() as i64, opponent_king_square.file() as i64 - 4)
//         + POSITIVE_INFINITY(3 - opponent_king_square.rank() as i64, opponent_king_square.rank() as i64 - 4);

//     ((14 - kings_distance) + opponent_king_center_distance) * 10
// }

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

/// Returns the score of a piece based on its role. The score is used for evaluation.
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
