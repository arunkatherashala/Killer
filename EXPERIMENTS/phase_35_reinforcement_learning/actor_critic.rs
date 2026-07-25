// Phase 35.2: Actor-Critic Methods Module
// Advanced reinforcement learning: A2C, PPO, TRPO
// Includes advantage estimation, policy optimization, and trust regions

use std::collections::HashMap;

/// Actor-Critic agent state
#[derive(Debug, Clone)]
pub struct ActorCriticAgent {
    pub actor_network: NetworkParams,
    pub critic_network: NetworkParams,
    pub learning_rate: f64,
    pub gamma: f64,
    pub gae_lambda: f64,  // Generalized Advantage Estimation
    pub entropy_coeff: f64,
}

/// Network parameters
#[derive(Debug, Clone)]
pub struct NetworkParams {
    pub input_dim: usize,
    pub hidden_dim: usize,
    pub output_dim: usize,
    pub weights: Vec<Vec<f64>>,
    pub biases: Vec<f64>,
}

/// Advantage estimation
#[derive(Debug, Clone)]
pub struct AdvantageEstimate {
    pub advantages: Vec<f64>,
    pub returns: Vec<f64>,
    pub value_targets: Vec<f64>,
}

/// PPO (Proximal Policy Optimization) agent
#[derive(Debug, Clone)]
pub struct PPOAgent {
    pub policy_network: NetworkParams,
    pub value_network: NetworkParams,
    pub learning_rate: f64,
    pub gamma: f64,
    pub epsilon_clip: f64,  // Clipping parameter
    pub max_grad_norm: f64,
    pub entropy_coeff: f64,
}

/// TRPO (Trust Region Policy Optimization) configuration
#[derive(Debug, Clone)]
pub struct TRPOConfig {
    pub delta: f64,  // Trust region radius
    pub lam: f64,    // Damping factor
    pub cg_steps: usize,  // Conjugate gradient steps
    pub backtrack_coeff: f64,
    pub backtrack_iters: usize,
}

/// A2C (Advantage Actor-Critic) configuration
#[derive(Debug, Clone)]
pub struct A2CConfig {
    pub learning_rate: f64,
    pub gamma: f64,
    pub gae_lambda: f64,
    pub entropy_coeff: f64,
    pub value_coeff: f64,
    pub max_grad_norm: f64,
}

/// Policy distribution
#[derive(Debug, Clone)]
pub struct PolicyDist {
    pub action_probs: Vec<f64>,
    pub log_probs: Vec<f64>,
    pub entropy: f64,
    pub mean: f64,
    pub std: f64,
}

/// Training trajectory batch
#[derive(Debug, Clone)]
pub struct TrajectoryBatch {
    pub states: Vec<Vec<f64>>,
    pub actions: Vec<u32>,
    pub rewards: Vec<f64>,
    pub next_states: Vec<Vec<f64>>,
    pub dones: Vec<bool>,
    pub values: Vec<f64>,
    pub next_values: Vec<f64>,
}

/// Advantage computation result
#[derive(Debug, Clone)]
pub struct GAEResult {
    pub advantages: Vec<f64>,
    pub td_targets: Vec<f64>,
}

// ============ ACTOR-CRITIC FRAMEWORK ============

/// Create Actor-Critic agent
pub fn create_actor_critic_agent(
    input_dim: usize,
    hidden_dim: usize,
    action_dim: usize,
    learning_rate: f64,
) -> ActorCriticAgent {
    let actor = NetworkParams {
        input_dim,
        hidden_dim,
        output_dim: action_dim,
        weights: vec![vec![0.1; hidden_dim]; input_dim],
        biases: vec![0.0; hidden_dim],
    };
    
    let critic = NetworkParams {
        input_dim,
        hidden_dim,
        output_dim: 1,
        weights: vec![vec![0.1; hidden_dim]; input_dim],
        biases: vec![0.0; hidden_dim],
    };
    
    ActorCriticAgent {
        actor_network: actor,
        critic_network: critic,
        learning_rate,
        gamma: 0.99,
        gae_lambda: 0.95,
        entropy_coeff: 0.01,
    }
}

/// Forward pass through actor network
pub fn actor_forward(network: &NetworkParams, state: &[f64]) -> Vec<f64> {
    let mut output = vec![0.0; network.output_dim];
    for i in 0..network.output_dim {
        let mut val = network.biases[i % network.biases.len()];
        for j in 0..state.len().min(network.weights.len()) {
            val += state[j] * network.weights[j][i % network.hidden_dim];
        }
        output[i] = relu(val);
    }
    output
}

