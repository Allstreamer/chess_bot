use std::sync::OnceLock;

use shakmaty::{Chess, Color, Outcome, Position, Role, Square};

// Values taken from: https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function
const PIECE_VALUES_MG: [i64; 6] = [
    82,   // Pawn
    337,  // Knight
    365,  // Bishop
    477,  // Rook
    1025, // Queen
    0,    // King
];

const PIECE_VALUES_EG: [i64; 6] = [
    94,  // Pawn
    281, // Knight
    297, // Bishop
    512, // Rook
    936, // Queen
    0,   // King
];

const BISHOP_PAIR_BONUS: i64 = 30; // A bonus for having two bishops
const MOBILITY_WEIGHT: i64 = 2; // Weight for piece mobility
const KING_SAFETY_WEIGHT: i64 = 5; // Weight for king safety evaluation
const ISOLATED_PAWN_PENALTY: i64 = 15; // Penalty for isolated pawns
const DOUBLED_PAWN_PENALTY: i64 = 10; // Penalty for doubled pawns
pub const MATE_SCORE: i64 = 100_000_000;
//   i64  Max                9_223_372_036_854_775_807
pub const POSITIVE_INFINITY: i64 = 9_999_999_999_999;
pub const NEGATIVE_INFINITY: i64 = -POSITIVE_INFINITY;

#[rustfmt::skip]
pub const MG_PAWN_TABLE: [i64; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,
     98, 134,  61,  95,  68, 126,  34, -11,
     -6,   7,  26,  31,  65,  56,  25, -20,
    -14,  13,   6,  21,  23,  12,  17, -23,
    -27,  -2,  -5,  12,  17,   6,  10, -25,
    -26,  -4,  -4, -10,   3,   3,  33, -12,
    -35,  -1, -20, -23, -15,  24,  38, -22,
      0,   0,   0,   0,   0,   0,   0,   0,
];

#[rustfmt::skip]
pub const EG_PAWN_TABLE: [i64; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,
    178, 173, 158, 134, 147, 132, 165, 187,
     94, 100,  85,  67,  56,  53,  82,  84,
     32,  24,  13,   5,  -2,   4,  17,  17,
     13,   9,  -3,  -7,  -7,  -8,   3,  -1,
      4,   7,  -6,   1,   0,  -5,  -1,  -8,
     13,   8,   8,  10,  13,   0,   2,  -7,
      0,   0,   0,   0,   0,   0,   0,   0,
];

#[rustfmt::skip]
pub const MG_KNIGHT_TABLE: [i64; 64] = [
    -167, -89, -34, -49,  61, -97, -15, -107,
     -73, -41,  72,  36,  23,  62,   7,  -17,
     -47,  60,  37,  65,  84, 129,  73,   44,
      -9,  17,  19,  53,  37,  69,  18,   22,
     -13,   4,  16,  13,  28,  19,  21,   -8,
     -23,  -9,  12,  10,  19,  17,  25,  -16,
     -29, -53, -12,  -3,  -1,  18, -14,  -19,
    -105, -21, -58, -33, -17, -28, -19,  -23,
];

#[rustfmt::skip]
pub const EG_KNIGHT_TABLE: [i64; 64] = [
    -58, -38, -13, -28, -31, -27, -63, -99,
    -25,  -8, -25,  -2,  -9, -25, -24, -52,
    -24, -20,  10,   9,  -1,  -9, -19, -41,
    -17,   3,  22,  22,  22,  11,   8, -18,
    -18,  -6,  16,  25,  16,  17,   4, -18,
    -23,  -3,  -1,  15,  10,  -3, -20, -22,
    -42, -20, -10,  -5,  -2, -20, -23, -44,
    -29, -51, -23, -15, -22, -18, -50, -64,
];

#[rustfmt::skip]
pub const MG_BISHOP_TABLE: [i64; 64] = [
    -29,   4, -82, -37, -25, -42,   7,  -8,
    -26,  16, -18, -13,  30,  59,  18, -47,
    -16,  37,  43,  40,  35,  50,  37,  -2,
     -4,   5,  19,  50,  37,  37,   7,  -2,
     -6,  13,  13,  26,  34,  12,  10,   4,
      0,  15,  15,  15,  14,  27,  18,  10,
      4,  15,  16,   0,   7,  21,  33,   1,
    -33,  -3, -14, -21, -13, -12, -39, -21,
];

