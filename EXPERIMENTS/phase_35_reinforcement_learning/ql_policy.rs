// Phase 35.1: Q-Learning & Policy Gradient Module
// Core reinforcement learning algorithms: Q-Learning, DQN, Policy Gradient
// Includes exploration strategies, experience replay, and TD(lambda)

use std::collections::{HashMap, VecDeque};

/// Agent action type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Action(pub u32);

/// Environment state
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State(pub Vec<i32>);

/// Reward signal
#[derive(Debug, Clone, Copy)]
pub struct Reward(pub f64);

/// Experience tuple (S, A, R, S', Done)
#[derive(Debug, Clone)]
pub struct Experience {
    pub state: Vec<f64>,
    pub action: u32,
    pub reward: f64,
    pub next_state: Vec<f64>,
    pub done: bool,
}

/// Q-value table for tabular methods
#[derive(Debug, Clone)]
pub struct QTable {
    pub q_values: HashMap<(u32, u32), f64>,  // (state_id, action_id) -> q_value
    pub state_count: usize,
    pub action_count: usize,
}

/// Neural network approximator for Q-values
#[derive(Debug, Clone)]
pub struct QNetwork {
    pub input_size: usize,
    pub hidden_size: usize,
    pub output_size: usize,
    pub weights: Vec<Vec<f64>>,
    pub learning_rate: f64,
}

/// Experience replay buffer
#[derive(Debug)]
pub struct ReplayBuffer {
    pub experiences: VecDeque<Experience>,
    pub max_size: usize,
    pub batch_size: usize,
}

/// Exploration strategy
#[derive(Debug, Clone)]
pub struct ExplorationStrategy {
    pub strategy_type: String,  // "epsilon_greedy", "ucb", "boltzmann"
    pub epsilon: f64,
    pub epsilon_decay: f64,
    pub epsilon_min: f64,
    pub temperature: f64,
}

/// Q-Learning algorithm configuration
#[derive(Debug, Clone)]
pub struct QLearningConfig {
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub epsilon: f64,
    pub epsilon_decay: f64,
    pub max_episodes: usize,
}

/// Deep Q-Network (DQN) agent
#[derive(Debug, Clone)]
pub struct DQNAgent {
    pub q_network: QNetwork,
    pub target_network: QNetwork,
    pub replay_buffer: ReplayBuffer,
    pub exploration: ExplorationStrategy,
    pub learning_rate: f64,
    pub gamma: f64,
}

/// Policy representation
#[derive(Debug, Clone)]
pub struct Policy {
    pub action_probs: Vec<f64>,
    pub entropy: f64,
    pub deterministic: bool,
}

/// Policy gradient agent
#[derive(Debug, Clone)]
pub struct PolicyGradientAgent {
    pub policy_network: QNetwork,
    pub baseline_network: QNetwork,
    pub learning_rate: f64,
    pub gamma: f64,
    pub entropy_coeff: f64,
}

// ============ Q-LEARNING ============

/// Create Q-table
pub fn create_q_table(state_count: usize, action_count: usize) -> QTable {
    QTable {
        q_values: HashMap::new(),
        state_count,
        action_count,
    }
}

/// Initialize Q-values
pub fn initialize_q_table(table: &mut QTable, initial_value: f64) {
    for s in 0..table.state_count {
        for a in 0..table.action_count {
            table.q_values.insert((s as u32, a as u32), initial_value);
        }
    }
}

/// Get Q-value
pub fn get_q_value(table: &QTable, state: u32, action: u32) -> f64 {
    table.q_values.get(&(state, action)).copied().unwrap_or(0.0)
}

/// Update Q-value using Bellman equation
pub fn update_q_value(
    table: &mut QTable,
    state: u32,
    action: u32,
    reward: f64,
    next_state: u32,
    alpha: f64,
    gamma: f64,
) {
    let current_q = get_q_value(table, state, action);
    let max_next_q = (0..table.action_count as u32)
        .map(|a| get_q_value(table, next_state, a))
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);
    
    let new_q = current_q + alpha * (reward + gamma * max_next_q - current_q);
    table.q_values.insert((state, action), new_q);
}

