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

// Pawn structure bonuses/penalties
const DOUBLED_PAWN_PENALTY: i64 = -10;
const ISOLATED_PAWN_PENALTY: i64 = -15;
const PASSED_PAWN_BONUS: i64 = 20;
const PAWN_CHAIN_BONUS: i64 = 5;

// King safety bonuses/penalties
const KING_SAFETY_BONUS: i64 = 20;
const KING_EXPOSURE_PENALTY: i64 = -30;

// Mobility bonuses (per square controlled)
const MOBILITY_BONUS_KNIGHT: i64 = 5;
const MOBILITY_BONUS_BISHOP: i64 = 4;
const MOBILITY_BONUS_ROOK: i64 = 3;
const MOBILITY_BONUS_QUEEN: i64 = 2;

const BISHOP_PAIR_BONUS: i64 = 30; // A bonus for having two bishops
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

    // Add pawn structure evaluation
    let pawn_structure_eval = evaluate_pawn_structure(position);
    mg_evals[Color::White as usize] += pawn_structure_eval.0;
    mg_evals[Color::Black as usize] += pawn_structure_eval.1;
    eg_evals[Color::White as usize] += pawn_structure_eval.0;
    eg_evals[Color::Black as usize] += pawn_structure_eval.1;

    // Add king safety evaluation
    let king_safety_eval = evaluate_king_safety(position);
    mg_evals[Color::White as usize] += king_safety_eval.0;
    mg_evals[Color::Black as usize] += king_safety_eval.1;
    eg_evals[Color::White as usize] += king_safety_eval.0;
    eg_evals[Color::Black as usize] += king_safety_eval.1;

    // Add mobility evaluation
    let mobility_eval = evaluate_mobility(position);
    mg_evals[Color::White as usize] += mobility_eval.0;
    mg_evals[Color::Black as usize] += mobility_eval.1;
    eg_evals[Color::White as usize] += mobility_eval.0;
    eg_evals[Color::Black as usize] += mobility_eval.1;

    // Add bishop pair bonus
    if bishop_counts[Color::White as usize] >= 2 {
        mg_evals[Color::White as usize] += BISHOP_PAIR_BONUS;
        eg_evals[Color::White as usize] += BISHOP_PAIR_BONUS;
    }
    if bishop_counts[Color::Black as usize] >= 2 {
        mg_evals[Color::Black as usize] += BISHOP_PAIR_BONUS;
        eg_evals[Color::Black as usize] += BISHOP_PAIR_BONUS;
    }
    let mg_score =
        mg_evals[current_player_color as usize] - mg_evals[current_player_color.other() as usize];
    let eg_score =
        eg_evals[current_player_color as usize] - eg_evals[current_player_color.other() as usize];
    let mg_phase = game_phase.min(24);
    let eg_phase = 24 - mg_phase;

    (mg_score * mg_phase + eg_score * eg_phase) / 24
}