#[rustfmt::skip]
pub const EG_BISHOP_TABLE: [i64; 64] = [
    -14, -21, -11,  -8,  -7,  -9, -17, -24,
     -8,  -4,   7, -12,  -3, -13,  -4, -14,
      2,  -8,   0,  -1,  -2,   6,   0,   4,
     -3,   9,  12,   9,  14,  10,   3,   2,
     -6,   3,  13,  19,   7,  10,  -3,  -9,
    -12,  -3,   8,  10,  13,   3,  -7, -15,
    -14, -18,  -7,  -1,   4,  -9, -15, -27,
    -23,  -9, -23,  -5,  -9, -16,  -5, -17,
];

#[rustfmt::skip]
pub const MG_ROOK_TABLE: [i64; 64] = [
     32,  42,  32,  51,  63,   9,  31,  43,
     27,  32,  58,  62,  80,  67,  26,  44,
     -5,  19,  26,  36,  17,  45,  61,  16,
    -24, -11,   7,  26,  24,  35,  -8, -20,
    -36, -26, -12,  -1,   9,  -7,   6, -23,
    -45, -25, -16, -17,   3,   0,  -5, -33,
    -44, -16, -20,  -9,  -1,  11,  -6, -71,
    -19, -13,   1,  17,  16,   7, -37, -26,
];

#[rustfmt::skip]
pub const EG_ROOK_TABLE: [i64; 64] = [
     13,  10,  18,  15,  12,  12,   8,   5,
     11,  13,  13,  11,  -3,   3,   8,   3,
      7,   7,   7,   5,   4,  -3,  -5,  -3,
      4,   3,  13,   1,   2,   1,  -1,   2,
      3,   5,   8,   4,  -5,  -6,  -8, -11,
     -4,   0,  -5,  -1,  -7, -12,  -8, -16,
     -6,  -6,   0,   2,  -9,  -9, -11,  -3,
     -9,   2,   3,  -1,  -5, -13,   4, -20,
];

#[rustfmt::skip]
pub const MG_QUEEN_TABLE: [i64; 64] = [
    -28,   0,  29,  12,  59,  44,  43,  45,
    -24, -39,  -5,   1, -16,  57,  28,  54,
    -13, -17,   7,   8,  29,  56,  47,  57,
    -27, -27, -16, -16,  -1,  17,  -2,   1,
     -9, -26,  -9, -10,  -2,  -4,   3,  -3,
    -14,   2, -11,  -2,  -5,   2,  14,   5,
    -35,  -8,  11,   2,   8,  15,  -3,   1,
     -1, -18,  -9,  10, -15, -25, -31, -50,
];

#[rustfmt::skip]
pub const EG_QUEEN_TABLE: [i64; 64] = [
     -9,  22,  22,  27,  27,  19,  10,  20,
    -17,  20,  32,  41,  58,  25,  30,   0,
    -20,   6,   9,  49,  47,  35,  19,   9,
      3,  22,  24,  45,  57,  40,  57,  36,
    -18,  28,  19,  47,  31,  34,  39,  23,
    -16, -27,  15,   6,   9,  17,  10,   5,
    -22, -23, -30, -16, -16, -23, -36, -32,
    -33, -28, -22, -43,  -5, -32, -20, -41,
];

#[rustfmt::skip]
pub const MG_KING_TABLE: [i64; 64] = [
    -65,  23,  16, -15, -56, -34,   2,  13,
     29,  -1, -20,  -7,  -8,  -4, -38, -29,
     -9,  24,   2, -16, -20,   6,  22, -22,
    -17, -20, -12, -27, -30, -25, -14, -36,
    -49,  -1, -27, -39, -46, -44, -33, -51,
    -14, -14, -22, -46, -44, -30, -15, -27,
      1,   7,  -8, -64, -43, -16,   9,   8,
    -15,  36,  12, -54,   8, -28,  24,  14,
];

#[rustfmt::skip]
pub const EG_KING_TABLE: [i64; 64] = [
    -74, -35, -18, -18, -11,  15,   4, -17,
    -12,  17,  14,  17,  17,  38,  23,  11,
     10,  17,  23,  15,  20,  45,  44,  13,
     -8,  22,  24,  27,  26,  33,  26,   3,
    -18,  -4,  21,  24,  27,  23,   9, -11,
    -19,  -3,  11,  21,  23,  16,   7,  -9,
    -27, -11,   4,  13,  14,   4,  -5, -17,
    -53, -34, -21, -11, -28, -14, -24, -43,
];