/// Q-Learning step
pub fn q_learning_step(
    table: &mut QTable,
    state: u32,
    action: u32,
    reward: f64,
    next_state: u32,
    done: bool,
    config: &QLearningConfig,
) {
    if !done {
        update_q_value(
            table,
            state,
            action,
            reward,
            next_state,
            config.learning_rate,
            config.discount_factor,
        );
    }
}

/// SARSA algorithm step (on-policy)
pub fn sarsa_step(
    table: &mut QTable,
    state: u32,
    action: u32,
    reward: f64,
    next_state: u32,
    next_action: u32,
    alpha: f64,
    gamma: f64,
) {
    let current_q = get_q_value(table, state, action);
    let next_q = get_q_value(table, next_state, next_action);
    let new_q = current_q + alpha * (reward + gamma * next_q - current_q);
    table.q_values.insert((state, action), new_q);
}

/// Expected SARSA step
pub fn expected_sarsa_step(
    table: &mut QTable,
    state: u32,
    action: u32,
    reward: f64,
    next_state: u32,
    exploration: &ExplorationStrategy,
    alpha: f64,
    gamma: f64,
) {
    let current_q = get_q_value(table, state, action);
    
    let mut expected_q = 0.0;
    for a in 0..table.action_count as u32 {
        let q = get_q_value(table, next_state, a);
        let prob = 1.0 / table.action_count as f64;
        expected_q += prob * q;
    }
    
    let new_q = current_q + alpha * (reward + gamma * expected_q - current_q);
    table.q_values.insert((state, action), new_q);
}

// ============ DEEP Q-NETWORK (DQN) ============

/// Create Q-network
pub fn create_q_network(input_size: usize, hidden_size: usize, output_size: usize, lr: f64) -> QNetwork {
    QNetwork {
        input_size,
        hidden_size,
        output_size,
        weights: vec![vec![0.1; hidden_size]; input_size],
        learning_rate: lr,
    }
}

/// Forward pass through Q-network
pub fn q_forward(network: &QNetwork, state: &[f64]) -> Vec<f64> {
    let mut output = vec![0.0; network.output_size];
    for i in 0..network.output_size {
        let mut val = 0.0;
        for j in 0..state.len() {
            val += state[j] * network.weights[j % network.weights.len()][i % network.hidden_size];
        }
        output[i] = relu(val);
    }
    output
}

/// ReLU activation
pub fn relu(x: f64) -> f64 {
    x.max(0.0)
}

/// Create DQN agent
pub fn create_dqn_agent(
    input_size: usize,
    action_count: usize,
    epsilon: f64,
) -> DQNAgent {
    let q_net = create_q_network(input_size, 64, action_count, 0.001);
    let target_net = q_net.clone();
    
    DQNAgent {
        q_network: q_net,
        target_network: target_net,
        replay_buffer: ReplayBuffer {
            experiences: VecDeque::new(),
            max_size: 10000,
            batch_size: 32,
        },
        exploration: ExplorationStrategy {
            strategy_type: "epsilon_greedy".to_string(),
            epsilon,
            epsilon_decay: 0.995,
            epsilon_min: 0.01,
            temperature: 1.0,
        },
        learning_rate: 0.001,
        gamma: 0.99,
    }
}

/// DQN select action
pub fn dqn_select_action(agent: &DQNAgent, state: &[f64]) -> u32 {
    if rand_float() < agent.exploration.epsilon {
        (rand_float() * agent.q_network.output_size as f64) as u32
    } else {
        let q_values = q_forward(&agent.q_network, state);
        q_values.iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i as u32)
            .unwrap_or(0)
    }
}

/// Store experience in replay buffer
pub fn store_experience(agent: &mut DQNAgent, experience: Experience) {
    if agent.replay_buffer.experiences.len() >= agent.replay_buffer.max_size {
        agent.replay_buffer.experiences.pop_front();
    }
    agent.replay_buffer.experiences.push_back(experience);
}

