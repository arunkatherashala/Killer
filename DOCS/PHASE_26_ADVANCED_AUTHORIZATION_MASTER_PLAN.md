# PHASE 26 - ADVANCED AUTHORIZATION & SECURITY
## Master Plan & Detailed Specifications

**Date:** March 18, 2026  
**Target Delivery:** Single session  
**Scope:** 5 modules, 250+ functions, 3,050+ lines, 50+ tests  
**Integration:** All modules registered in lib.rs

---

## Phase 26 Overview

Extend Killer's security with enterprise-grade authorization, supporting OAuth 2.0, OpenID Connect, role-based access control (RBAC), attribute-based access control (ABAC), distributed sessions, and advanced token management.

**Why Phase 26?**
- Phase 24-25 built complete web framework + real-time communication
- Authorization is critical for production services
- Complements existing JWT + basic auth from Phase 24
- Essential for enterprise deployments

---

## Module Specifications

### Phase 26.1: OAuth 2.0 & OpenID Connect (400-450 lines, 50 functions)

**Purpose:** Industry-standard authorization protocol with identity layer

**Categories:**

#### OAuth 2.0 Core (10 functions)
- `new_oauth_client()` - Create OAuth client config
- `generate_authorization_code()` - Auth code for user consent
- `exchange_code_for_token()` - Convert auth code → access token
- `refresh_access_token()` - Extend token lifetime
- `validate_token_signature()` - Verify JWT signature
- `decode_access_token()` - Extract token claims
- `revoke_token()` - Invalidate token
- `get_token_expiration()` - Time until token expires
- `create_token_pair()` - Access + refresh token pair
- `validate_client_credentials()` - Client ID/secret verification

#### OAuth 2.0 Flows (12 functions)
- `authorization_code_flow_step1()` - Redirect to auth endpoint
- `authorization_code_flow_step2()` - User login & consent
- `implicit_flow()` - Direct token (browser-based)
- `password_flow()` - Resource owner password credentials
- `client_credentials_flow()` - Service-to-service auth
- `device_flow_init()` - IoT device authorization
- `device_flow_poll()` - Poll for user approval
- `hybrid_flow()` - Combine auth code + implicit
- `get_authorization_endpoint_url()` - Build auth URL
- `get_token_endpoint_url()` - Build token URL
- `get_userinfo_endpoint_url()` - User profile endpoint
- `validate_redirect_uri()` - PKCE + open redirect prevention

#### OpenID Connect Identity (12 functions)
- `create_id_token()` - JWT with user claims
- `validate_id_token()` - Verify signature + nonce
- `get_userinfo()` - Fetch user profile
- `request_userinfo()` - Call userinfo endpoint
- `parse_id_token_header()` - Extract alg/typ/kid
- `parse_id_token_payload()` - Extract claims (sub, name, email, etc)
- `verify_nonce()` - Prevent replay attacks
- `validate_audience_claim()` - Verify aud matches client_id
- `validate_issuer_claim()` - Verify token issuer
- `validate_issued_at_claim()` - Verify iat not in future
- `get_userinfo_from_token()` - Extract user info from ID token
- `discover_openid_configuration()` - Fetch .well-known/openid-configuration

#### PKCE (Proof Key for Code Exchange) (8 functions)
- `generate_code_verifier()` - Create 43-128 char random string
- `generate_code_challenge()` - Hash verifier with S256
- `validate_code_challenge()` - Verify challenge matches verifier
- `create_pkce_parameter_string()` - Build code_challenge_method param
- `extract_pkce_parameters()` - Parse from request
- `is_pkce_required()` - Check if PKCE mandatory
- `validate_verifier_length()` - Ensure 43-128 chars
- `get_code_challenge_method()` - S256 or plain

#### Token Management (8 functions)
- `create_access_token_payload()` - Build token claims
- `create_refresh_token_payload()` - Build refresh token
- `set_token_expiration()` - Configure TTL
- `add_custom_claims()` - Add app-specific claims
- `get_all_token_claims()` - Extract all claims
- `build_token_response()` - RFC 6749 response format
- `validate_token_response()` - Check required fields
- `handle_token_error()` - Error responses (invalid_grant, etc)

---

### Phase 26.2: Role-Based Access Control (400-450 lines, 50 functions)

**Purpose:** Fine-grained permission management with hierarchical roles

**Categories:**

#### Role Definitions (10 functions)
- `create_role()` - Define new role
- `add_permission_to_role()` - Grant permission
- `remove_permission_from_role()` - Revoke permission
- `get_role_permissions()` - List all role permissions
- `delete_role()` - Remove role
- `get_role_by_name()` - Lookup role
- `get_all_roles()` - List all roles
- `update_role_description()` - Change role metadata
- `create_role_hierarchy()` - Define role inheritance
- `get_parent_roles()` - Get inherited roles