pub fn get_piece_eg_increase(role: Role) -> i64 {
    match role {
        Role::Pawn => 0,
        Role::Knight => 1,
        Role::Bishop => 1,
        Role::Rook => 2,
        Role::Queen => 4,
        Role::King => 0,
    }
}

// Color[PieceType[Square]]
type PieceSquareTableType = [[[i64; 64]; 6]; 2];

pub fn mg_table() -> &'static PieceSquareTableType {
    static MG_TABLE: OnceLock<PieceSquareTableType> = OnceLock::new();
    MG_TABLE.get_or_init(|| {
        let mut m = [[[0; 64]; 6]; 2];

        for square in Square::ALL {
            for (piece_idx, _) in PIECE_VALUES_MG.iter().enumerate() {
                let mg_value = match piece_idx {
                    0 => MG_PAWN_TABLE[square as usize],
                    1 => MG_KNIGHT_TABLE[square as usize],
                    2 => MG_BISHOP_TABLE[square as usize],
                    3 => MG_ROOK_TABLE[square as usize],
                    4 => MG_QUEEN_TABLE[square as usize],
                    5 => MG_KING_TABLE[square as usize],
                    _ => unreachable!(),
                } + PIECE_VALUES_MG[piece_idx];

                m[Color::White as usize][piece_idx][square.flip_vertical() as usize] = mg_value;
                m[Color::Black as usize][piece_idx][square as usize] = mg_value;
            }
        }

        m
    })
}

pub fn eg_table() -> &'static PieceSquareTableType {
    static EG_TABLE: OnceLock<PieceSquareTableType> = OnceLock::new();
    EG_TABLE.get_or_init(|| {
        let mut m = [[[0; 64]; 6]; 2];

        for square in Square::ALL {
            for (piece_idx, _) in PIECE_VALUES_EG.iter().enumerate() {
                let eg_value = match piece_idx {
                    0 => EG_PAWN_TABLE[square as usize],
                    1 => EG_KNIGHT_TABLE[square as usize],
                    2 => EG_BISHOP_TABLE[square as usize],
                    3 => EG_ROOK_TABLE[square as usize],
                    4 => EG_QUEEN_TABLE[square as usize],
                    5 => EG_KING_TABLE[square as usize],
                    _ => unreachable!(),
                } + PIECE_VALUES_EG[piece_idx];

                m[Color::White as usize][piece_idx][square.flip_vertical() as usize] = eg_value;
                m[Color::Black as usize][piece_idx][square as usize] = eg_value;
            }
        }

        m
    })
}

/// Assesses the complexity of a chess position for time management purposes.
/// Returns a complexity score where higher values indicate more complex positions.
pub fn assess_position_complexity(position: &Chess) -> f64 {
    let mut complexity = 0.0;
    
    // Base complexity from number of legal moves
    let legal_moves_count = position.legal_moves().len();
    complexity += (legal_moves_count as f64) * 0.1;
    
    // Complexity from material balance - closer games are more complex
    let eval = evaluate(position);
    let material_balance_factor = 1.0 - (eval.abs() as f64 / 500.0).min(1.0);
    complexity += material_balance_factor * 2.0;
    
    // Complexity from game phase - middle game is most complex
    let board = position.board();
    let mut total_material = 0;
    for (_, piece) in board {
        total_material += get_piece_eg_increase(piece.role);
    }
    
    // Peak complexity around middle game (12-18 material points)
    let phase_complexity = if total_material >= 12 && total_material <= 18 {
        2.0
    } else if total_material >= 8 && total_material <= 24 {
        1.5
    } else {
        1.0
    };
    complexity += phase_complexity;
    
    // Add complexity if in check (tactical positions)
    if position.checkers().any() {
        complexity += 1.5;
    }
    
    // Add complexity based on tactical threats (captures available)
    let capture_count = position.legal_moves().iter()
        .filter(|m| m.capture().is_some())
        .count();
    complexity += (capture_count as f64) * 0.2;
    
    complexity.max(1.0) // Minimum complexity of 1.0
}