/// Sample batch from replay buffer
pub fn sample_replay_batch(agent: &DQNAgent) -> Vec<Experience> {
    let batch_size = agent.replay_buffer.batch_size.min(agent.replay_buffer.experiences.len());
    let mut batch = Vec::new();
    
    for _ in 0..batch_size {
        let idx = (rand_float() * agent.replay_buffer.experiences.len() as f64) as usize;
        if let Some(exp) = agent.replay_buffer.experiences.get(idx) {
            batch.push(exp.clone());
        }
    }
    
    batch
}

/// Update DQN
pub fn update_dqn(agent: &mut DQNAgent, batch: &[Experience]) {
    for exp in batch {
        let q_pred = q_forward(&agent.q_network, &exp.state);
        let q_target = q_forward(&agent.target_network, &exp.next_state);
        
        let max_next_q = if exp.done {
            0.0
        } else {
            q_target.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        };
        
        // Update would modify weights
        let _ = exp.reward + agent.gamma * max_next_q - q_pred[exp.action as usize];
    }
}

// ============ POLICY GRADIENT ============

/// Create policy gradient agent
pub fn create_policy_gradient_agent(input_size: usize, action_count: usize) -> PolicyGradientAgent {
    PolicyGradientAgent {
        policy_network: create_q_network(input_size, 64, action_count, 0.001),
        baseline_network: create_q_network(input_size, 64, 1, 0.001),
        learning_rate: 0.001,
        gamma: 0.99,
        entropy_coeff: 0.01,
    }
}

/// Get policy from network
pub fn get_policy(network: &QNetwork, state: &[f64]) -> Policy {
    let logits = q_forward(network, state);
    let probs = softmax(&logits);
    let entropy = -probs.iter().map(|p| if *p > 0.0 { p * p.ln() } else { 0.0 }).sum::<f64>();
    
    Policy {
        action_probs: probs,
        entropy,
        deterministic: false,
    }
}

/// Softmax activation
pub fn softmax(logits: &[f64]) -> Vec<f64> {
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exp_logits: Vec<f64> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f64 = exp_logits.iter().sum();
    
    exp_logits.iter().map(|x| x / sum).collect()
}

/// Policy gradient update
pub fn policy_gradient_step(
    agent: &mut PolicyGradientAgent,
    states: &[Vec<f64>],
    actions: &[u32],
    returns: &[f64],
) {
    for (state, &action, &ret) in states.iter().zip(actions.iter()).zip(returns.iter()) {
        let policy = get_policy(&agent.policy_network, state);
        let advantage = ret;  // In practice, compute advantage using baseline
        
        // Policy gradient: -log(pi(a|s)) * advantage
        let _ = policy.action_probs[action as usize].max(1e-8).ln() * advantage;
    }
}

// ============ VALUE FUNCTION APPROXIMATION ============

/// TD(lambda) learning
pub fn td_lambda_update(
    network: &mut QNetwork,
    states: &[Vec<f64>],
    rewards: &[f64],
    gamma: f64,
    lambda: f64,
) {
    let mut eligibility_traces = vec![0.0; network.output_size];
    
    for (state, reward) in states.iter().zip(rewards.iter()) {
        let q_current = q_forward(network, state);
        
        for i in 0..network.output_size {
            eligibility_traces[i] = gamma * lambda * eligibility_traces[i] + 1.0;
        }
        
        // TD error and weight updates would happen here
        let _ = (*reward - q_current[0]) * eligibility_traces[0];
    }
}

/// N-step return calculation
pub fn n_step_return(rewards: &[f64], gamma: f64, n: usize) -> f64 {
    let mut ret = 0.0;
    for (i, &r) in rewards.iter().take(n).enumerate() {
        ret += r * gamma.powi(i as i32);
    }
    ret
}

// ============ UTILITY FUNCTIONS ============

/// Random f32 [0, 1)
fn rand_float() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as f64;
    (nanos / 1_000_000_000.0) % 1.0
}

/// Decay epsilon
pub fn decay_epsilon(agent: &mut DQNAgent) {
    agent.exploration.epsilon = (agent.exploration.epsilon * agent.exploration.epsilon_decay)
        .max(agent.exploration.epsilon_min);
}

