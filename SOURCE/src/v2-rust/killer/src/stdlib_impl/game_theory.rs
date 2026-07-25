// ================================================================
// GAME THEORY SOLVER - Phase 21.3
// Strategic analysis, Nash equilibrium, cooperative games
// Ported from: solver_game_theory.killer
// ================================================================

use std::collections::HashMap;

pub type Vector = Vec<f64>;
pub type Matrix = Vec<Vec<f64>>;

/// Game Theory Solver
pub struct GameTheorySolver;

impl GameTheorySolver {
    // ================================================================
    // NASH EQUILIBRIUM (1-20)
    // ================================================================

    /// Problem 1: Pure Strategy Nash Equilibrium (2x2)
    pub fn nash_2x2(payoff: &[[f64; 2]; 2]) -> (usize, usize) {
        // Simplified: find dominant strategies
        let (p1_u, p1_d) = (payoff[0][0], payoff[1][0]);
        let (p2_l, p2_r) = (payoff[0][0], payoff[0][1]);
        
        if p1_u >= p1_d && p2_l >= p2_r { (0, 0) }
        else if p1_d >= p1_u && p2_r >= p2_l { (1, 1) }
        else if p1_u >= p1_d && p2_r >= p2_l { (0, 1) }
        else { (1, 0) }
    }

    /// Problem 2: Mixed Strategy - Matching Pennies
    pub fn mixed_strategy_symmetric() -> f64 {
        0.5  // For symmetric games
    }

    /// Problem 3: Best Response Function
    pub fn best_response(opponent_q: f64, coeff: f64) -> f64 {
        (coeff * opponent_q).max(0.0)
    }

    /// Problem 4: Payoff Evaluation
    pub fn payoff_mixed(own_strat: &[f64], opp_strat: &[f64], matrix: &Matrix) -> f64 {
        let mut payoff = 0.0;
        for i in 0..own_strat.len() {
            for j in 0..opp_strat.len() {
                if i < matrix.len() && j < matrix[i].len() {
                    payoff += matrix[i][j] * own_strat[i] * opp_strat[j];
                }
            }
        }
        payoff
    }

    /// Problem 5: Dominant Strategy Check
    pub fn is_dominant(strategy: usize, payoff_matrix: &Matrix) -> bool {
        if strategy >= payoff_matrix.len() { return false; }
        
        let strategy_payoff = &payoff_matrix[strategy];
        for other_payoff in payoff_matrix.iter() {
            if other_payoff != strategy_payoff {
                let dominated = strategy_payoff.iter().zip(other_payoff.iter())
                    .all(|(s, o)| s >= o);
                if !dominated { return false; }
            }
        }
        true
    }

    /// Problem 6: Iterated Elimination of Dominated Strategies
    pub fn eliminate_dominated(matrix: &Matrix) -> Matrix {
        let mut m = matrix.clone();
        let mut changed = true;
        
        while changed {
            changed = false;
            let mut new_m = Vec::new();
            for row in m.iter() {
                let is_dominated = m.iter()
                    .any(|other| other != row && other.iter().zip(row.iter()).all(|(o, s)| o >= s));
                if is_dominated {
                    changed = true;
                } else {
                    new_m.push(row.clone());
                }
            }
            m = new_m;
        }
        m
    }

    // ================================================================
    // COOPERATIVE GAMES (7-15)
    // ================================================================

    /// Problem 7: Shapley Value
    pub fn shapley_value(player: usize, coalition_values: &[f64]) -> f64 {
        if coalition_values.is_empty() { return 0.0; }
        coalition_values[player] / coalition_values.len() as f64
    }

    /// Problem 8: Core Feasibility Check
    pub fn is_in_core(imputation: &[f64], coalition_values: &Matrix) -> bool {
        if coalition_values.is_empty() { return false; }
        
        let total: f64 = imputation.iter().sum();
        let expected: f64 = coalition_values[0].iter().sum::<f64>() / coalition_values[0].len() as f64;
        
        (total - expected).abs() < 1e-10
    }

    /// Problem 9: Banzhaf Power Index
    pub fn banzhaf_power_index(player: usize, num_players: usize) -> f64 {
        // Simplified: equal weight divided by number of players
        1.0 / num_players as f64
    }

    // ================================================================
    // AUCTION THEORY (10-25)
    // ================================================================

    /// Problem 10: First-Price Sealed Bid - Symmetric Equilibrium
    pub fn first_price_equilibrium_bid(valuation: f64, num_bidders: usize) -> f64 {
        if num_bidders <= 1 { return valuation; }
        // Equilibrium bid: v · (n-1)/n
        valuation * (num_bidders - 1) as f64 / num_bidders as f64
    }

    /// Problem 11: Second-Price Sealed Bid (Vickrey)
    pub fn second_price_equilibrium_bid(valuation: f64) -> f64 {
        // Truthful bidding is dominant strategy
        valuation
    }

    /// Problem 12: English Auction Winner
    pub fn english_auction_winner(bids: &[f64]) -> (usize, f64) {
        if bids.is_empty() { return (0, 0.0); }
        let max_idx = bids.iter().enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);
        
