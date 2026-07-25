// Phase 35.3: Environment & Game Integration Module
// Game environments, multi-agent coordination, curriculum learning
// Supports OpenAI Gym-like interfaces, custom environments, and training

use std::collections::HashMap;

/// Environment observation
#[derive(Debug, Clone)]
pub struct Observation {
    pub state: Vec<f64>,
    pub info: HashHashMap<String, String>,
}

/// Environment action
#[derive(Debug, Clone, Copy)]
pub struct EnvAction(pub u32);

/// Environment step result
#[derive(Debug, Clone)]
pub struct StepResult {
    pub observation: Observation,
    pub reward: f64,
    pub done: bool,
    pub truncated: bool,
    pub info: HashHashMap<String, String>,
}

/// Environment configuration
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    pub name: String,
    pub observation_shape: Vec<usize>,
    pub action_space: usize,
    pub max_episode_steps: u32,
    pub render_mode: String,
}

/// Environment trait implementation
#[derive(Debug)]
pub struct Environment {
    pub config: EnvironmentConfig,
    pub state: Vec<f64>,
    pub episode_step: u32,
    pub episode_reward: f64,
}

/// Game environment (Cartpole, MuJoCo, etc.)
#[derive(Debug, Clone)]
pub struct GameEnvironment {
    pub name: String,
    pub state: Vec<f64>,
    pub state_bounds: (Vec<f64>, Vec<f64>),
    pub action_space_size: usize,
    pub episode_steps: u32,
    pub max_steps: u32,
}

/// Multi-agent environment
#[derive(Debug)]
pub struct MultiAgentEnvironment {
    pub num_agents: usize,
    pub agent_states: Vec<Vec<f64>>,
    pub shared_reward: f64,
    pub agent_rewards: Vec<f64>,
    pub dones: Vec<bool>,
    pub observations: Vec<Observation>,
}

/// Curriculum learning stage
#[derive(Debug, Clone)]
pub struct CurriculumStage {
    pub stage_id: usize,
    pub difficulty: f64,
    pub reward_threshold: f64,
    pub max_episodes: usize,
    pub episode_count: usize,
}

/// Episode statistics
#[derive(Debug, Clone)]
pub struct EpisodeStats {
    pub episode_num: usize,
    pub total_reward: f64,
    pub steps: u32,
    pub max_reward: f64,
    pub min_reward: f64,
    pub win: bool,
}

/// Training episode buffer
#[derive(Debug, Clone)]
pub struct EpisodeBuffer {
    pub states: Vec<Vec<f64>>,
    pub actions: Vec<u32>,
    pub rewards: Vec<f64>,
    pub dones: Vec<bool>,
}

/// Curriculum learning controller
#[derive(Debug)]
pub struct CurriculumController {
    pub stages: Vec<CurriculumStage>,
    pub current_stage: usize,
    pub stage_progress: f64,
}

/// Environment wrapper for preprocessing
#[derive(Debug, Clone)]
pub struct EnvironmentWrapper {
    pub env_name: String,
    pub preprocessing_steps: Vec<String>,
    pub frame_skip: usize,
    pub action_repeat: usize,
}

// ============ BASIC ENVIRONMENT ============

/// Create environment from config
pub fn create_environment(config: EnvironmentConfig) -> Environment {
    Environment {
        config,
        state: vec![0.0; 4],
        episode_step: 0,
        episode_reward: 0.0,
    }
}

/// Initialize environment
pub fn reset_environment(env: &mut Environment) -> Observation {
    env.state = vec![0.0; env.config.observation_shape.iter().product()];
    env.episode_step = 0;
    env.episode_reward = 0.0;
    
    Observation {
        state: env.state.clone(),
        info: HashMap::new(),
    }
}

/// Step environment
pub fn step_environment(env: &mut Environment, action: EnvAction) -> StepResult {
    env.episode_step += 1;
    
    // Simulate simple dynamics
    let reward = simulate_step(&env.state, action.0 as f64);
    env.episode_reward += reward;
    
    // Update state
    env.state[0] += (action.0 as f64 - 0.5) * 0.1;
    
    let done = env.episode_step >= env.config.max_episode_steps;
    
    StepResult {
        observation: Observation {
            state: env.state.clone(),
            info: HashMap::new(),
        },
        reward,
        done,
        truncated: false,
        info: HashMap::new(),
    }
}