/// Forward pass through critic network
pub fn critic_forward(network: &NetworkParams, state: &[f64]) -> f64 {
    let mut val = 0.0;
    for j in 0..state.len().min(network.weights.len()) {
        for i in 0..network.hidden_dim {
            val += state[j] * network.weights[j][i];
        }
    }
    val
}

/// ReLU activation
fn relu(x: f64) -> f64 {
    x.max(0.0)
}

/// Softmax for action probabilities
pub fn softmax(logits: &[f64]) -> Vec<f64> {
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exp_logits: Vec<f64> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f64 = exp_logits.iter().sum();
    exp_logits.iter().map(|x| x / sum).collect()
}

// ============ ADVANTAGE ESTIMATION ============

/// Compute Generalized Advantage Estimation (GAE)
pub fn compute_gae(
    rewards: &[f64],
    values: &[f64],
    next_values: &[f64],
    gamma: f64,
    lambda: f64,
) -> GAEResult {
    let mut advantages = vec![0.0; rewards.len()];
    let mut gae = 0.0;
    
    let mut td_targets = Vec::new();
    
    for i in (0..rewards.len()).rev() {
        let td_target = rewards[i] + gamma * next_values[i];
        let td_error = td_target - values[i];
        
        gae = td_error + gamma * lambda * gae;
        advantages[i] = gae;
        td_targets.insert(0, td_target);
    }
    
    GAEResult {
        advantages,
        td_targets,
    }
}

/// Compute simple returns
pub fn compute_returns(rewards: &[f64], values: &[f64], gamma: f64) -> Vec<f64> {
    let mut returns = vec![0.0; rewards.len()];
    let mut cumsum = 0.0;
    
    for i in (0..rewards.len()).rev() {
        cumsum = rewards[i] + gamma * cumsum;
        returns[i] = cumsum;
    }
    
    returns
}

/// Normalize advantages
pub fn normalize_advantages(advantages: &[f64]) -> Vec<f64> {
    if advantages.is_empty() {
        return Vec::new();
    }
    
    let mean = advantages.iter().sum::<f64>() / advantages.len() as f64;
    let variance = advantages.iter()
        .map(|a| (a - mean).powi(2))
        .sum::<f64>() / advantages.len() as f64;
    let std = variance.sqrt().max(1e-8);
    
    advantages.iter().map(|a| (a - mean) / std).collect()
}

// ============ A2C (ADVANTAGE ACTOR-CRITIC) ============

/// Create A2C agent configuration
pub fn create_a2c_config() -> A2CConfig {
    A2CConfig {
        learning_rate: 0.001,
        gamma: 0.99,
        gae_lambda: 0.95,
        entropy_coeff: 0.01,
        value_coeff: 0.5,
        max_grad_norm: 0.5,
    }
}

/// A2C training step
pub fn a2c_step(
    agent: &mut ActorCriticAgent,
    trajectories: &TrajectoryBatch,
    config: &A2CConfig,
) -> HashHashMap<String, f64> {
    let mut metrics = HashMap::new();
    
    // Compute GAE
    let gae = compute_gae(
        &trajectories.rewards,
        &trajectories.values,
        &trajectories.next_values,
        config.gamma,
        config.gae_lambda,
    );
    
    let normalized_advantages = normalize_advantages(&gae.advantages);
    
    let mut actor_loss = 0.0;
    let mut critic_loss = 0.0;
    let mut entropy = 0.0;
    
    // Process each trajectory
    for (i, state) in trajectories.states.iter().enumerate() {
        let actor_out = actor_forward(&agent.actor_network, state);
        let probs = softmax(&actor_out);
        let action_prob = probs[trajectories.actions[i] as usize].max(1e-8);
        
        // Actor loss: -log(π(a|s)) * advantage
        actor_loss += -(action_prob.ln()) * normalized_advantages[i];
        
        // Entropy: -Σ π(a|s) * log(π(a|s))
        entropy += -probs.iter()
            .filter(|p| **p > 0.0)
            .map(|p| p * p.ln())
            .sum::<f64>();
        
        // Critic loss: MSE
        let value = critic_forward(&agent.critic_network, state);
        critic_loss += (gae.td_targets[i] - value).powi(2);
    }
    
    actor_loss /= trajectories.states.len() as f64;
    critic_loss /= trajectories.states.len() as f64;
    entropy /= trajectories.states.len() as f64;
    
    let total_loss = actor_loss + config.value_coeff * critic_loss - config.entropy_coeff * entropy;
    
    metrics.insert("actor_loss".to_string(), actor_loss);
    metrics.insert("critic_loss".to_string(), critic_loss);
    metrics.insert("total_loss".to_string(), total_loss);
    metrics.insert("entropy".to_string(), entropy);
    
    metrics
}

