use shakmaty::{Chess, Color, Move, Position, Role};

use crate::engine_hyperparams;

/// Returns the best move for the current position using piece count evaluation.
pub fn next_move(position: &Chess) -> Move {
    let legal_moves = position.legal_moves(); // Get all legal moves

    // Find the move that maximizes the evaluation (piece count)
    let best_move = legal_moves
        .iter()
        .max_by_key(|legal_move| {
            let new_position = position.clone().play(**legal_move).expect("Move is legal");
            evaluate(&new_position)
        })
        .expect("No legal moves found");

    *best_move
}

/// Evaluates the position using piece count.
fn evaluate(position: &Chess) -> i64 {
    /*
        This is using iterative folding to calculate the evaluation of the position.
        The evaluation is calculated by iterating over the pieces on the board and summing up the score of each piece.
        The score of each piece is calculated by multiplying the count of the piece by the score of the piece and then taken positively or negatively based on the color of the piece.
    */
    position
        .board()
        .material()
        .zip_color()
        .iter()
        .fold(0, |acc, (color, pieces)| {
            acc + pieces.zip_role().iter().fold(0, |acc, (role, count)| {
                let score = get_score(*role)
                    * (*count as i64)
                    * is_opponent((*color).other(), position.turn()); // we have to invert the color because by playing the move we are changing the turn.
                acc + score
            })
        })
}

/// Calculates a chess position's material score from the opponent's perspective.
/// A positive score means the opponent is ahead; a negative score means the current player is ahead.
fn evaluate_simple(position: &Chess) -> i64 {
    // Initialize a mutable variable `total_score` to 0. This will be our accumulator.
    let mut total_score = 0;
    // Get the color of the player whose turn it is to move (e.g., White or Black).
    let current_player_color = position.turn();

    // This comment block describes the logic within the upcoming for loop.
    // First, calculate the score from the current player's perspective.

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

    // This comment explains the final, non-standard step of the function.
    // Flip the score to match the original function's opponent-centric perspective.
    
    // Multiply the standard score by -1 to invert it.
    // This provides a score from the opponent's point of view, matching the quirky original function.
    total_score * -1
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
    use rand::prelude::*;
    use shakmaty::fen;

    #[test]
    fn test_evaluate() {
        let position = Chess::default();
        let evaluation = evaluate(&position);
        assert_eq!(evaluation, 0);
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
        let best_move = next_move(&position);
        assert!(position.legal_moves().contains(&best_move));
    }

    /// This test verifies that `evaluate` and `evaluate_simple` are functionally equivalent.
    #[test]
    fn test_evaluations_are_equivalent() {
        // 1. Initialize dependencies
        let mut rng = thread_rng();
        for i in 0..1_000_000 {
            let mut pos = Chess::default();

            // 2. Generate a random position by playing a series of random legal moves
            // We play 40 half-moves (20 full moves) to get a reasonably complex position.
            for _ in 0..=rng.gen_range(40..=80) {
                let moves = pos.legal_moves();
                if moves.is_empty() {
                    // Game is over (checkmate or stalemate), stop making moves.
                    break;
                }

                // Select a random move from the list of legal moves
                if let Some(random_move) = moves.choose(&mut rng) {
                    pos.play_unchecked(*random_move);
                }
            }

            // 3. Get the FEN representation for debugging purposes.
            // If the test fails, this will be printed, allowing you to replicate the exact position.
            let fen = fen::Fen::from_position(&pos, shakmaty::EnPassantMode::Legal).to_string();

            // 4. Evaluate the final random position with both functions
            let score_original = evaluate(&pos);
            let score_simple = evaluate_simple(&pos);

            // 5. Assert that the scores are identical
            assert_eq!(
                score_original, score_simple,
                "\n{} Evaluation functions returned different scores for the same position.\n  - FEN: {}\n  - Original Score: {}\n  - Simple Score: {}",
                i, fen, score_original, score_simple
            );
        }
    }
}
