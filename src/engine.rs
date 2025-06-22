use shakmaty::{Chess, Color, Move, Outcome, Position, Role};

use crate::engine_hyperparams;

/// Returns the best move for the current position using piece count evaluation.
pub fn next_move(position: &Chess, depth: u8) -> Move {
    let legal_moves = position.legal_moves(); // Get all legal moves

    // Find the move that maximizes the evaluation (piece count)
    let best_move = legal_moves
        .iter()
        .max_by_key(|legal_move| {
            let mut new_position = position.clone();
            new_position.play_unchecked(**legal_move);
            -negamax(&new_position, depth - 1)
        })
        .expect("No legal moves found");

    *best_move
}

fn negamax(position: &Chess, depth: u8) -> i64 {
    if depth == 0 || position.is_game_over() {
        return evaluate(position);
    }

    let mut max_score = i64::MIN;
    let legal_moves = position.legal_moves();

    for m in legal_moves {
        let mut new_pos = position.clone();
        new_pos.play_unchecked(m);

        let score = -negamax(&new_pos, depth - 1);

        if score > max_score {
            max_score = score;
        }
    }

    max_score
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
                }else {
                    -engine_hyperparams::MATE_SCORE // Being checkmated is the worst outcome
                }
            }, 
            _ => 0, // Any other outcome (stalemate, etc.) is neutral
        };
    }

    // Iterate through all the pieces on the board, grouped by color.
    // This loop will run twice: once for all of White's pieces, and once for all of Black's.
    for (color, pieces) in position.board().material().zip_color().iter() {
        // For the current color being processed, calculate its total material value.
        let side_score: i64 = pieces
            // Get an iterator that yields each piece type and how many of them exist (e.g., (Pawn, 8)).
            .zip_role()
            // Create a standard iterator from the result.
            .iter()
            // For each piece type, transform it into a score.
            // The `map` function takes a closure that defines this transformation.
            .map(|(role, count)| get_score(*role) * (*count as i64))
            // Sum up all the individual piece scores to get the total material value for this side.
            .sum();

        // Check if the pieces we just evaluated belong to the player whose turn it is.
        if *color == current_player_color {
            // If they are the current player's pieces, add their value to the total score.
            total_score += side_score;
        } else {
            // Otherwise, they must be the opponent's pieces, so subtract their value.
            total_score -= side_score;
        }
    } // After this loop, `total_score` holds the standard evaluation: (my material) - (opponent's material).

    total_score
}

/// Returns the score of a piece based on its role. The score is used for evaluation.
fn get_score(role: Role) -> i64 {
    match role {
        Role::Pawn => engine_hyperparams::PAWN_VALUE,
        Role::Knight => engine_hyperparams::KNIGHT_VALUE,
        Role::Bishop => engine_hyperparams::BISHOP_VALUE,
        Role::Rook => engine_hyperparams::ROOK_VALUE,
        Role::Queen => engine_hyperparams::QUEEN_VALUE,
        Role::King => engine_hyperparams::KING_VALUE,
    }
}

/// Returns 1 if the piece color is the same as the color of the player whose turn it is, otherwise -1.
fn is_opponent(piece_color: Color, our_color: Color) -> i64 {
    if piece_color == our_color { 1 } else { -1 }
}

#[cfg(test)]
mod test {
    use super::*;
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

    #[test]
    fn test_is_opponent() {
        assert_eq!(is_opponent(Color::White, Color::White), 1);
        assert_eq!(is_opponent(Color::Black, Color::White), -1);
        assert_eq!(is_opponent(Color::White, Color::Black), -1);
        assert_eq!(is_opponent(Color::Black, Color::Black), 1);
    }

    #[test]
    fn test_next_move() {
        let position = Chess::default();
        let best_move = next_move(&position, 2);
        assert!(position.legal_moves().contains(&best_move));
    }
}