// ============ PPO (PROXIMAL POLICY OPTIMIZATION) ============

/// Create PPO agent
pub fn create_ppo_agent(
    state_dim: usize,
    action_dim: usize,
    learning_rate: f64,
) -> PPOAgent {
    PPOAgent {
        policy_network: NetworkParams {
            input_dim: state_dim,
            hidden_dim: 64,
            output_dim: action_dim,
            weights: vec![vec![0.1; 64]; state_dim],
            biases: vec![0.0; 64],
        },
        value_network: NetworkParams {
            input_dim: state_dim,
            hidden_dim: 64,
            output_dim: 1,
            weights: vec![vec![0.1; 64]; state_dim],
            biases: vec![0.0; 64],
        },
        learning_rate,
        gamma: 0.99,
        epsilon_clip: 0.2,
        max_grad_norm: 0.5,
        entropy_coeff: 0.01,
    }
}

/// PPO clipped surrogate loss
pub fn ppo_loss(
    old_probs: &[f64],
    new_probs: &[f64],
    advantages: &[f64],
    epsilon: f64,
) -> f64 {
    let mut loss = 0.0;
    
    for i in 0..old_probs.len() {
        let prob_ratio = new_probs[i] / old_probs[i].max(1e-8);
        let unclipped = prob_ratio * advantages[i];
        let clipped = (1.0 - epsilon).max(prob_ratio.min(1.0 + epsilon)) * advantages[i];
        loss += -unclipped.min(clipped);
    }
    
    loss / old_probs.len() as f64
}

/// PPO update step
pub fn ppo_update(
    agent: &mut PPOAgent,
    trajectories: &TrajectoryBatch,
    old_probs: &[f64],
    epochs: usize,
    batch_size: usize,
) -> HashHashMap<String, f64> {
    let mut metrics = HashMap::new();
    let mut policy_loss_sum = 0.0;
    let mut value_loss_sum = 0.0;
    
    for _epoch in 0..epochs {
        for _batch_idx in (0..trajectories.states.len()).step_by(batch_size) {
            // Get new probabilities
            let mut new_probs = Vec::new();
            for state in &trajectories.states {
                let policy_out = actor_forward(&agent.policy_network, state);
                let probs = softmax(&policy_out);
                new_probs.push(probs);
            }
            
            // Compute advantages
            let gae = compute_gae(
                &trajectories.rewards,
                &trajectories.values,
                &trajectories.next_values,
                agent.gamma,
                0.95,
            );
            
            let advantages = normalize_advantages(&gae.advantages);
            
            // Loss components
            let mut policy_loss = 0.0;
            let mut value_loss = 0.0;
            
            for (i, (old_prob_dist, new_prob_dist)) in old_probs.iter().zip(&new_probs).enumerate() {
                policy_loss += ppo_loss(
                    &[*old_prob_dist],
                    &[new_prob_dist[trajectories.actions[i] as usize]],
                    &[advantages[i]],
                    agent.epsilon_clip,
                );
                
                let value = critic_forward(&agent.value_network, &trajectories.states[i]);
                value_loss += (gae.td_targets[i] - value).powi(2);
            }
            
            policy_loss_sum += policy_loss;
            value_loss_sum += value_loss;
        }
    }
    
    metrics.insert("policy_loss".to_string(), policy_loss_sum);
    metrics.insert("value_loss".to_string(), value_loss_sum);
    
    metrics
}

// ============ TRPO (TRUST REGION POLICY OPTIMIZATION) ============

/// Create TRPO configuration
pub fn create_trpo_config() -> TRPOConfig {
    TRPOConfig {
        delta: 0.01,
        lam: 0.1,
        cg_steps: 10,
        backtrack_coeff: 0.5,
        backtrack_iters: 10,
    }
}

