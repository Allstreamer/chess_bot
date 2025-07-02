use std::sync::OnceLock;

use shakmaty::{Chess, Color, Outcome, Position, Role};

// Values taken from: https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function
const PIECE_VALUES_MG: [i64; 6] = [
    82, // Pawn
    337, // Knight
    365, // Bishop
    477, // Rook
    1025, // Queen
    0, // King
];

const PIECE_VALUES_EG: [i64; 6] = [
    94, // Pawn
    281, // Knight
    297, // Bishop
    512, // Rook
    936, // Queen
    0, // King
];

pub const MATE_SCORE: i64 =             100_000_000;
//   i64  Max                9_223_372_036_854_775_807
pub const POSITIVE_INFINITY: i64 =  9_999_999_999_999;
pub const NEGATIVE_INFINITY: i64 = -POSITIVE_INFINITY;

// ...existing code...

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

fn flip(square: usize) -> usize {
    square ^ 56
}

// Color[PieceType[Square]]
type PieceSquareTableType = [[[i64; 64]; 6]; 2];

fn mg_table() -> &'static PieceSquareTableType {
    static MG_TABLE: OnceLock<PieceSquareTableType> = OnceLock::new();
    MG_TABLE.get_or_init(|| {
        let mut m = [[[0; 64]; 6]; 2];

        for (piece_idx, _) in PIECE_VALUES_MG.iter().enumerate() {
            for square in 0..64 {
                let mg_value = match piece_idx {
                    0 => MG_PAWN_TABLE[square],
                    1 => MG_KNIGHT_TABLE[square],
                    2 => MG_BISHOP_TABLE[square],
                    3 => MG_ROOK_TABLE[square],
                    4 => MG_QUEEN_TABLE[square],
                    5 => MG_KING_TABLE[square],
                    _ => unreachable!(),
                } + PIECE_VALUES_MG[piece_idx];

                m[Color::White as usize][piece_idx][square] = mg_value;
                m[Color::Black as usize][piece_idx][flip(square)] = mg_value;
            }
        } 

        m
    })
}

fn eg_table() -> &'static PieceSquareTableType {
    static EG_TABLE: OnceLock<PieceSquareTableType> = OnceLock::new();
    EG_TABLE.get_or_init(|| {
        let mut m = [[[0; 64]; 6]; 2];

        for (piece_idx, _) in PIECE_VALUES_EG.iter().enumerate() {
            for square in 0..64 {
                let eg_value = match piece_idx {
                    0 => EG_PAWN_TABLE[square],
                    1 => EG_KNIGHT_TABLE[square],
                    2 => EG_BISHOP_TABLE[square],
                    3 => EG_ROOK_TABLE[square],
                    4 => EG_QUEEN_TABLE[square],
                    5 => EG_KING_TABLE[square],
                    _ => unreachable!(),
                } + PIECE_VALUES_EG[piece_idx];

                m[Color::White as usize][piece_idx][square] = eg_value;
                m[Color::Black as usize][piece_idx][flip(square)] = eg_value;
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
    let board = position.board();

    for (square, piece) in board {
        mg_evals[piece.color as usize] += mg_table()[piece.color as usize][piece.role as usize - 1][square as usize];
        eg_evals[piece.color as usize] += eg_table()[piece.color as usize][piece.role as usize - 1][square as usize];
        game_phase += get_piece_eg_increase(piece.role);
    }

    let mg_score = mg_evals[current_player_color as usize] - mg_evals[1 - current_player_color as usize];
    let eg_score = eg_evals[current_player_color as usize] - eg_evals[1 - current_player_color as usize];
    let mg_phase = game_phase.min(24);
    let eg_phase = 24 - mg_phase;

    (mg_score * mg_phase + eg_score * eg_phase) / 24
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