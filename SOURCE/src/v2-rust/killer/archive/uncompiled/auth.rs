//! Authentication — User authentication and credential management

pub struct User { pub id: String, pub name: String }

pub struct AuthProvider;

impl AuthProvider {
    pub fn authenticate(_username: &str, _password: &str) -> Option<User> {
        None
    }
}
