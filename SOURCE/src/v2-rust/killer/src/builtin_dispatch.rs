//! Thin forwarder so `tool_calling` (and similar) can invoke [`BuiltinFunctions::call`]
//! without creating a `builtin` ↔ `tool_calling` dependency cycle.

use crate::builtin::BuiltinFunctions;
use crate::error::VmError;
use crate::value::Value;

pub fn call_builtin(name: &str, args: &[Value]) -> Result<Value, VmError> {
    BuiltinFunctions::call(name, args)
}
