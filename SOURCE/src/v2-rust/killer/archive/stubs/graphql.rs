//! GraphQL — Query language and execution engine

pub struct GraphQLSchema { pub types: Vec<String> }

impl GraphQLSchema {
    pub fn new() -> Self { Self { types: vec![] } }
}