/// Simple environment simulation
fn simulate_step(state: &[f64], action: f64) -> f64 {
    if state.len() > 0 && state[0].abs() < 2.4 && state[2].abs() < 0.209 {
        1.0
    } else {
        0.0
    }
}

// ============ GAME ENVIRONMENTS ============

/// Create Cartpole environment
pub fn create_cartpole() -> GameEnvironment {
    GameEnvironment {
        name: "CartPole-v1".to_string(),
        state: vec![0.0, 0.0, 0.0, 0.0],
        state_bounds: (
            vec![-2.4, f64::NEG_INFINITY, -0.209, f64::NEG_INFINITY],
            vec![2.4, f64::INFINITY, 0.209, f64::INFINITY],
        ),
        action_space_size: 2,
        episode_steps: 0,
        max_steps: 500,
    }
}

/// Create Mountain Car environment
pub fn create_mountain_car() -> GameEnvironment {
    GameEnvironment {
        name: "MountainCar-v0".to_string(),
        state: vec![-0.5, 0.0],
        state_bounds: (
            vec![-1.2, -0.07],
            vec![0.6, 0.07],
        ),
        action_space_size: 3,
        episode_steps: 0,
        max_steps: 200,
    }
}

/// Create Acrobot environment
pub fn create_acrobot() -> GameEnvironment {
    GameEnvironment {
        name: "Acrobot-v1".to_string(),
        state: vec![1.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        state_bounds: (
            vec![-1.0; 6],
            vec![1.0; 6],
        ),
        action_space_size: 3,
        episode_steps: 0,
        max_steps: 500,
    }
}

/// Reset game environment
pub fn game_reset(env: &mut GameEnvironment) -> Vec<f64> {
    env.state = generate_random_state(&env.state);
    env.episode_steps = 0;
    env.state.clone()
}

/// Step game environment
pub fn game_step(env: &mut GameEnvironment, action: u32) -> (Vec<f64>, f64, bool) {
    env.episode_steps += 1;
    
    let reward = calculate_reward(&env.name, &env.state, action);
    update_game_state(env, action);
    
    let done = env.episode_steps >= env.max_steps ||
               is_terminal_state(&env.name, &env.state);
    
    (env.state.clone(), reward, done)
}

/// Calculate game-specific reward
fn calculate_reward(game_name: &str, state: &[f64], _action: u32) -> f64 {
    match game_name {
        "CartPole-v1" => {
            if state[0].abs() < 2.4 && state[2].abs() < 0.209 {
                1.0
            } else {
                0.0
            }
        }
        "MountainCar-v0" => {
            if state[0] > 0.5 {
                0.0
            } else {
                -1.0
            }
        }
        _ => 0.0,
    }
}

/// Update game state based on action
fn update_game_state(env: &mut GameEnvironment, action: u32) {
    match env.name.as_str() {
        "CartPole-v1" => {
            let force = if action == 0 { -10.0 } else { 10.0 };
            env.state[0] += env.state[1] * 0.02;
            env.state[1] += (force + 9.8 * env.state[2].sin()) / 1.0 * 0.02;
        }
        "MountainCar-v0" => {
            let accel = action as f64 - 1.0;
            env.state[1] += accel * 0.001 - 0.0025 * env.state[0].cos();
            env.state[1] = env.state[1].max(-0.07).min(0.07);
            env.state[0] += env.state[1];
            env.state[0] = env.state[0].max(-1.2).min(0.6);
        }
        _ => {}
    }
}

/// Check terminal state
fn is_terminal_state(game_name: &str, state: &[f64]) -> bool {
    match game_name {
        "CartPole-v1" => state[0].abs() > 2.4 || state[2].abs() > 0.209,
        "MountainCar-v0" => state[0] > 0.5,
        _ => false,
    }
}

// ============ MULTI-AGENT ENVIRONMENT ============

/// Create multi-agent environment
pub fn create_multi_agent_env(num_agents: usize, obs_size: usize) -> MultiAgentEnvironment {
    let agent_states = vec![vec![0.0; obs_size]; num_agents];
    let observations = agent_states.iter()
        .map(|state| Observation {
            state: state.clone(),
            info: HashMap::new(),
        })
        .collect();
    
    MultiAgentEnvironment {
        num_agents,
        agent_states,
        shared_reward: 0.0,
        agent_rewards: vec![0.0; num_agents],
        dones: vec![false; num_agents],
        observations,
    }
}

/// Multi-agent step
pub fn multi_agent_step(
    env: &mut MultiAgentEnvironment,
    actions: &[u32],
) -> Vec<(Observation, f64, bool)> {
    let mut results = Vec::new();
    
    for (i, &action) in actions.iter().enumerate() {
        if i < env.agent_states.len() {
            // Update agent state
            env.agent_states[i][0] += (action as f64 - 0.5) * 0.1;
            
            // Calculate reward
            let reward = if env.agent_states[i][0].abs() < 1.0 { 1.0 } else { -1.0 };
            env.agent_rewards[i] = reward;
            
            results.push((
                Observation {
                    state: env.agent_states[i].clone(),
                    info: HashMap::new(),
                },
                reward,
                false,
            ));
        }
    }
    
    results
}

/// Multi-agent cooperative reward
pub fn assign_cooperative_reward(rewards: &mut [f64]) {
    let avg = rewards.iter().sum::<f64>() / rewards.len() as f64;
    for r in rewards.iter_mut() {
        *r = avg;
    }
}

// ============ CURRICULUM LEARNING ============

/// Create curriculum controller
pub fn create_curriculum(num_stages: usize) -> CurriculumController {
    let stages: Vec<CurriculumStage> = (0..num_stages)
        .map(|i| CurriculumStage {
            stage_id: i,
            difficulty: (i as f64 + 1.0) / num_stages as f64,
            reward_threshold: 100.0 * (i as f64 + 1.0),
            max_episodes: 1000,
            episode_count: 0,
        })
        .collect();
    
    CurriculumController {
        stages,
        current_stage: 0,
        stage_progress: 0.0,
    }
}

/// Get current difficulty
pub fn get_current_difficulty(controller: &CurriculumController) -> f64 {
    if controller.current_stage < controller.stages.len() {
        controller.stages[controller.current_stage].difficulty
    } else {
        1.0
    }
}

/// Update curriculum progress
pub fn update_curriculum_progress(
    controller: &mut CurriculumController,
    episode_reward: f64,
) {
    if controller.current_stage < controller.stages.len() {
        let stage = &mut controller.stages[controller.current_stage];
        stage.episode_count += 1;
        
        if episode_reward >= stage.reward_threshold {
            controller.current_stage += 1;
            controller.stage_progress = 0.0;
        }
    }
}

// ============ ENVIRONMENT WRAPPERS ============

/// Create environment wrapper
pub fn create_wrapper(env_name: &str) -> EnvironmentWrapper {
    EnvironmentWrapper {
        env_name: env_name.to_string(),
        preprocessing_steps: vec!["normalize".to_string(), "grayscale".to_string()],
        frame_skip: 4,
        action_repeat: 1,
    }
}

/// Apply frame skipping
pub fn apply_frame_skip(
    env: &mut GameEnvironment,
    action: u32,
    skip: usize,
) -> (Vec<f64>, f64, bool) {
    let mut total_reward = 0.0;
    let mut done = false;
    
    for _ in 0..skip {
        let (state, reward, step_done) = game_step(env, action);
        total_reward += reward;
        done = step_done;
        if done {
            break;
        }
    }
    
    (env.state.clone(), total_reward, done)
}

/// Normalize observation
pub fn normalize_observation(obs: &[f64]) -> Vec<f64> {
    let max_val = obs.iter().cloned().fold(f64::NEG_INFINITY, f64::max).abs();
    if max_val > 0.0 {
        obs.iter().map(|x| x / max_val).collect()
    } else {
        obs.to_vec()
    }
}

// ============ EPISODE RECORDING & STATS ============

/// Create episode buffer
pub fn create_episode_buffer() -> EpisodeBuffer {
    EpisodeBuffer {
        states: Vec::new(),
        actions: Vec::new(),
        rewards: Vec::new(),
        dones: Vec::new(),
    }
}

/// Record episode step
pub fn record_step(
    buffer: &mut EpisodeBuffer,
    state: &[f64],
    action: u32,
    reward: f64,
    done: bool,
) {
    buffer.states.push(state.to_vec());
    buffer.actions.push(action);
    buffer.rewards.push(reward);
    buffer.dones.push(done);
}

/// Calculate episode statistics
pub fn calculate_episode_stats(episode: usize, buffer: &EpisodeBuffer) -> EpisodeStats {
    let total_reward: f64 = buffer.rewards.iter().sum();
    let max_reward = buffer.rewards.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_reward = buffer.rewards.iter().cloned().fold(f64::INFINITY, f64::min);
    
    EpisodeStats {
        episode_num: episode,
        total_reward,
        steps: buffer.states.len() as u32,
        max_reward,
        min_reward,
        win: total_reward > 100.0,
    }
}

// ============ UTILITY FUNCTIONS ============

/// Generate random state
fn generate_random_state(template: &[f64]) -> Vec<f64> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as f64;
    
    template.iter()
        .enumerate()
        .map(|(i, _)| ((nanos * (i + 1) as f64) % 2.0 - 1.0) * 0.1)
        .collect()
}

