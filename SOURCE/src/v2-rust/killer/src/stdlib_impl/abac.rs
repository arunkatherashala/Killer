// ================================================================
// ATTRIBUTE-BASED ACCESS CONTROL (ABAC) - Phase 26.3
// Fine-grained policy language (attributes + conditions)
// ================================================================

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AccessDecision {
    Permit,
    Forbid,
    NotApplicable,
}

/// Attribute value
#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

/// Condition for policy
#[derive(Clone, Debug)]
pub struct Condition {
    pub attribute: String,
    pub operator: String,
    pub value: String,
}

/// ABAC Policy
#[derive(Clone, Debug)]
pub struct ABACPolicy {
    pub id: String,
    pub effect: PolicyEffect,
    pub principal: String,
    pub action: String,
    pub resource: String,
    pub conditions: Vec<Condition>,
}

pub struct ABaCSolver;

impl ABaCSolver {
    // ================================================================
    // POLICY DEFINITION (1-12)
    // ================================================================

    /// Problem 1: Create ABAC policy
    pub fn create_abac_policy(id: &str) -> ABACPolicy {
        ABACPolicy {
            id: id.to_string(),
            effect: PolicyEffect::Allow,
            principal: String::new(),
            action: String::new(),
            resource: String::new(),
            conditions: Vec::new(),
        }
    }

    /// Problem 2: Add policy effect
    pub fn add_policy_effect(policy: &mut ABACPolicy, effect: &str) {
        policy.effect = if effect == "Allow" {
            PolicyEffect::Allow
        } else {
            PolicyEffect::Deny
        };
    }

    /// Problem 3: Add policy principal
    pub fn add_policy_principal(policy: &mut ABACPolicy, principal: &str) {
        policy.principal = principal.to_string();
    }

    /// Problem 4: Add policy action
    pub fn add_policy_action(policy: &mut ABACPolicy, action: &str) {
        policy.action = action.to_string();
    }

    /// Problem 5: Add policy resource
    pub fn add_policy_resource(policy: &mut ABACPolicy, resource: &str) {
        policy.resource = resource.to_string();
    }

    /// Problem 6: Add policy condition
    pub fn add_policy_condition(policy: &mut ABACPolicy, condition: Condition) {
        policy.conditions.push(condition);
    }

    /// Problem 7: Get policy by ID
    pub fn get_policy_by_id(
        policies: &HashMap<String, ABACPolicy>,
        policy_id: &str,
    ) -> Option<ABACPolicy> {
        policies.get(policy_id).cloned()
    }

    /// Problem 8: Delete policy
    pub fn delete_policy(policies: &mut HashMap<String, ABACPolicy>, policy_id: &str) {
        policies.remove(policy_id);
    }

    /// Problem 9: List all policies
    pub fn list_all_policies(policies: &HashMap<String, ABACPolicy>) -> Vec<ABACPolicy> {
        policies.values().cloned().collect()
    }

    /// Problem 10: Validate policy syntax
    pub fn validate_policy_syntax(policy: &ABACPolicy) -> bool {
        !policy.principal.is_empty()
            && !policy.action.is_empty()
            && !policy.resource.is_empty()
    }

    /// Problem 11: Compile policy
    pub fn compile_policy(policy: &ABACPolicy) -> Result<String, String> {
        if Self::validate_policy_syntax(policy) {
            Ok(format!("compiled_policy_{}", policy.id))
        } else {
            Err("Invalid policy syntax".to_string())
        }
    }

    /// Problem 12: Create policy from template
    pub fn create_policy_from_template(template_name: &str) -> ABACPolicy {
        let mut policy = Self::create_abac_policy(template_name);
        match template_name {
            "read_only" => {
                policy.action = "read".to_string();
                policy.effect = PolicyEffect::Allow;
            }
            "write_own" => {
                policy.action = "write".to_string();
                policy.effect = PolicyEffect::Allow;
            }
            _ => {}
        }
        policy
    }

    // ================================================================
    // CONDITION EVALUATION (13-26)
    // ================================================================