/// Evaluate pawn structure for both sides (doubled, isolated, passed pawns)
fn evaluate_pawn_structure(position: &Chess) -> (i64, i64) {
    let mut white_pawn_eval = 0;
    let mut black_pawn_eval = 0;

    let white_pawns = position.board().pawns() & position.board().white();
    let black_pawns = position.board().pawns() & position.board().black();

    // Evaluate doubled pawns (simplified approach)
    for file in 0..8 {
        let mut white_count = 0;
        let mut black_count = 0;

        // Count pawns on this file
        for square in white_pawns {
            if square.file() as u32 == file {
                white_count += 1;
            }
        }
        for square in black_pawns {
            if square.file() as u32 == file {
                black_count += 1;
            }
        }

        if white_count > 1 {
            white_pawn_eval += DOUBLED_PAWN_PENALTY * (white_count - 1) as i64;
        }
        if black_count > 1 {
            black_pawn_eval += DOUBLED_PAWN_PENALTY * (black_count - 1) as i64;
        }
    }

    // Evaluate isolated pawns (simplified approach)
    for square in white_pawns {
        let file = square.file() as u32;
        let mut has_adjacent_pawn = false;

        // Check adjacent files for pawns
        for adj_square in white_pawns {
            let adj_file = adj_square.file() as u32;
            if adj_file != file && (adj_file as i32 - file as i32).abs() == 1 {
                has_adjacent_pawn = true;
                break;
            }
        }

        if !has_adjacent_pawn {
            white_pawn_eval += ISOLATED_PAWN_PENALTY;
        }
    }

    for square in black_pawns {
        let file = square.file() as u32;
        let mut has_adjacent_pawn = false;

        // Check adjacent files for pawns
        for adj_square in black_pawns {
            let adj_file = adj_square.file() as u32;
            if adj_file != file && (adj_file as i32 - file as i32).abs() == 1 {
                has_adjacent_pawn = true;
                break;
            }
        }

        if !has_adjacent_pawn {
            black_pawn_eval += ISOLATED_PAWN_PENALTY;
        }
    }

    // Evaluate passed pawns (simplified approach)
    for square in white_pawns {
        let file = square.file() as u32;
        let rank = square.rank() as u32;
        let mut is_passed = true;

        // Check if there are black pawns ahead
        for opp_square in black_pawns {
            let opp_file = opp_square.file() as u32;
            let opp_rank = opp_square.rank() as u32;

            // Check if opponent pawn is ahead on same or adjacent file
            if opp_rank > rank && (opp_file as i32 - file as i32).abs() <= 1 {
                is_passed = false;
                break;
            }
        }

        if is_passed {
            white_pawn_eval += PASSED_PAWN_BONUS;
        }
    }

    for square in black_pawns {
        let file = square.file() as u32;
        let rank = square.rank() as u32;
        let mut is_passed = true;

        // Check if there are white pawns ahead
        for opp_square in white_pawns {
            let opp_file = opp_square.file() as u32;
            let opp_rank = opp_square.rank() as u32;

            // Check if opponent pawn is ahead on same or adjacent file
            if opp_rank < rank && (opp_file as i32 - file as i32).abs() <= 1 {
                is_passed = false;
                break;
            }
        }

        if is_passed {
            black_pawn_eval += PASSED_PAWN_BONUS;
        }
    }

    // Evaluate pawn chains (simplified approach)
    for square in white_pawns {
        let file = square.file() as u32;
        let rank = square.rank() as u32;
        let mut is_supported = false;

        // Check if this pawn is supported by another pawn diagonally
        if rank > 0 {
            // Check diagonally backward squares for supporting pawns
            if file > 0 {
                let support_square = Square::from_coords(
                    shakmaty::File::new(file - 1),
                    shakmaty::Rank::new(rank - 1),
                );
                for support_pawn in white_pawns {
                    if support_pawn == support_square {
                        is_supported = true;
                        break;
                    }
                }
            }
            if file < 7 {
                let support_square = Square::from_coords(
                    shakmaty::File::new(file + 1),
                    shakmaty::Rank::new(rank - 1),
                );
                for support_pawn in white_pawns {
                    if support_pawn == support_square {
                        is_supported = true;
                        break;
                    }
                }
            }
        }

        if is_supported {
            white_pawn_eval += PAWN_CHAIN_BONUS;
        }
    }

    for square in black_pawns {
        let file = square.file() as u32;
        let rank = square.rank() as u32;
        let mut is_supported = false;

        // Check if this pawn is supported by another pawn diagonally
        if rank < 7 {
            // Check diagonally forward squares for supporting pawns
            if file > 0 {
                let support_square = Square::from_coords(
                    shakmaty::File::new(file - 1),
                    shakmaty::Rank::new(rank + 1),
                );
                for support_pawn in black_pawns {
                    if support_pawn == support_square {
                        is_supported = true;
                        break;
                    }
                }
            }
            if file < 7 {
                let support_square = Square::from_coords(
                    shakmaty::File::new(file + 1),
                    shakmaty::Rank::new(rank + 1),
                );
                for support_pawn in black_pawns {
                    if support_pawn == support_square {
                        is_supported = true;
                        break;
                    }
                }
            }
        }

        if is_supported {
            black_pawn_eval += PAWN_CHAIN_BONUS;
        }
    }

    (white_pawn_eval, black_pawn_eval)
}

/// Evaluate king safety for both sides
fn evaluate_king_safety(position: &Chess) -> (i64, i64) {
    let mut white_king_eval = 0;
    let mut black_king_eval = 0;

    // Find king positions
    let white_king_square = position.board().king_of(Color::White);
    let black_king_square = position.board().king_of(Color::Black);

    if let (Some(white_king), Some(black_king)) = (white_king_square, black_king_square) {
        // Check if kings are in the center (more exposed)
        let white_king_file = white_king.file() as u32;
        let black_king_file = black_king.file() as u32;

        // Kings on center files (d, e, f) are more exposed
        if (3..6).contains(&white_king_file) {
            white_king_eval += KING_EXPOSURE_PENALTY;
        }
        if (3..6).contains(&black_king_file) {
            black_king_eval += KING_EXPOSURE_PENALTY;
        }

        // Count friendly pieces near the king (more pieces = safer)
        let mut white_friendly_count = 0;
        let mut black_friendly_count = 0;

        // Count pieces near white king
        for square in position.board().white() {
            let file_diff = (square.file() as i32 - white_king.file() as i32).abs();
            let rank_diff = (square.rank() as i32 - white_king.rank() as i32).abs();
            if file_diff <= 2 && rank_diff <= 2 {
                white_friendly_count += 1;
            }
        }

        // Count pieces near black king
        for square in position.board().black() {
            let file_diff = (square.file() as i32 - black_king.file() as i32).abs();
            let rank_diff = (square.rank() as i32 - black_king.rank() as i32).abs();
            if file_diff <= 2 && rank_diff <= 2 {
                black_friendly_count += 1;
            }
        }

        white_king_eval += white_friendly_count * KING_SAFETY_BONUS;
        black_king_eval += black_friendly_count * KING_SAFETY_BONUS;
    }

    (white_king_eval, black_king_eval)
}

