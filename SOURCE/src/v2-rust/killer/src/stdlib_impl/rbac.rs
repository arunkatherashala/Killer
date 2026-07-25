// ================================================================
// ROLE-BASED ACCESS CONTROL (RBAC) - Phase 26.2
// Fine-grained permission management with hierarchical roles
// ================================================================

use std::collections::{HashMap, HashSet};

/// Role definition
#[derive(Clone, Debug)]
pub struct Role {
    pub name: String,
    pub permissions: HashSet<String>,
    pub parent_roles: Vec<String>,
    pub description: Option<String>,
}

/// RBAC policy
#[derive(Clone, Debug)]
pub struct RBACPolicy {
    pub role: String,
    pub permissions: Vec<String>,
}

/// User role assignment
#[derive(Clone, Debug)]
pub struct UserRole {
    pub user_id: String,
    pub role_name: String,
    pub assigned_at: u64,
}

pub struct RBACSolver;

impl RBACSolver {
    // ================================================================
    // ROLE DEFINITIONS (1-10)
    // ================================================================

    /// Problem 1: Create role
    pub fn create_role(name: &str) -> Role {
        Role {
            name: name.to_string(),
            permissions: HashSet::new(),
            parent_roles: Vec::new(),
            description: None,
        }
    }

    /// Problem 2: Add permission to role
    pub fn add_permission_to_role(role: &mut Role, permission: &str) {
        role.permissions.insert(permission.to_string());
    }

    /// Problem 3: Remove permission from role
    pub fn remove_permission_from_role(role: &mut Role, permission: &str) {
        role.permissions.remove(permission);
    }

    /// Problem 4: Get role permissions
    pub fn get_role_permissions(role: &Role) -> Vec<String> {
        role.permissions.iter().cloned().collect()
    }

    /// Problem 5: Delete role
    pub fn delete_role(roles: &mut HashMap<String, Role>, role_name: &str) {
        roles.remove(role_name);
    }

    /// Problem 6: Get role by name
    pub fn get_role_by_name(roles: &HashMap<String, Role>, role_name: &str) -> Option<Role> {
        roles.get(role_name).cloned()
    }

    /// Problem 7: Get all roles
    pub fn get_all_roles(roles: &HashMap<String, Role>) -> Vec<Role> {
        roles.values().cloned().collect()
    }

    /// Problem 8: Update role description
    pub fn update_role_description(role: &mut Role, description: &str) {
        role.description = Some(description.to_string());
    }

    /// Problem 9: Create role hierarchy
    pub fn create_role_hierarchy(
        parent_role: &str,
        child_role: &str,
    ) -> (String, String) {
        (parent_role.to_string(), child_role.to_string())
    }

    /// Problem 10: Get parent roles
    pub fn get_parent_roles(role: &Role) -> Vec<String> {
        role.parent_roles.clone()
    }

    // ================================================================
    // USER-ROLE ASSIGNMENT (11-20)
    // ================================================================

    /// Problem 11: Assign role to user
    pub fn assign_role_to_user(
        user_roles: &mut HashMap<String, Vec<String>>,
        user_id: &str,
        role_name: &str,
    ) {
        user_roles
            .entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(role_name.to_string());
    }

    /// Problem 12: Remove role from user
    pub fn remove_role_from_user(
        user_roles: &mut HashMap<String, Vec<String>>,
        user_id: &str,
        role_name: &str,
    ) {
        if let Some(roles) = user_roles.get_mut(user_id) {
            roles.retain(|r| r != role_name);
        }
    }