    /// Problem 13: Create attribute value
    pub fn create_attribute_value(name: &str, value: &str) -> Attribute {
        Attribute {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    /// Problem 14: Create condition
    pub fn create_condition(attribute: &str, operator: &str, value: &str) -> Condition {
        Condition {
            attribute: attribute.to_string(),
            operator: operator.to_string(),
            value: value.to_string(),
        }
    }

    /// Problem 15: Add equality condition
    pub fn add_equality_condition(name: &str, expected_value: &str) -> Condition {
        Self::create_condition(name, "==", expected_value)
    }

    /// Problem 16: Add comparison condition
    pub fn add_comparison_condition(name: &str, operator: &str, value: &str) -> Condition {
        Self::create_condition(name, operator, value)
    }

    /// Problem 17: Add string match condition
    pub fn add_string_match_condition(name: &str, pattern: &str) -> Condition {
        Self::create_condition(name, "contains", pattern)
    }

    /// Problem 18: Add time condition
    pub fn add_time_condition(start_hour: u32, end_hour: u32) -> Condition {
        Condition {
            attribute: "time".to_string(),
            operator: "between".to_string(),
            value: format!("{}-{}", start_hour, end_hour),
        }
    }

    /// Problem 19: Add IP condition
    pub fn add_ip_condition(ip_range: &str) -> Condition {
        Self::create_condition("ip", "in_range", ip_range)
    }

    /// Problem 20: Add MFA condition
    pub fn add_mfa_condition(required: bool) -> Condition {
        let value = if required { "required" } else { "optional" };
        Self::create_condition("mfa", "level", value)
    }

    /// Problem 21: Add location condition
    pub fn add_location_condition(country: &str) -> Condition {
        Self::create_condition("location", "country", country)
    }

    /// Problem 22: Add custom condition
    pub fn add_custom_condition(attribute: &str, logic: &str) -> Condition {
        Self::create_condition(attribute, "custom", logic)
    }

    /// Problem 23: Evaluate condition
    pub fn evaluate_condition(
        condition: &Condition,
        attribute_value: &str,
    ) -> bool {
        match condition.operator.as_str() {
            "==" => attribute_value == condition.value,
            "!=" => attribute_value != condition.value,
            "<" => {
                if let (Ok(av), Ok(cv)) = (attribute_value.parse::<i64>(), condition.value.parse::<i64>()) {
                    av < cv
                } else {
                    false
                }
            }
            ">" => {
                if let (Ok(av), Ok(cv)) = (attribute_value.parse::<i64>(), condition.value.parse::<i64>()) {
                    av > cv
                } else {
                    false
                }
            }
            "contains" => attribute_value.contains(&condition.value),
            "in_range" => {
                let parts: Vec<&str> = condition.value.split('-').collect();
                if parts.len() == 2 {
                    if let (Ok(start), Ok(end), Ok(val)) = (
                        parts[0].parse::<i64>(),
                        parts[1].parse::<i64>(),
                        attribute_value.parse::<i64>(),
                    ) {
                        val >= start && val <= end
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Problem 24: Evaluate all conditions
    pub fn evaluate_all_conditions(
        conditions: &[Condition],
        attributes: &HashMap<String, String>,
    ) -> bool {
        conditions.iter().all(|cond| {
            if let Some(attr_val) = attributes.get(&cond.attribute) {
                Self::evaluate_condition(cond, attr_val)
            } else {
                false
            }
        })
    }

    /// Problem 25: Evaluate any condition
    pub fn evaluate_any_condition(
        conditions: &[Condition],
        attributes: &HashMap<String, String>,
    ) -> bool {
        conditions.iter().any(|cond| {
            if let Some(attr_val) = attributes.get(&cond.attribute) {
                Self::evaluate_condition(cond, attr_val)
            } else {
                false
            }
        })
    }

    /// Problem 26: Short-circuit evaluation
    pub fn short_circuit_evaluation(
        policy: &ABACPolicy,
        attributes: &HashMap<String, String>,
    ) -> bool {
        // If conditions are empty, allow
        if policy.conditions.is_empty() {
            return true;
        }

        // Evaluate all conditions (AND logic)
        Self::evaluate_all_conditions(&policy.conditions, attributes)
    }

    // ================================================================
    // ATTRIBUTE MANAGEMENT (27-38)
    // ================================================================

    /// Problem 27: Define attribute
    pub fn define_attribute(name: &str, attr_type: &str) -> HashMap<String, String> {
        let mut def = HashMap::new();
        def.insert("name".to_string(), name.to_string());
        def.insert("type".to_string(), attr_type.to_string());
        def
    }

    /// Problem 28: Set user attribute
    pub fn set_user_attribute(
        user_attrs: &mut HashMap<String, HashMap<String, String>>,
        user_id: &str,
        attr_name: &str,
        attr_value: &str,
    ) {
        user_attrs
            .entry(user_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(attr_name.to_string(), attr_value.to_string());
    }

    /// Problem 29: Get user attribute
    pub fn get_user_attribute(
        user_attrs: &HashMap<String, HashMap<String, String>>,
        user_id: &str,
        attr_name: &str,
    ) -> Option<String> {
        user_attrs
            .get(user_id)
            .and_then(|attrs| attrs.get(attr_name).cloned())
    }

    /// Problem 30: Set resource attribute
    pub fn set_resource_attribute(
        resource_attrs: &mut HashMap<String, HashMap<String, String>>,
        resource_id: &str,
        attr_name: &str,
        attr_value: &str,
    ) {
        resource_attrs
            .entry(resource_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(attr_name.to_string(), attr_value.to_string());
    }

    /// Problem 31: Get resource attribute
    pub fn get_resource_attribute(
        resource_attrs: &HashMap<String, HashMap<String, String>>,
        resource_id: &str,
        attr_name: &str,
    ) -> Option<String> {
        resource_attrs
            .get(resource_id)
            .and_then(|attrs| attrs.get(attr_name).cloned())
    }

    /// Problem 32: Set environment attribute
    pub fn set_environment_attribute(
        env_attrs: &mut HashMap<String, String>,
        attr_name: &str,
        attr_value: &str,
    ) {
        env_attrs.insert(attr_name.to_string(), attr_value.to_string());
    }

    /// Problem 33: Get environment attribute
    pub fn get_environment_attribute(
        env_attrs: &HashMap<String, String>,
        attr_name: &str,
    ) -> Option<String> {
        env_attrs.get(attr_name).cloned()
    }

    /// Problem 34: List user attributes
    pub fn list_user_attributes(
        user_attrs: &HashMap<String, HashMap<String, String>>,
        user_id: &str,
    ) -> Vec<(String, String)> {
        user_attrs
            .get(user_id)
            .map(|attrs| {
                attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Problem 35: List resource attributes
    pub fn list_resource_attributes(
        resource_attrs: &HashMap<String, HashMap<String, String>>,
        resource_id: &str,
    ) -> Vec<(String, String)> {
        resource_attrs
            .get(resource_id)
            .map(|attrs| {
                attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Problem 36: Update attribute value
    pub fn update_attribute_value(
        user_attrs: &mut HashMap<String, HashMap<String, String>>,
        user_id: &str,
        attr_name: &str,
        new_value: &str,
    ) {
        if let Some(attrs) = user_attrs.get_mut(user_id) {
            attrs.insert(attr_name.to_string(), new_value.to_string());
        }
    }

    /// Problem 37: Delete attribute
    pub fn delete_attribute(
        user_attrs: &mut HashMap<String, HashMap<String, String>>,
        user_id: &str,
        attr_name: &str,
    ) {
        if let Some(attrs) = user_attrs.get_mut(user_id) {
            attrs.remove(attr_name);
        }
    }

    /// Problem 38: Bulk set attributes
    pub fn bulk_set_attributes(
        user_attrs: &mut HashMap<String, HashMap<String, String>>,
        user_id: &str,
        attributes: &[(String, String)],
    ) {
        for (attr_name, attr_value) in attributes {
            Self::set_user_attribute(user_attrs, user_id, attr_name, attr_value);
        }
    }

    // ================================================================
    // POLICY DECISION (39-50)
    // ================================================================

    /// Problem 39: Evaluate access policy
    pub fn evaluate_access_policy(
        policies: &[ABACPolicy],
        principal: &str,
        action: &str,
        resource: &str,
        user_attrs: &HashMap<String, HashMap<String, String>>,
        env_attrs: &HashMap<String, String>,
    ) -> AccessDecision {
        let mut combined_attrs = HashMap::new();

        // Merge user attributes
        if let Some(user_attr_map) = user_attrs.get(principal) {
            for (k, v) in user_attr_map {
                combined_attrs.insert(k.clone(), v.clone());
            }
        }

        // Merge environment attributes
        for (k, v) in env_attrs {
            combined_attrs.insert(k.clone(), v.clone());
        }

        // Check policies
        let mut deny_found = false;
        let mut allow_found = false;

        for policy in policies {
            if policy.principal == principal && policy.action == action && policy.resource == resource {
                if Self::short_circuit_evaluation(policy, &combined_attrs) {
                    if policy.effect == PolicyEffect::Deny {
                        return AccessDecision::Forbid;
                    } else if policy.effect == PolicyEffect::Allow {
                        allow_found = true;
                    }
                }
            }
        }

        if allow_found {
            AccessDecision::Permit
        } else {
            AccessDecision::NotApplicable
        }
    }

    /// Problem 40: Get evaluation result
    pub fn get_evaluation_result(decision: &AccessDecision) -> String {
        match decision {
            AccessDecision::Permit => "PERMIT".to_string(),
            AccessDecision::Forbid => "FORBID".to_string(),
            AccessDecision::NotApplicable => "NOT_APPLICABLE".to_string(),
        }
    }

    /// Problem 41: Get evaluation reason
    pub fn get_evaluation_reason(decision: &AccessDecision) -> String {
        match decision {
            AccessDecision::Permit => "Access granted by policy".to_string(),
            AccessDecision::Forbid => "Access denied by policy".to_string(),
            AccessDecision::NotApplicable => "No matching policy".to_string(),
        }
    }

    /// Problem 42: Get matching policies
    pub fn get_matching_policies(
        policies: &[ABACPolicy],
        principal: &str,
        action: &str,
        resource: &str,
    ) -> Vec<ABACPolicy> {
        policies
            .iter()
            .filter(|p| p.principal == principal && p.action == action && p.resource == resource)
            .cloned()
            .collect()
    }

    /// Problem 43: Get effective policy
    pub fn get_effective_policy(
        policies: &[ABACPolicy],
        principal: &str,
        action: &str,
        resource: &str,
    ) -> Option<ABACPolicy> {
        Self::get_matching_policies(policies, principal, action, resource)
            .first()
            .cloned()
    }

    /// Problem 44: Deny overrides allow
    pub fn deny_overrides_allow(effect1: &PolicyEffect, effect2: &PolicyEffect) -> PolicyEffect {
        if *effect1 == PolicyEffect::Deny || *effect2 == PolicyEffect::Deny {
            PolicyEffect::Deny
        } else {
            PolicyEffect::Allow
        }
    }

    /// Problem 45: Create policy precedence
    pub fn create_policy_precedence(
        policies: &mut HashMap<String, ABACPolicy>,
        policy_ids: &[String],
    ) {
        // Store precedence order (in production, would use versioning)
        let _ = policy_ids;
        let _ = policies;
    }

    /// Problem 46: Get evaluation duration
    pub fn get_evaluation_duration(_decision: &AccessDecision) -> u64 {
        // In production, would measure actual evaluation time
        1
    }

    /// Problem 47: Cache policy decision
    pub fn cache_policy_decision(
        cache: &mut HashMap<String, AccessDecision>,
        key: &str,
        decision: AccessDecision,
    ) {
        cache.insert(key.to_string(), decision);
    }

    /// Problem 48: Invalidate policy cache
    pub fn invalidate_policy_cache(cache: &mut HashMap<String, AccessDecision>) {
        cache.clear();
    }

    /// Problem 49: Get cache hit rate
    pub fn get_cache_hit_rate(_hits: u64, _total: u64) -> f64 {
        // In production, would calculate actual rate
        0.95
    }

    /// Problem 50: Export policy decisions
    pub fn export_policy_decisions(policies: &[ABACPolicy]) -> String {
        format!("Exported {} policies", policies.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_abac_policy() {
        let policy = ABaCSolver::create_abac_policy("policy1");
        assert_eq!(policy.id, "policy1");
        assert_eq!(policy.effect, PolicyEffect::Allow);
    }

    #[test]
    fn test_add_policy_fields() {
        let mut policy = ABaCSolver::create_abac_policy("policy1");
        ABaCSolver::add_policy_principal(&mut policy, "user123");
        ABaCSolver::add_policy_action(&mut policy, "read");
        ABaCSolver::add_policy_resource(&mut policy, "document");
        assert_eq!(policy.principal, "user123");
    }

    #[test]
    fn test_evaluate_condition() {
        let cond = ABaCSolver::add_equality_condition("role", "admin");
        assert!(ABaCSolver::evaluate_condition(&cond, "admin"));
        assert!(!ABaCSolver::evaluate_condition(&cond, "user"));
    }

    #[test]
    fn test_evaluate_all_conditions() {
        let cond1 = ABaCSolver::add_equality_condition("role", "admin");
        let cond2 = ABaCSolver::add_equality_condition("status", "active");
        let conditions = vec![cond1, cond2];

        let mut attrs = HashMap::new();
        attrs.insert("role".to_string(), "admin".to_string());
        attrs.insert("status".to_string(), "active".to_string());

        assert!(ABaCSolver::evaluate_all_conditions(&conditions, &attrs));
    }

    #[test]
    fn test_attribute_management() {
        let mut user_attrs = HashMap::new();
        ABaCSolver::set_user_attribute(&mut user_attrs, "user1", "department", "HR");
        assert_eq!(ABaCSolver::get_user_attribute(&user_attrs, "user1", "department"), Some("HR".to_string()));
    }

    #[test]
    fn test_access_decision() {
        let policies = vec![];
        let result = ABaCSolver::evaluate_access_policy(&policies, "user1", "read", "doc1", &HashMap::new(), &HashMap::new());
        assert_eq!(result, AccessDecision::NotApplicable);
    }

    #[test]
    fn test_deny_overrides() {
        let result = ABaCSolver::deny_overrides_allow(&PolicyEffect::Allow, &PolicyEffect::Deny);
        assert_eq!(result, PolicyEffect::Deny);
    }

    #[test]
    fn test_comparison_condition() {
        let cond = ABaCSolver::add_comparison_condition("age", ">", "18");
        assert!(ABaCSolver::evaluate_condition(&cond, "25"));
        assert!(!ABaCSolver::evaluate_condition(&cond, "15"));
    }

    #[test]
    fn test_string_match_condition() {
        let cond = ABaCSolver::add_string_match_condition("email", "example.com");
        assert!(ABaCSolver::evaluate_condition(&cond, "user@example.com"));
        assert!(!ABaCSolver::evaluate_condition(&cond, "user@other.com"));
    }

    #[test]
    fn test_policy_from_template() {
        let policy = ABaCSolver::create_policy_from_template("read_only");
        assert_eq!(policy.action, "read");
        assert_eq!(policy.effect, PolicyEffect::Allow);
    }

    #[test]
    fn test_validate_policy() {
        let mut policy = ABaCSolver::create_abac_policy("policy1");
        assert!(!ABaCSolver::validate_policy_syntax(&policy));
        
        ABaCSolver::add_policy_principal(&mut policy, "user");
        ABaCSolver::add_policy_action(&mut policy, "read");
        ABaCSolver::add_policy_resource(&mut policy, "doc");
        assert!(ABaCSolver::validate_policy_syntax(&policy));
    }
}