#### User-Role Assignment (10 functions)
- `assign_role_to_user()` - Grant role to user
- `remove_role_from_user()` - Revoke role from user
- `get_user_roles()` - List user's roles
- `get_user_all_roles()` - Include inherited roles
- `has_user_role()` - Check if user has role
- `assign_role_to_group()` - Grant role to group
- `get_group_roles()` - List group's roles
- `get_user_roles_from_groups()` - Roles via group membership
- `bulk_assign_roles()` - Batch role assignment
- `bulk_remove_roles()` - Batch role removal

#### Permission Checks (12 functions)
- `check_permission()` - User has permission?
- `check_all_permissions()` - User has ALL permissions?
- `check_any_permission()` - User has ANY permission?
- `check_resource_permission()` - Permission on specific resource
- `check_resource_role()` - Role on specific resource
- `require_permission()` - Enforce permission or error
- `require_role()` - Enforce role or error
- `require_all_roles()` - Enforce multiple roles
- `require_any_role()` - Enforce at least one role
- `get_missing_permissions()` - Which permissions missing?
- `get_permission_level()` - Numeric priority
- `can_delegate_permission()` - Transitive permission check

#### Role Hierarchy & Inheritance (12 functions)
- `create_hierarchical_roles()` - Define role tree
- `add_role_inheritance()` - Child inherits perms from parent
- `remove_role_inheritance()` - Break inheritance link
- `get_inherited_permissions()` - All permissions including inherited
- `get_direct_permissions()` - Only direct permissions
- `get_role_depth()` - Nesting level
- `flatten_role_hierarchy()` - Convert to flat list
- `validate_role_hierarchy()` - Check for cycles
- `get_conflicting_roles()` - Roles that exclude each other
- `get_complementary_roles()` - Roles that imply others
- `recompute_inherited_permissions()` - Cache invalidation
- `soft_delete_role()` - Archive role, keep history

#### Audit & Logging (6 functions)
- `log_role_assignment()` - Record who assigned role
- `log_permission_check()` - Record permission decision
- `get_role_assignment_history()` - Audit trail
- `get_permission_check_audit()` - Query audit log
- `export_rbac_audit()` - Generate compliance report
- `purge_old_audit_logs()` - Retention policy

---

### Phase 26.3: Attribute-Based Access Control (400-450 lines, 50 functions)

**Purpose:** Fine-grained policy language (attributes + conditions)

**Categories:**

#### Policy Definition (12 functions)
- `create_abac_policy()` - Define attribute policy
- `add_policy_effect()` - Allow/Deny
- `add_policy_principal()` - Subject: user/group/service
- `add_policy_action()` - Action: read/write/delete
- `add_policy_resource()` - Resource: document/file/API
- `add_policy_condition()` - Condition: time/IP/MFA
- `get_policy_by_id()` - Lookup policy
- `delete_policy()` - Remove policy
- `list_all_policies()` - Get all active policies
- `validate_policy_syntax()` - Ensure valid structure
- `compile_policy()` - Optimize for evaluation
- `create_policy_from_template()` - Use predefined template

#### Condition Evaluation (14 functions)
- `create_attribute_value()` - Define attribute (user.department=HR)
- `create_condition()` - Build condition expression
- `add_equality_condition()` - attribute = value
- `add_comparison_condition()` - attribute < value (numeric)
- `add_string_match_condition()` - Contains/starts-with/regex
- `add_time_condition()` - Time range (9am-5pm, weekdays)
- `add_ip_condition()` - IP range restriction
- `add_mfa_condition()` - MFA required/optional
- `add_location_condition()` - Geofence
- `add_custom_condition()` - User-defined lambda
- `evaluate_condition()` - Check if condition passes
- `evaluate_all_conditions()` - AND all conditions
- `evaluate_any_condition()` - OR any condition
- `short_circuit_evaluation()` - Optimize evaluation

#### Attribute Management (12 functions)
- `define_attribute()` - Create attribute schema
- `set_user_attribute()` - user.department = HR
- `get_user_attribute()` - Retrieve attribute value
- `set_resource_attribute()` - resource.owner = alice
- `get_resource_attribute()` - Fetch resource attribute
- `set_environment_attribute()` - env.time, env.ip
- `get_environment_attribute()` - Fetch env attribute
- `list_user_attributes()` - All attributes for user
- `list_resource_attributes()` - All attributes for resource
- `update_attribute_value()` - Modify attribute
- `delete_attribute()` - Remove attribute
- `bulk_set_attributes()` - Batch import

