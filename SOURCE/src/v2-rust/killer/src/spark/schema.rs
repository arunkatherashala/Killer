/// Schema definition for Killer Spark DataFrames
/// 
/// Defines data types and schema structures

use std::fmt;

/// Data types supported in Killer Spark
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    // Numeric types
    Int32,
    Int64,
    Float32,
    Float64,
    
    // String and binary
    String,
    Binary,
    
    // Boolean
    Boolean,
    
    // Temporal
    Date,
    Timestamp,
    
    // Complex types
    Array(Box<DataType>),
    Map(Box<DataType>, Box<DataType>),
    Struct(Vec<Field>),
    
    // Null
    Null,
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::Int32 => write!(f, "int"),
            DataType::Int64 => write!(f, "long"),
            DataType::Float32 => write!(f, "float"),
            DataType::Float64 => write!(f, "double"),
            DataType::String => write!(f, "string"),
            DataType::Binary => write!(f, "binary"),
            DataType::Boolean => write!(f, "boolean"),
            DataType::Date => write!(f, "date"),
            DataType::Timestamp => write!(f, "timestamp"),
            DataType::Array(dt) => write!(f, "array({})", dt),
            DataType::Map(k, v) => write!(f, "map({},{})", k, v),
            DataType::Struct(fields) => {
                write!(f, "struct(")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}:{}", field.name, field.data_type)?;
                }
                write!(f, ")")
            }
            DataType::Null => write!(f, "null"),
        }
    }
}

/// Schema field definition
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

impl Field {
    /// Create a new field
    pub fn new(name: impl Into<String>, data_type: DataType, nullable: bool) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable,
        }
    }

    /// Create non-nullable field
    pub fn non_nullable(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: false,
        }
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}{}", 
            self.name, 
            self.data_type,
            if self.nullable { "" } else { " NOT NULL" }
        )
    }
}

/// DataFrame schema - collection of fields
#[derive(Debug, Clone)]
pub struct Schema {
    fields: Vec<Field>,
}

impl Schema {
    /// Create a new schema
    pub fn new(fields: Vec<Field>) -> Self {
        Self { fields }
    }

    /// Create schema from field names and types
    pub fn from_tuples(tuples: Vec<(&str, DataType)>) -> Self {
        let fields = tuples.into_iter()
            .map(|(name, dtype)| Field::new(name, dtype, true))
            .collect();
        Self { fields }
    }

    /// Get all fields
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// Get field by index
    pub fn field_at(&self, index: usize) -> Option<&Field> {
        self.fields.get(index)
    }

    /// Get field by name
    pub fn field_by_name(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get field index by name
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }

    /// Get number of fields
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Check if schema is empty
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Add a field to schema
    pub fn with_field(mut self, field: Field) -> Self {
        self.fields.push(field);
        self
    }

    /// Select specific fields
    pub fn select(&self, names: &[&str]) -> Result<Schema, String> {
        let mut selected = Vec::new();
        for name in names {
            if let Some(field) = self.field_by_name(name) {
                selected.push(field.clone());
            } else {
                return Err(format!("Field '{}' not found in schema", name));
            }
        }
        Ok(Schema::new(selected))
    }
}

impl fmt::Display for Schema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "root\n")?;
        for field in &self.fields {
            write!(f, " |-- {}\n", field)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_creation() {
        let field = Field::new("name", DataType::String, true);
        assert_eq!(field.name, "name");
        assert!(field.nullable);
    }

    #[test]
    fn test_field_non_nullable() {
        let field = Field::non_nullable("id", DataType::Int64);
        assert!(!field.nullable);
    }

    #[test]
    fn test_schema_creation() {
        let fields = vec![
            Field::new("name", DataType::String, false),
            Field::new("age", DataType::Int32, true),
        ];
        let schema = Schema::new(fields);
        assert_eq!(schema.len(), 2);
    }

    #[test]
    fn test_schema_from_tuples() {
        let schema = Schema::from_tuples(vec![
            ("name", DataType::String),
            ("age", DataType::Int32),
        ]);
        assert_eq!(schema.len(), 2);
    }

    #[test]
    fn test_field_by_name() {
        let fields = vec![
            Field::new("name", DataType::String, false),
            Field::new("age", DataType::Int32, true),
        ];
        let schema = Schema::new(fields);
        
        let field = schema.field_by_name("name");
        assert!(field.is_some());
        assert_eq!(field.unwrap().data_type, DataType::String);
    }

    #[test]
    fn test_select_fields() {
        let schema = Schema::from_tuples(vec![
            ("name", DataType::String),
            ("age", DataType::Int32),
            ("email", DataType::String),
        ]);
        
        let selected = schema.select(&["name", "email"]).unwrap();
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_datatype_display() {
        assert_eq!(DataType::Int32.to_string(), "int");
        assert_eq!(DataType::String.to_string(), "string");
        assert_eq!(DataType::Array(Box::new(DataType::Int32)).to_string(), "array(int)");
    }
}