/// Evaluates piece mobility - how many squares pieces can move to
fn evaluate_mobility(position: &Chess, color: Color) -> i64 {
    let mut mobility = 0;
    let board = position.board();
    
    for (square, piece) in board {
        if piece.color != color {
            continue;
        }
        
        let piece_mobility = match piece.role {
            Role::Knight => {
                board.attacks_from(square).count() as i64
            },
            Role::Bishop => {
                board.attacks_from(square).count() as i64
            },
            Role::Rook => {
                board.attacks_from(square).count() as i64
            },
            Role::Queen => {
                (board.attacks_from(square).count() as i64) / 2
            },
            _ => 0, // Don't count pawn and king mobility this way
        };
        
        mobility += piece_mobility;
    }
    
    mobility * MOBILITY_WEIGHT
}

/// Evaluates king safety by checking pawn shelter and enemy piece attacks
fn evaluate_king_safety(position: &Chess, color: Color) -> i64 {
    let board = position.board();
    let king_square = board.king_of(color);
    
    if king_square.is_none() {
        return -1000; // No king is very bad
    }
    
    let king_square = king_square.unwrap();
    let mut safety_score = 0;
    
    // Evaluate pawn shelter in front of king
    let king_file = king_square.file();
    let direction = if color == Color::White { 1 } else { -1 };
    let king_rank = king_square.rank() as i8;
    
    // Check pawn shelter on king's file and adjacent files
    for file_offset in -1..=1 {
        if let Some(file) = king_file.offset(file_offset) {
            // Look for friendly pawns in front of king (1-2 ranks ahead)
            for rank_offset in 1..=2 {
                let target_rank = king_rank + (direction * rank_offset);
                if target_rank >= 0 && target_rank < 8 {
                    let rank = shakmaty::Rank::new(target_rank as u32);
                    let target_square = Square::from_coords(file, rank);
                    if let Some(piece) = board.piece_at(target_square) {
                        if piece.color == color && piece.role == Role::Pawn {
                            safety_score += 10; // Pawn shelter bonus
                        }
                    }
                }
            }
        }
    }
    
    // Penalty for enemy pieces attacking near the king
    let king_area = board.attacks_from(king_square);
    for attack_square in king_area {
        let attackers = board.attacks_to(attack_square, color.other(), board.occupied());
        safety_score -= (attackers.count() as i64) * 5;
    }
    
    safety_score * KING_SAFETY_WEIGHT
}

/// Evaluates pawn structure (isolated and doubled pawns)
fn evaluate_pawn_structure(position: &Chess, color: Color) -> i64 {
    let board = position.board();
    let mut pawn_structure_score = 0;
    let mut pawn_files = [0u8; 8]; // Count pawns on each file
    
    // Count pawns on each file
    for (square, piece) in board {
        if piece.color == color && piece.role == Role::Pawn {
            pawn_files[square.file() as usize] += 1;
        }
    }
    
    // Apply penalties for pawn structure issues
    for file in 0..8 {
        let pawn_count = pawn_files[file];
        
        if pawn_count > 0 {
            // Check for isolated pawns (no pawns on adjacent files)
            let has_neighbor = (file > 0 && pawn_files[file - 1] > 0) ||
                              (file < 7 && pawn_files[file + 1] > 0);
            
            if !has_neighbor {
                pawn_structure_score -= ISOLATED_PAWN_PENALTY * (pawn_count as i64);
            }
            
            // Penalty for doubled pawns
            if pawn_count > 1 {
                pawn_structure_score -= DOUBLED_PAWN_PENALTY * (pawn_count as i64 - 1);
            }
        }
    }
    
    pawn_structure_score
}