#### Policy Decision (12 functions)
- `evaluate_access_policy()` - Principal + Action + Resource?
- `get_evaluation_result()` - Allow/Deny/NotApplicable
- `get_evaluation_reason()` - Why decision taken
- `get_matching_policies()` - Which policies matched
- `get_effective_policy()` - Highest priority matching
- `deny_overrides_allow()` - Policy precedence
- `create_policy_precedence()` - Define conflict resolution
- `get_evaluation_duration()` - Perf metric (ms)
- `cache_policy_decision()` - Memoize result
- `invalidate_policy_cache()` - Clear cache
- `get_cache_hit_rate()` - Metrics
- `export_policy_decisions()` - Compliance audit

---

### Phase 26.4: Distributed Session Management (400-450 lines, 50 functions)

**Purpose:** Cross-service session state (Redis/MongoDB backed)

**Categories:**

#### Session Storage (12 functions)
- `create_distributed_session()` - New session in store
- `store_session_data()` - Save session to Redis/MongoDB
- `retrieve_session_data()` - Load session from store
- `update_session_data()` - Modify session
- `delete_session()` - Destroy session
- `get_session_expiration()` - TTL remaining
- `extend_session_lifetime()` - Refresh expiry
- `invalidate_all_user_sessions()` - Logout all devices
- `invalidate_session_by_id()` - Logout specific device
- `list_user_sessions()` - All active sessions for user
- `get_session_metadata()` - Timestamp, IP, user-agent
- `bulk_cleanup_expired_sessions()` - Background cleanup

#### Session Identity (10 functions)
- `generate_session_id()` - Random 32+ char token
- `validate_session_id()` - Check format + active
- `get_session_user_id()` - Extract user from session
- `get_session_device_id()` - Device fingerprint
- `get_session_ip_address()` - Source IP
- `get_session_user_agent()` - Browser/client info
- `verify_session_signature()` - HMAC validation
- `bind_session_to_device()` - Prevent session theft
- `bind_session_to_ip()` - IP lock (optional)
- `create_session_cookie()` - HttpOnly/Secure attributes

#### Multi-Device Sessions (12 functions)
- `add_device_session()` - Register new device/browser
- `get_active_devices()` - All user's logged-in devices
- `get_session_per_device()` - Session for specific device
- `update_device_last_activity()` - Touch timestamp
- `get_device_lastactivity()` - Last use time
- `revoke_device_session()` - Logout one device
- `get_device_creation_time()` - When device added
- `set_device_nickname()` - Label device (Chrome, iPhone)
- `get_device_nickname()` - Retrieve label
- `set_device_trust_level()` - Trusted/untrusted
- `get_devices_by_trust_level()` - Filter by trust
- `check_new_device_login()` - Alert on new device

#### Session Synchronization (12 functions)
- `broadcast_session_update()` - Redis pub/sub alert
- `listen_for_session_changes()` - Subscribe to updates
- `sync_across_services()` - Replicate session state
- `handle_session_conflict()` - Resolve race conditions
- `create_session_event_log()` - Audit trail
- `get_session_event_log()` - Query activity history
- `detect_suspicious_activity()` - Anomaly detection
- `flag_session_for_review()` - Security investigation
- `get_flagged_sessions()` - List suspicious
- `export_session_statistics()` - Usage metrics
- `get_concurrent_session_count()` - Active sessions
- `set_session_concurrent_limit()` - Max devices per user

#### Session Security (4 functions)
- `rotate_session_id()` - Generate new ID after login
- `check_session_binding_violation()` - Detect hijack attempt
- `log_security_event()` - Record suspicious access
- `timeout_idle_session()` - Enforce timeout policy

---

### Phase 26.5: Token Introspection & Revocation (350-400 lines, 40 functions)

**Purpose:** Runtime token validation, revocation, and status tracking

**Categories:**

#### Token Introspection (12 functions)
- `introspect_token()` - Validate + return claims
- `get_token_active_status()` - active/inactive
- `get_token_scope()` - Authorization scope
- `get_token_client_id()` - Which client issued
- `get_token_subject()` - User ID (sub claim)
- `get_token_issued_time()` - iat claim
- `get_token_expires_time()` - exp claim
- `get_token_username()` - username claim
- `get_token_audience()` - aud claim
- `get_token_issuer()` - iss claim
- `validate_token_use()` - id_token vs access_token
- `export_introspection_result()` - RFC 7662 format