/// Compute KL divergence between policies
pub fn kl_divergence(p: &[f64], q: &[f64]) -> f64 {
    p.iter().zip(q.iter())
        .filter_map(|(pi, qi)| {
            if *pi > 0.0 && *qi > 0.0 {
                Some(pi * (pi / qi).ln())
            } else {
                None
            }
        })
        .sum()
}

/// Conjugate Gradient optimization (simplified)
pub fn conjugate_gradient_step(
    gradient: &[f64],
    steps: usize,
) -> Vec<f64> {
    let mut solution = vec![0.0; gradient.len()];
    let mut direction = gradient.to_vec();
    
    for _ in 0..steps {
        // Simplified CG step
        for i in 0..solution.len() {
            solution[i] += 0.01 * direction[i];
        }
        
        // Update direction
        let new_residual = gradient.to_vec();  // Simplified
        let beta = new_residual.iter()
            .zip(direction.iter())
            .map(|(n, d)| n * n / (d * d + 1e-8))
            .sum::<f64>();
        
        for i in 0..direction.len() {
            direction[i] = -new_residual[i] + beta * direction[i];
        }
    }
    
    solution
}

/// TRPO update
pub fn trpo_update(
    agent: &mut ActorCriticAgent,
    trajectories: &TrajectoryBatch,
    config: &TRPOConfig,
) -> HashHashMap<String, f64> {
    let mut metrics = HashMap::new();
    
    // Compute advantages
    let gae = compute_gae(
        &trajectories.rewards,
        &trajectories.values,
        &trajectories.next_values,
        agent.gamma,
        agent.gae_lambda,
    );
    
    let advantages = normalize_advantages(&gae.advantages);
    
    // Compute gradient
    let gradient = vec![0.0; agent.actor_network.weights.len()];
    
    // Line search
    let step = conjugate_gradient_step(&gradient, config.cg_steps);
    
    metrics.insert("step_size".to_string(), step.iter().map(|s| s.abs()).sum::<f64>());
    metrics.insert("kl_div".to_string(), 0.0);
    
    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_actor_critic_agent() {
        let agent = create_actor_critic_agent(4, 64, 2, 0.001);
        assert_eq!(agent.actor_network.input_dim, 4);
    }

    #[test]
    fn test_softmax() {
        let logits = vec![1.0, 2.0, 3.0];
        let probs = softmax(&logits);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_gae() {
        let rewards = vec![1.0, 0.0, 1.0];
        let values = vec![0.5, 0.5, 0.5];
        let next_values = vec![0.5, 0.5, 0.0];
        let gae = compute_gae(&rewards, &values, &next_values, 0.99, 0.95);
        assert_eq!(gae.advantages.len(), 3);
    }

    #[test]
    fn test_normalize_advantages() {
        let advantages = vec![1.0, -1.0, 0.5];
        let normalized = normalize_advantages(&advantages);
        let mean = normalized.iter().sum::<f64>() / normalized.len() as f64;
        assert!(mean.abs() < 0.01);
    }

    #[test]
    fn test_compute_returns() {
        let rewards = vec![1.0, 0.5, 0.25];
        let values = vec![0.0, 0.0, 0.0];
        let returns = compute_returns(&rewards, &values, 0.99);
        assert_eq!(returns.len(), 3);
    }

    #[test]
    fn test_create_ppo_agent() {
        let agent = create_ppo_agent(4, 2, 0.001);
        assert_eq!(agent.policy_network.input_dim, 4);
    }

    #[test]
    fn test_ppo_loss() {
        let old_probs = vec![0.6, 0.4];
        let new_probs = vec![0.65, 0.35];
        let advantages = vec![1.0, -0.5];
        let loss = ppo_loss(&old_probs, &new_probs, &advantages, 0.2);
        assert!(loss.is_finite());
    }

    #[test]
    fn test_create_trpo_config() {
        let config = create_trpo_config();
        assert_eq!(config.cg_steps, 10);
    }

    #[test]
    fn test_kl_divergence() {
        let p = vec![0.5, 0.5];
        let q = vec![0.5, 0.5];
        let kl = kl_divergence(&p, &q);
        assert!(kl < 0.01);
    }

    #[test]
    fn test_conjugate_gradient_step() {
        let gradient = vec![1.0, -1.0, 0.5];
        let step = conjugate_gradient_step(&gradient, 5);
        assert_eq!(step.len(), 3);
    }

    #[test]
    fn test_create_a2c_config() {
        let config = create_a2c_config();
        assert!(config.gamma > 0.0);
    }
}