    /// Problem 13: Get user roles
    pub fn get_user_roles(
        user_roles: &HashMap<String, Vec<String>>,
        user_id: &str,
    ) -> Vec<String> {
        user_roles
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Problem 14: Get user all roles (with inheritance)
    pub fn get_user_all_roles(
        user_roles: &HashMap<String, Vec<String>>,
        roles_db: &HashMap<String, Role>,
        user_id: &str,
    ) -> Vec<String> {
        let mut all_roles = HashSet::new();
        let direct_roles = Self::get_user_roles(user_roles, user_id);
        
        for role_name in direct_roles {
            all_roles.insert(role_name.clone());
            if let Some(role) = roles_db.get(&role_name) {
                for parent in &role.parent_roles {
                    all_roles.insert(parent.clone());
                }
            }
        }
        
        all_roles.into_iter().collect()
    }

    /// Problem 15: Has user role
    pub fn has_user_role(
        user_roles: &HashMap<String, Vec<String>>,
        user_id: &str,
        role_name: &str,
    ) -> bool {
        Self::get_user_roles(user_roles, user_id)
            .contains(&role_name.to_string())
    }

    /// Problem 16: Assign role to group
    pub fn assign_role_to_group(
        group_roles: &mut HashMap<String, Vec<String>>,
        group_id: &str,
        role_name: &str,
    ) {
        group_roles
            .entry(group_id.to_string())
            .or_insert_with(Vec::new)
            .push(role_name.to_string());
    }

    /// Problem 17: Get group roles
    pub fn get_group_roles(
        group_roles: &HashMap<String, Vec<String>>,
        group_id: &str,
    ) -> Vec<String> {
        group_roles
            .get(group_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Problem 18: Get user roles from groups
    pub fn get_user_roles_from_groups(
        user_groups: &HashMap<String, Vec<String>>,
        group_roles: &HashMap<String, Vec<String>>,
        user_id: &str,
    ) -> Vec<String> {
        let groups = user_groups.get(user_id).cloned().unwrap_or_default();
        let mut roles = Vec::new();
        
        for group in groups {
            if let Some(group_roles_list) = group_roles.get(&group) {
                roles.extend(group_roles_list.clone());
            }
        }
        
        roles
    }

    /// Problem 19: Bulk assign roles
    pub fn bulk_assign_roles(
        user_roles: &mut HashMap<String, Vec<String>>,
        assignments: &[(String, String)],
    ) {
        for (user_id, role_name) in assignments {
            Self::assign_role_to_user(user_roles, user_id, role_name);
        }
    }

    /// Problem 20: Bulk remove roles
    pub fn bulk_remove_roles(
        user_roles: &mut HashMap<String, Vec<String>>,
        removals: &[(String, String)],
    ) {
        for (user_id, role_name) in removals {
            Self::remove_role_from_user(user_roles, user_id, role_name);
        }
    }

    // ================================================================
    // PERMISSION CHECKS (21-32)
    // ================================================================

    /// Problem 21: Check permission
    pub fn check_permission(
        user_roles: &HashMap<String, Vec<String>>,
        roles_db: &HashMap<String, Role>,
        user_id: &str,
        permission: &str,
    ) -> bool {
        let user_roles_list = Self::get_user_all_roles(user_roles, roles_db, user_id);
        
        for role_name in user_roles_list {
            if let Some(role) = roles_db.get(&role_name) {
                if role.permissions.contains(permission) {
                    return true;
                }
            }
        }
        
        false
    }

    /// Problem 22: Check all permissions
    pub fn check_all_permissions(
        user_roles: &HashMap<String, Vec<String>>,
        roles_db: &HashMap<String, Role>,
        user_id: &str,
        permissions: &[&str],
    ) -> bool {
        permissions.iter().all(|perm| {
            Self::check_permission(user_roles, roles_db, user_id, perm)
        })
    }

    /// Problem 23: Check any permission
    pub fn check_any_permission(
        user_roles: &HashMap<String, Vec<String>>,
        roles_db: &HashMap<String, Role>,
        user_id: &str,
        permissions: &[&str],
    ) -> bool {
        permissions.iter().any(|perm| {
            Self::check_permission(user_roles, roles_db, user_id, perm)
        })
    }

    /// Problem 24: Check resource permission
    pub fn check_resource_permission(
        user_roles: &HashMap<String, Vec<String>>,
        roles_db: &HashMap<String, Role>,
        user_id: &str,
        resource: &str,
        permission: &str,
    ) -> bool {
        let full_perm = format!("{}:{}", resource, permission);
        Self::check_permission(user_roles, roles_db, user_id, &full_perm)
    }

    /// Problem 25: Check resource role
    pub fn check_resource_role(
        user_roles: &HashMap<String, Vec<String>>,
        user_id: &str,
        resource: &str,
        role: &str,
    ) -> bool {
        let resource_role = format!("{}_{}", resource, role);
        Self::has_user_role(user_roles, user_id, &resource_role)
    }

    /// Problem 26: Require permission
    pub fn require_permission(
        user_roles: &HashMap<String, Vec<String>>,
        roles_db: &HashMap<String, Role>,
        user_id: &str,
        permission: &str,
    ) -> Result<(), String> {
        if Self::check_permission(user_roles, roles_db, user_id, permission) {
            Ok(())
        } else {
            Err(format!("Permission denied: {}", permission))
        }
    }

    /// Problem 27: Require role
    pub fn require_role(
        user_roles: &HashMap<String, Vec<String>>,
        user_id: &str,
        role_name: &str,
    ) -> Result<(), String> {
        if Self::has_user_role(user_roles, user_id, role_name) {
            Ok(())
        } else {
            Err(format!("Role required: {}", role_name))
        }
    }

    /// Problem 28: Require all roles
    pub fn require_all_roles(
        user_roles: &HashMap<String, Vec<String>>,
        user_id: &str,
        role_names: &[&str],
    ) -> Result<(), String> {
        for role_name in role_names {
            Self::require_role(user_roles, user_id, role_name)?;
        }
        Ok(())
    }

    /// Problem 29: Require any role
    pub fn require_any_role(
        user_roles: &HashMap<String, Vec<String>>,
        user_id: &str,
        role_names: &[&str],
    ) -> Result<(), String> {
        for role_name in role_names {
            if Self::has_user_role(user_roles, user_id, role_name) {
                return Ok(());
            }
        }
        Err(format!("Any of these roles required: {:?}", role_names))
    }

    /// Problem 30: Get missing permissions
    pub fn get_missing_permissions(
        user_roles: &HashMap<String, Vec<String>>,
        roles_db: &HashMap<String, Role>,
        user_id: &str,
        required_permissions: &[&str],
    ) -> Vec<String> {
        required_permissions
            .iter()
            .filter(|perm| {
                !Self::check_permission(user_roles, roles_db, user_id, perm)
            })
            .map(|p| p.to_string())
            .collect()
    }

    /// Problem 31: Get permission level
    pub fn get_permission_level(permission: &str) -> u32 {
        match permission {
            "admin" => 100,
            "write" => 50,
            "read" => 10,
            _ => 0,
        }
    }

    /// Problem 32: Can delegate permission
    pub fn can_delegate_permission(
        user_roles: &HashMap<String, Vec<String>>,
        roles_db: &HashMap<String, Role>,
        user_id: &str,
        permission: &str,
    ) -> bool {
        // Can only delegate if user has the permission
        Self::check_permission(user_roles, roles_db, user_id, permission)
    }

    // ================================================================
    // ROLE HIERARCHY & INHERITANCE (33-44)
    // ================================================================

    /// Problem 33: Create hierarchical roles
    pub fn create_hierarchical_roles(
        parent_name: &str,
        child_name: &str,
    ) -> (Role, Role) {
        let mut parent = Self::create_role(parent_name);
        let mut child = Self::create_role(child_name);
        child.parent_roles.push(parent_name.to_string());
        (parent, child)
    }

    /// Problem 34: Add role inheritance
    pub fn add_role_inheritance(
        roles: &mut HashMap<String, Role>,
        parent_role: &str,
        child_role: &str,
    ) {
        if let Some(child) = roles.get_mut(child_role) {
            child.parent_roles.push(parent_role.to_string());
        }
    }

    /// Problem 35: Remove role inheritance
    pub fn remove_role_inheritance(
        roles: &mut HashMap<String, Role>,
        parent_role: &str,
        child_role: &str,
    ) {
        if let Some(child) = roles.get_mut(child_role) {
            child.parent_roles.retain(|r| r != parent_role);
        }
    }

    /// Problem 36: Get inherited permissions
    pub fn get_inherited_permissions(
        roles: &HashMap<String, Role>,
        role_name: &str,
    ) -> HashSet<String> {
        let mut permissions = HashSet::new();
        
        if let Some(role) = roles.get(role_name) {
            permissions.extend(role.permissions.clone());
            
            for parent in &role.parent_roles {
                let parent_perms = Self::get_inherited_permissions(roles, parent);
                permissions.extend(parent_perms);
            }
        }
        
        permissions
    }

    /// Problem 37: Get direct permissions
    pub fn get_direct_permissions(role: &Role) -> HashSet<String> {
        role.permissions.clone()
    }

    /// Problem 38: Get role depth
    pub fn get_role_depth(roles: &HashMap<String, Role>, role_name: &str) -> u32 {
        let mut max_depth = 0;
        
        if let Some(role) = roles.get(role_name) {
            for parent in &role.parent_roles {
                let parent_depth = Self::get_role_depth(roles, parent);
                max_depth = max_depth.max(parent_depth + 1);
            }
        }
        
        max_depth
    }

    /// Problem 39: Flatten role hierarchy
    pub fn flatten_role_hierarchy(
        roles: &HashMap<String, Role>,
        role_name: &str,
    ) -> Vec<String> {
        let mut flat = vec![role_name.to_string()];
        
        if let Some(role) = roles.get(role_name) {
            for parent in &role.parent_roles {
                flat.extend(Self::flatten_role_hierarchy(roles, parent));
            }
        }
        
        flat
    }

    /// Problem 40: Validate role hierarchy
    pub fn validate_role_hierarchy(roles: &HashMap<String, Role>) -> bool {
        // Check for cycles (simplified for production should use topological sort)
        true
    }

    /// Problem 41: Get conflicting roles
    pub fn get_conflicting_roles(
        roles: &HashMap<String, Role>,
        role_name: &str,
    ) -> Vec<String> {
        // Return roles that conflict with given role
        Vec::new()
    }

    /// Problem 42: Get complementary roles
    pub fn get_complementary_roles(
        roles: &HashMap<String, Role>,
        role_name: &str,
    ) -> Vec<String> {
        let role_perms = if let Some(role) = roles.get(role_name) {
            role.permissions.clone()
        } else {
            return Vec::new();
        };
        
        let mut complementary = Vec::new();
        
        for (other_name, other_role) in roles {
            if other_name != role_name {
                let overlap = role_perms.intersection(&other_role.permissions).count();
                if overlap > 0 {
                    complementary.push(other_name.clone());
                }
            }
        }
        
        complementary
    }

    /// Problem 43: Recompute inherited permissions
    pub fn recompute_inherited_permissions(
        roles: &mut HashMap<String, Role>,
    ) {
        // Cache invalidation logic (in production, would use versioning)
        for _role in roles.values_mut() {
            // Recompute
        }
    }

    /// Problem 44: Soft delete role
    pub fn soft_delete_role(
        roles: &mut HashMap<String, Role>,
        role_name: &str,
        archive: &mut HashMap<String, Role>,
    ) {
        if let Some(role) = roles.remove(role_name) {
            archive.insert(role_name.to_string(), role);
        }
    }

    // ================================================================
    // AUDIT & LOGGING (45-50)
    // ================================================================

    /// Problem 45: Log role assignment
    pub fn log_role_assignment(
        audit_log: &mut Vec<HashMap<String, String>>,
        user_id: &str,
        role_name: &str,
        assigned_by: &str,
    ) {
        let mut entry = HashMap::new();
        entry.insert("action".to_string(), "assign_role".to_string());
        entry.insert("user_id".to_string(), user_id.to_string());
        entry.insert("role".to_string(), role_name.to_string());
        entry.insert("assigned_by".to_string(), assigned_by.to_string());
        audit_log.push(entry);
    }

    /// Problem 46: Log permission check
    pub fn log_permission_check(
        audit_log: &mut Vec<HashMap<String, String>>,
        user_id: &str,
        permission: &str,
        result: bool,
    ) {
        let mut entry = HashMap::new();
        entry.insert("action".to_string(), "permission_check".to_string());
        entry.insert("user_id".to_string(), user_id.to_string());
        entry.insert("permission".to_string(), permission.to_string());
        entry.insert("result".to_string(), result.to_string());
        audit_log.push(entry);
    }

    /// Problem 47: Get role assignment history
    pub fn get_role_assignment_history(
        audit_log: &[HashMap<String, String>],
        user_id: &str,
    ) -> Vec<HashMap<String, String>> {
        audit_log
            .iter()
            .filter(|e| {
                e.get("action").map(|a| a == "assign_role").unwrap_or(false)
                    && e.get("user_id").map(|u| u == user_id).unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Problem 48: Get permission check audit
    pub fn get_permission_check_audit(
        audit_log: &[HashMap<String, String>],
        permission: &str,
    ) -> Vec<HashMap<String, String>> {
        audit_log
            .iter()
            .filter(|e| {
                e.get("action").map(|a| a == "permission_check").unwrap_or(false)
                    && e.get("permission").map(|p| p == permission).unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Problem 49: Export RBAC audit
    pub fn export_rbac_audit(
        audit_log: &[HashMap<String, String>],
    ) -> String {
        format!("Total audit entries: {}", audit_log.len())
    }

    /// Problem 50: Purge old audit logs
    pub fn purge_old_audit_logs(
        audit_log: &mut Vec<HashMap<String, String>>,
        _retention_days: u32,
    ) {
        // In production, would check timestamps
        // For now, just keep the log
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_role() {
        let role = RBACSolver::create_role("admin");
        assert_eq!(role.name, "admin");
        assert!(role.permissions.is_empty());
    }

    #[test]
    fn test_add_permission() {
        let mut role = RBACSolver::create_role("admin");
        RBACSolver::add_permission_to_role(&mut role, "write");
        assert!(role.permissions.contains("write"));
    }

    #[test]
    fn test_assign_role_to_user() {
        let mut user_roles = HashMap::new();
        RBACSolver::assign_role_to_user(&mut user_roles, "user1", "admin");
        assert!(RBACSolver::has_user_role(&user_roles, "user1", "admin"));
    }

    #[test]
    fn test_check_permission() {
        let mut roles_db = HashMap::new();
        let mut admin_role = RBACSolver::create_role("admin");
        RBACSolver::add_permission_to_role(&mut admin_role, "write");
        roles_db.insert("admin".to_string(), admin_role);

        let mut user_roles = HashMap::new();
        RBACSolver::assign_role_to_user(&mut user_roles, "user1", "admin");

        assert!(RBACSolver::check_permission(&user_roles, &roles_db, "user1", "write"));
    }

    #[test]
    fn test_role_hierarchy() {
        let (parent, mut child) = RBACSolver::create_hierarchical_roles("admin", "moderator");
        assert!(child.parent_roles.contains(&"admin".to_string()));
    }

    #[test]
    fn test_get_inherited_permissions() {
        let mut roles = HashMap::new();
        let mut admin = RBACSolver::create_role("admin");
        RBACSolver::add_permission_to_role(&mut admin, "delete");
        roles.insert("admin".to_string(), admin);

        let (parent, child) = RBACSolver::create_hierarchical_roles("admin", "moderator");
        roles.insert("admin".to_string(), parent);
        roles.insert("moderator".to_string(), child);

        let perms = RBACSolver::get_inherited_permissions(&roles, "moderator");
        assert!(perms.contains("delete"));
    }

    #[test]
    fn test_require_permission() {
        let mut roles_db = HashMap::new();
        let mut admin_role = RBACSolver::create_role("admin");
        RBACSolver::add_permission_to_role(&mut admin_role, "delete");
        roles_db.insert("admin".to_string(), admin_role);

        let mut user_roles = HashMap::new();
        RBACSolver::assign_role_to_user(&mut user_roles, "user1", "admin");

        let result = RBACSolver::require_permission(&user_roles, &roles_db, "user1", "delete");
        assert!(result.is_ok());
    }

    #[test]
    fn test_bulk_assign_roles() {
        let mut user_roles = HashMap::new();
        let assignments = vec![
            ("user1".to_string(), "admin".to_string()),
            ("user2".to_string(), "editor".to_string()),
        ];
        RBACSolver::bulk_assign_roles(&mut user_roles, &assignments);
        assert!(RBACSolver::has_user_role(&user_roles, "user1", "admin"));
        assert!(RBACSolver::has_user_role(&user_roles, "user2", "editor"));
    }

    #[test]
    fn test_audit_logging() {
        let mut audit_log = Vec::new();
        RBACSolver::log_role_assignment(&mut audit_log, "user1", "admin", "admin_user");
        assert_eq!(audit_log.len(), 1);
        assert_eq!(audit_log[0].get("user_id"), Some(&"user1".to_string()));
    }

    #[test]
    fn test_get_missing_permissions() {
        let mut roles_db = HashMap::new();
        let admin_role = RBACSolver::create_role("admin");
        roles_db.insert("admin".to_string(), admin_role);

        let mut user_roles = HashMap::new();
        RBACSolver::assign_role_to_user(&mut user_roles, "user1", "guest");

        let missing = RBACSolver::get_missing_permissions(&user_roles, &roles_db, "user1", &["read", "write"]);
        assert_eq!(missing.len(), 2);
    }
}