#### Token Revocation (10 functions)
- `revoke_token_by_id()` - Invalidate token
- `revoke_all_user_tokens()` - Logout user
- `revoke_all_client_tokens()` - Invalidate app's tokens
- `add_to_revocation_list()` - Blacklist
- `check_revocation_list()` - Is token revoked?
- `get_revocation_reason()` - Why revoked
- `schedule_token_revocation()` - Future revocation
- `cancel_scheduled_revocation()` - Undo scheduled
- `export_revocation_list()` - Audit report
- `cleanup_revoked_tokens()` - Archive old

#### Token Status Tracking (10 functions)
- `create_token_status_entry()` - Track token lifetime
- `mark_token_as_issued()` - Log creation
- `mark_token_as_used()` - Record first use
- `mark_token_as_expired()` - Record expiration
- `mark_token_as_revoked()` - Record revocation
- `get_token_status_history()` - State transitions
- `get_token_lifecycle_duration()` - Time from issue to revocation
- `export_expired_tokens_report()` - Audit compliance
- `get_token_usage_statistics()` - How many used
- `forecast_token_expiration()` - Batch expiry alert

#### JTI (JWT ID) Tracking (8 functions)
- `generate_unique_jti()` - UUID for token
- `store_jti_record()` - Register JTI
- `check_jti_exists()` - Already issued?
- `mark_jti_revoked()` - Invalidate
- `get_jti_creation_time()` - When issued
- `get_jti_last_used()` - Last validation
- `cleanup_old_jtis()` - Retention policy
- `export_jti_statistics()` - Metrics

---

## Integration Points

### CrossModule Dependencies
- **OAuth + Sessions:** OAuth token stored in distributed session
- **OAuth + RBAC:** User roles loaded after OAuth validation
- **RBAC + ABAC:** ABAC policies reference RBAC roles
- **ABAC + Introspection:** Introspection validates policy conditions
- **Sessions + Revocation:** Revoked tokens invalidate sessions

### Existing Phase Integration
- **Phase 24 Auth:** Extends JWT + basic auth
- **Phase 24 Middleware:** Introspection middleware guards routes
- **Phase 25 GraphQL:** RBAC/ABAC guards resolved fields
- **Phase 25 WebSocket:** RBAC authentication on upgrade

---

## Implementation Patterns

### Type-Safe Design
```rust
pub enum PolicyEffect { Allow, Deny }
pub enum AccessDecision { Permit, Forbid, NotApplicable }
pub struct OAuthClient { client_id: String, client_secret: String, redirect_uris: Vec<String> }
pub struct RBACPolicy { role: String, permissions: Vec<String> }
pub struct ABACPolicy { subject: Attribute, action: String, resource: Attribute, conditions: Vec<Condition> }
```

### Error Handling
```rust
pub enum AuthError {
    InvalidToken,
    ExpiredToken,
    InsufficientPermissions,
    PolicyDenied,
    SessionNotFound,
    TokenRevoked,
}
```

### Testing Strategy
- Unit tests per function (50+ total)
- Integration tests (OAuth flow end-to-end)
- Security tests (token tamperering, replay attacks)
- Performance tests (policy evaluation at scale)

---

## Success Criteria

✅ 250+ functions across 5 modules  
✅ 3,050+ lines of production code  
✅ 50+ comprehensive unit tests  
✅ RFC 6749 (OAuth 2.0), RFC 7519 (JWT), RFC 7662 (Introspection) compliance  
✅ Zero unsafe code  
✅ Type-safe error handling  
✅ Full lib.rs integration  

---

## Delivery Timeline

**Estimated:** Single session (~45 minutes)

1. **Phase 26.1 OAuth 2.0 & OpenID Connect** (~20 min)
2. **Phase 26.2 RBAC** (~20 min)
3. **Phase 26.3 ABAC** (~20 min)
4. **Phase 26.4 Distributed Sessions** (~25 min)
5. **Phase 26.5 Token Introspection** (~15 min)
6. **Integration & Testing** (~10 min)

**Total:** ~120 minutes max, single-session delivery

---

## Resources

📚 **OAuth 2.0 Specification:** RFC 6749  
📚 **OpenID Connect:** https://openid.net/specs/openid-connect-core-1_0.html  
📚 **JWT:** RFC 7519  
📚 **Token Introspection:** RFC 7662  
📚 **RBAC Model:** Ferraiolo & Kuhn (1992)  
📚 **ABAC:** NIST SP 800-162  

---

## Next Phase (Phase 27 Preview)

After Phase 26 Authorization is complete, Phase 27 options:

1. **Message Queues** - RabbitMQ, Kafka, Redis integration
2. **Distributed Systems** - Service discovery, load balancing, circuit breakers
3. **Advanced Monitoring** - Distributed tracing, metrics, alerting
4. **Machine Learning** - ML model serving, inference pipelines

---

**Phase 26 Ready to Launch** 🚀