        // Winner pays second highest
        let mut sorted = bids.to_vec();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let price = if sorted.len() > 1 { sorted[1] } else { sorted[0] };
        
        (max_idx, price)
    }

    // ================================================================
    // EVOLUTIONARY GAME THEORY (16-30)
    // ================================================================

    /// Problem 13: Replicator Dynamics
    pub fn replicator_dynamics(frequencies: &[f64], payoffs: &[f64]) -> Vec<f64> {
        if frequencies.is_empty() { return vec![]; }
        
        let avg_payoff: f64 = frequencies.iter().zip(payoffs.iter())
            .map(|(f, p)| f * p)
            .sum();
        
        frequencies.iter().zip(payoffs.iter())
            .map(|(f, p)| if avg_payoff > 0.0 { f * p / avg_payoff } else { *f })
            .collect()
    }

    /// Problem 14: ESS (Evolutionarily Stable Strategy)
    pub fn is_ess(strategy: f64, payoff_vs_self: f64, payoff_vs_mutant: f64, 
                  mutant_vs_self: f64, mutant_vs_mutant: f64) -> bool {
        // ESS if: (payoff_vs_self > mutant_vs_self) OR 
        //         (payoff_vs_self == mutant_vs_self AND payoff_vs_mutant > mutant_vs_mutant)
        payoff_vs_self > mutant_vs_self || 
        (payoff_vs_self.abs() - mutant_vs_self.abs() < 1e-10 && payoff_vs_mutant > mutant_vs_mutant)
    }

    /// Problem 15: Hawk-Dove Equilibrium Frequency
    pub fn hawk_dove_frequency(hawk_payoff: f64, dove_payoff: f64) -> f64 {
        let total = hawk_payoff + dove_payoff;
        if total.abs() < 1e-14 { return 0.5; }
        hawk_payoff / total
    }

    // ================================================================
    // BARGAINING THEORY (20-30)
    // ================================================================

    /// Problem 16: Nash Bargaining Solution
    pub fn nash_bargain(value1: f64, value2: f64, threat1: f64, threat2: f64) -> (f64, f64) {
        let surplus1 = (value1 - threat1).max(0.0);
        let surplus2 = (value2 - threat2).max(0.0);
        
        let product = surplus1 * surplus2;
        if product <= 0.0 {
            return (threat1, threat2);
        }
        
        // Equal power: split surplus equally
        let share = (product).sqrt() / 2.0;
        (threat1 + share, threat2 + share)
    }

    /// Problem 17: Ultimatum Game - Proposer Offer
    pub fn ultimatum_proposer_offer(total: f64, responder_reservation: f64) -> f64 {
        // Rational proposer offers minimum to responder to accept
        responder_reservation.min(total)
    }

    // ================================================================
    // VOTING THEORY (25-40)
    // ================================================================

    /// Problem 18: Plurality Voting Winner
    pub fn plurality_winner(votes: &[usize]) -> (usize, usize) {
        if votes.is_empty() { return (0, 0); }
        let max_idx = votes.iter().enumerate()
            .max_by_key(|(_, &count)| count)
            .map(|(i, _)| i)
            .unwrap_or(0);
        (max_idx, votes[max_idx])
    }

    /// Problem 19: Borda Count
    pub fn borda_count(preferences: &[Vec<usize>]) -> Vec<usize> {
        if preferences.is_empty() { return vec![]; }
        
        let num_candidates = preferences[0].len();
        let mut scores = vec![0; num_candidates];
        
        for pref in preferences {
            for (rank, &candidate) in pref.iter().enumerate() {
                scores[candidate] += num_candidates - rank;
            }
        }
        scores
    }

    /// Problem 20: Condorcet Winner Check
    pub fn has_condorcet_winner(preferences: &[Vec<usize>]) -> Option<usize> {
        if preferences.is_empty() { return None; }
        let num_candidates = preferences[0].len();
        
        'candidate_loop: for candidate in 0..num_candidates {
            for other in 0..num_candidates {
                if candidate == other { continue; }
                
                let mut wins = 0;
                for pref in preferences {
                    let c_pos = pref.iter().position(|&x| x == candidate).unwrap_or(num_candidates);
                    let o_pos = pref.iter().position(|&x| x == other).unwrap_or(num_candidates);
                    if c_pos < o_pos { wins += 1; }
                }
                
                if wins <= preferences.len() / 2 {
                    continue 'candidate_loop;
                }
            }
            return Some(candidate);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nash_2x2() {
        let payoff = [[3.0, 0.0], [0.0, 2.0]];
        let (p1, p2) = GameTheorySolver::nash_2x2(&payoff);
        assert_eq!(p1, 0);
        assert_eq!(p2, 0);
    }

    #[test]
    fn test_first_price_bid() {
        let bid = GameTheorySolver::first_price_equilibrium_bid(100.0, 2);
        assert!((bid - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_plurality_winner() {
        let votes = vec![3, 5, 2];
        let (winner, count) = GameTheorySolver::plurality_winner(&votes);
        assert_eq!(winner, 1);
        assert_eq!(count, 5);
    }
}