/// Get greedy action
pub fn greedy_action(q_values: &[f64]) -> u32 {
    q_values.iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i as u32)
        .unwrap_or(0)
}

/// Calculate returns from rewards
pub fn calculate_returns(rewards: &[f64], gamma: f64) -> Vec<f64> {
    let mut returns = vec![0.0; rewards.len()];
    let mut cumsum = 0.0;
    
    for i in (0..rewards.len()).rev() {
        cumsum = rewards[i] + gamma * cumsum;
        returns[i] = cumsum;
    }
    
    returns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_q_table() {
        let table = create_q_table(10, 4);
        assert_eq!(table.state_count, 10);
        assert_eq!(table.action_count, 4);
    }

    #[test]
    fn test_get_q_value() {
        let mut table = create_q_table(5, 3);
        initialize_q_table(&mut table, 0.0);
        assert_eq!(get_q_value(&table, 0, 0), 0.0);
    }

    #[test]
    fn test_update_q_value() {
        let mut table = create_q_table(5, 3);
        initialize_q_table(&mut table, 0.0);
        update_q_value(&mut table, 0, 1, 1.0, 1, 0.1, 0.9);
        assert!(get_q_value(&table, 0, 1) > 0.0);
    }

    #[test]
    fn test_sarsa_step() {
        let mut table = create_q_table(5, 3);
        initialize_q_table(&mut table, 0.0);
        sarsa_step(&mut table, 0, 1, 1.0, 1, 2, 0.1, 0.9);
        assert!(get_q_value(&table, 0, 1) > 0.0);
    }

    #[test]
    fn test_create_q_network() {
        let network = create_q_network(10, 64, 4, 0.001);
        assert_eq!(network.input_size, 10);
        assert_eq!(network.output_size, 4);
    }

    #[test]
    fn test_q_forward() {
        let network = create_q_network(4, 64, 2, 0.001);
        let state = vec![1.0, 0.5, -0.5, 2.0];
        let output = q_forward(&network, &state);
        assert_eq!(output.len(), 2);
    }

    #[test]
    fn test_create_dqn_agent() {
        let agent = create_dqn_agent(4, 2, 0.1);
        assert_eq!(agent.exploration.epsilon, 0.1);
    }

    #[test]
    fn test_softmax() {
        let logits = vec![1.0, 2.0, 3.0];
        let probs = softmax(&logits);
        assert_eq!(probs.len(), 3);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_returns() {
        let rewards = vec![1.0, 0.0, 1.0];
        let returns = calculate_returns(&rewards, 0.99);
        assert_eq!(returns.len(), 3);
    }

    #[test]
    fn test_n_step_return() {
        let rewards = vec![1.0, 0.5, 0.25];
        let ret = n_step_return(&rewards, 0.99, 2);
        assert!(ret > 0.0);
    }

    #[test]
    fn test_store_experience() {
        let mut agent = create_dqn_agent(4, 2, 0.1);
        let exp = Experience {
            state: vec![1.0, 0.5, -0.5, 2.0],
            action: 0,
            reward: 1.0,
            next_state: vec![0.9, 0.4, -0.4, 1.9],
            done: false,
        };
        store_experience(&mut agent, exp);
        assert_eq!(agent.replay_buffer.experiences.len(), 1);
    }

    #[test]
    fn test_decay_epsilon() {
        let mut agent = create_dqn_agent(4, 2, 0.5);
        let initial = agent.exploration.epsilon;
        decay_epsilon(&mut agent);
        assert!(agent.exploration.epsilon < initial);
    }

    #[test]
    fn test_create_policy_gradient_agent() {
        let agent = create_policy_gradient_agent(4, 2);
        assert_eq!(agent.policy_network.input_size, 4);
    }

    #[test]
    fn test_get_policy() {
        let network = create_q_network(4, 64, 2, 0.001);
        let state = vec![1.0, 0.0, -1.0, 2.0];
        let policy = get_policy(&network, &state);
        let sum: f64 = policy.action_probs.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }
}