/// Evaluate piece mobility for both sides
fn evaluate_mobility(position: &Chess) -> (i64, i64) {
    let mut white_mobility_eval = 0;
    let mut black_mobility_eval = 0;

    // Get all pieces for each side
    let white_pieces = position.board().white();
    let black_pieces = position.board().black();

    // Evaluate mobility for white pieces
    for square in white_pieces {
        if let Some(piece) = position.board().role_at(square) {
            let mobility = get_piece_mobility(square, piece, position, Color::White);
            match piece {
                Role::Knight => white_mobility_eval += mobility * MOBILITY_BONUS_KNIGHT,
                Role::Bishop => white_mobility_eval += mobility * MOBILITY_BONUS_BISHOP,
                Role::Rook => white_mobility_eval += mobility * MOBILITY_BONUS_ROOK,
                Role::Queen => white_mobility_eval += mobility * MOBILITY_BONUS_QUEEN,
                _ => (), // Pawns and kings don't get mobility bonus in this simple implementation
            }
        }
    }

    // Evaluate mobility for black pieces
    for square in black_pieces {
        if let Some(piece) = position.board().role_at(square) {
            let mobility = get_piece_mobility(square, piece, position, Color::Black);
            match piece {
                Role::Knight => black_mobility_eval += mobility * MOBILITY_BONUS_KNIGHT,
                Role::Bishop => black_mobility_eval += mobility * MOBILITY_BONUS_BISHOP,
                Role::Rook => black_mobility_eval += mobility * MOBILITY_BONUS_ROOK,
                Role::Queen => black_mobility_eval += mobility * MOBILITY_BONUS_QUEEN,
                _ => (), // Pawns and kings don't get mobility bonus in this simple implementation
            }
        }
    }

    (white_mobility_eval, black_mobility_eval)
}

/// Get the mobility of a piece (number of legal moves it can make)
fn get_piece_mobility(square: Square, piece: Role, position: &Chess, color: Color) -> i64 {
    let mut mobility = 0;

    // For each piece type, count the number of squares it can attack/occupy
    match piece {
        Role::Knight => {
            // Knight moves in an L-shape: 8 possible directions
            let knight_moves = [
                (2, 1),
                (2, -1),
                (-2, 1),
                (-2, -1),
                (1, 2),
                (1, -2),
                (-1, 2),
                (-1, -2),
            ];

            for &(file_offset, rank_offset) in &knight_moves {
                let new_file = square.file() as i32 + file_offset;
                let new_rank = square.rank() as i32 + rank_offset;

                if (0..8).contains(&new_file) && (0..8).contains(&new_rank) {
                    // Check if the square is not occupied by a friendly piece
                    let target_square = Square::from_coords(
                        shakmaty::File::new(new_file as u32),
                        shakmaty::Rank::new(new_rank as u32),
                    );

                    // Check if square is occupied by a friendly piece
                    let occupied = position.board().occupied();
                    let friendly = position.board().white();
                    let is_friendly_occupied = if color == Color::White {
                        (friendly & occupied).contains(target_square)
                    } else {
                        let black = position.board().black();
                        (black & occupied).contains(target_square)
                    };

                    if !is_friendly_occupied {
                        mobility += 1;
                    }
                }
            }
        }
        Role::Bishop => {
            // Bishop moves diagonally in 4 directions
            let directions: &[(i32, i32)] = &[(1, 1), (1, -1), (-1, 1), (-1, -1)];
            mobility += count_ray_mobility(square, directions, position, color);
        }
        Role::Rook => {
            // Rook moves horizontally and vertically in 4 directions
            let directions: &[(i32, i32)] = &[(1, 0), (-1, 0), (0, 1), (0, -1)];
            mobility += count_ray_mobility(square, directions, position, color);
        }
        Role::Queen => {
            // Queen moves in 8 directions (bishop + rook)
            let directions: &[(i32, i32)] = &[
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ];
            mobility += count_ray_mobility(square, directions, position, color);
        }
        _ => (), // Other pieces don't get mobility in this simple implementation
    }

    mobility
}

/// Count mobility along rays (for sliding pieces)
fn count_ray_mobility(
    square: Square,
    directions: &[(i32, i32)],
    position: &Chess,
    color: Color,
) -> i64 {
    let mut mobility = 0;
    let occupied = position.board().occupied();
    let friendly = if color == Color::White {
        position.board().white()
    } else {
        position.board().black()
    };
    let opponent = if color == Color::White {
        position.board().black()
    } else {
        position.board().white()
    };

    for &(file_offset, rank_offset) in directions {
        let mut current_file = square.file() as i32;
        let mut current_rank = square.rank() as i32;

        loop {
            current_file += file_offset;
            current_rank += rank_offset;

            // Check if we're still on the board
            if !(0..8).contains(&current_file) || !(0..8).contains(&current_rank) {
                break;
            }

            let target_square = Square::from_coords(
                shakmaty::File::new(current_file as u32),
                shakmaty::Rank::new(current_rank as u32),
            );

            // If we hit a friendly piece, stop
            if (friendly & occupied).contains(target_square) {
                break;
            }

            // Count this square as mobile
            mobility += 1;

            // If we hit an enemy piece, stop (but we still count this square)
            if (opponent & occupied).contains(target_square) {
                break;
            }
        }
    }

    mobility
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