/// Determines if a position has obvious moves (opening, forced moves, winning positions)
pub fn has_obvious_move(position: &Chess) -> bool {
    let legal_moves = position.legal_moves();
    
    // Very few legal moves suggests forced play
    if legal_moves.len() <= 2 {
        return true;
    }
    
    // Large material advantage suggests obvious moves
    let eval = evaluate(position);
    if eval.abs() > 800 { // More than a rook ahead
        return true;
    }
    
    // Check if in opening (many pieces on starting squares)
    let board = position.board();
    let mut pieces_on_starting_squares = 0;
    let mut total_pieces = 0;
    
    for (square, piece) in board {
        total_pieces += 1;
        
        // Check if piece is on starting position more precisely
        let is_on_starting_square = match (piece.color, piece.role, square) {
            // White pieces on starting squares
            (Color::White, Role::Rook, shakmaty::Square::A1 | shakmaty::Square::H1) => true,
            (Color::White, Role::Knight, shakmaty::Square::B1 | shakmaty::Square::G1) => true,
            (Color::White, Role::Bishop, shakmaty::Square::C1 | shakmaty::Square::F1) => true,
            (Color::White, Role::Queen, shakmaty::Square::D1) => true,
            (Color::White, Role::King, shakmaty::Square::E1) => true,
            // Black pieces on starting squares
            (Color::Black, Role::Rook, shakmaty::Square::A8 | shakmaty::Square::H8) => true,
            (Color::Black, Role::Knight, shakmaty::Square::B8 | shakmaty::Square::G8) => true,
            (Color::Black, Role::Bishop, shakmaty::Square::C8 | shakmaty::Square::F8) => true,
            (Color::Black, Role::Queen, shakmaty::Square::D8) => true,
            (Color::Black, Role::King, shakmaty::Square::E8) => true,
            // Pawns on 2nd/7th rank
            (Color::White, Role::Pawn, square) if square.rank() == shakmaty::Rank::Second => true,
            (Color::Black, Role::Pawn, square) if square.rank() == shakmaty::Rank::Seventh => true,
            _ => false,
        };
        
        if is_on_starting_square {
            pieces_on_starting_squares += 1;
        }
    }
    
    // If more than 50% of pieces are still on starting squares, it's opening
    if pieces_on_starting_squares > (total_pieces * 50) / 100 {
        return true;
    }
    
    false
}

/// Calculates a chess position's score from the players's perspective.
/// A positive score means the player is ahead; a negative score means the opponent is ahead.
pub fn evaluate(position: &Chess) -> i64 {
    let current_player_color = position.turn();

    if position.is_game_over() {
        return match position.outcome() {
            Some(Outcome::Decisive { winner }) => {
                if winner == current_player_color {
                    MATE_SCORE
                } else {
                    -MATE_SCORE // Being checkmated is the worst outcome
                }
            }
            _ => 0, // Any other outcome (stalemate, etc.) is neutral
        };
    }

    let mut mg_evals = [0i64; 2];
    let mut eg_evals = [0i64; 2];
    let mut game_phase = 0;
    let mut bishop_counts = [0, 0];
    let board = position.board();

    for (square, piece) in board {
        if piece.role == Role::Bishop {
            bishop_counts[piece.color as usize] += 1;
        }

        // piece.color is 0 for Black and 1 for White
        // piece.role is 1-indexed (1 for Pawn, 2 for Knight, etc.)
        mg_evals[piece.color as usize] +=
            mg_table()[piece.color as usize][piece.role as usize - 1][square as usize];
        eg_evals[piece.color as usize] +=
            eg_table()[piece.color as usize][piece.role as usize - 1][square as usize];
        game_phase += get_piece_eg_increase(piece.role);
    }

    // Add bishop pair bonus
    if bishop_counts[Color::White as usize] >= 2 {
        mg_evals[Color::White as usize] += BISHOP_PAIR_BONUS;
        eg_evals[Color::White as usize] += BISHOP_PAIR_BONUS;
    }
    if bishop_counts[Color::Black as usize] >= 2 {
        mg_evals[Color::Black as usize] += BISHOP_PAIR_BONUS;
        eg_evals[Color::Black as usize] += BISHOP_PAIR_BONUS;
    }
    
    // Calculate base positional score
    let mg_score =
        mg_evals[current_player_color as usize] - mg_evals[current_player_color.other() as usize];
    let eg_score =
        eg_evals[current_player_color as usize] - eg_evals[current_player_color.other() as usize];
    let mg_phase = game_phase.min(24);
    let eg_phase = 24 - mg_phase;
    
    let mut total_score = (mg_score * mg_phase + eg_score * eg_phase) / 24;
    
    // Add mobility evaluation (more important in middlegame)
    let mobility_score = evaluate_mobility(position, current_player_color) - 
                        evaluate_mobility(position, current_player_color.other());
    total_score += (mobility_score * mg_phase) / 24;
    
    // Add king safety evaluation (more important in middlegame)
    let king_safety_score = evaluate_king_safety(position, current_player_color) - 
                           evaluate_king_safety(position, current_player_color.other());
    total_score += (king_safety_score * mg_phase) / 24;
    
    // Add pawn structure evaluation (important throughout the game)
    let pawn_structure_score = evaluate_pawn_structure(position, current_player_color) - 
                              evaluate_pawn_structure(position, current_player_color.other());
    total_score += pawn_structure_score;
    
    total_score
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_evaluate() {
        let position = Chess::default();
        let evaluation = evaluate(&position);
        assert_eq!(evaluation, 0);
    }
}