/// Render environment state (placeholder)
pub fn render_environment(state: &[f64]) -> String {
    format!("State: {:?}", state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_environment() {
        let config = EnvironmentConfig {
            name: "test".to_string(),
            observation_shape: vec![4],
            action_space: 2,
            max_episode_steps: 500,
            render_mode: "rgb_array".to_string(),
        };
        let env = create_environment(config);
        assert_eq!(env.episode_step, 0);
    }

    #[test]
    fn test_reset_environment() {
        let config = EnvironmentConfig {
            name: "test".to_string(),
            observation_shape: vec![4],
            action_space: 2,
            max_episode_steps: 500,
            render_mode: "rgb_array".to_string(),
        };
        let mut env = create_environment(config);
        let obs = reset_environment(&mut env);
        assert_eq!(obs.state.len(), 4);
    }

    #[test]
    fn test_create_cartpole() {
        let env = create_cartpole();
        assert_eq!(env.name, "CartPole-v1");
        assert_eq!(env.action_space_size, 2);
    }

    #[test]
    fn test_create_mountain_car() {
        let env = create_mountain_car();
        assert_eq!(env.name, "MountainCar-v0");
        assert_eq!(env.state.len(), 2);
    }

    #[test]
    fn test_create_multi_agent_env() {
        let env = create_multi_agent_env(3, 4);
        assert_eq!(env.num_agents, 3);
        assert_eq!(env.agent_states.len(), 3);
    }

    #[test]
    fn test_create_curriculum() {
        let controller = create_curriculum(5);
        assert_eq!(controller.stages.len(), 5);
        assert_eq!(controller.current_stage, 0);
    }

    #[test]
    fn test_get_current_difficulty() {
        let controller = create_curriculum(3);
        let diff = get_current_difficulty(&controller);
        assert!(diff > 0.0 && diff <= 1.0);
    }

    #[test]
    fn test_create_wrapper() {
        let wrapper = create_wrapper("CartPole-v1");
        assert_eq!(wrapper.env_name, "CartPole-v1");
        assert_eq!(wrapper.frame_skip, 4);
    }

    #[test]
    fn test_normalize_observation() {
        let obs = vec![0.5, -0.5, 1.0];
        let normalized = normalize_observation(&obs);
        assert_eq!(normalized.len(), 3);
    }

    #[test]
    fn test_create_episode_buffer() {
        let buffer = create_episode_buffer();
        assert_eq!(buffer.states.len(), 0);
    }

    #[test]
    fn test_calculate_episode_stats() {
        let mut buffer = create_episode_buffer();
        record_step(&mut buffer, &[0.0, 0.0], 1, 1.0, false);
        record_step(&mut buffer, &[0.1, 0.1], 0, 1.0, true);
        let stats = calculate_episode_stats(1, &buffer);
        assert_eq!(stats.total_reward, 2.0);
    }

    #[test]
    fn test_multi_agent_step() {
        let mut env = create_multi_agent_env(2, 4);
        let actions = vec![0, 1];
        let results = multi_agent_step(&mut env, &actions);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_assign_cooperative_reward() {
        let mut rewards = vec![1.0, 2.0, 3.0];
        assign_cooperative_reward(&mut rewards);
        assert!((rewards[0] - rewards[1]).abs() < 0.01);
    }
}
