//! TEST: Kala code generation across ALL supported languages.
//! Every language must: (1) return real code, (2) have correct language tag, (3) no generic stub.

use killer_native::builtin::BuiltinFunctions;

fn code(q: &str) -> String {
    BuiltinFunctions::kala_dispatch("code", q, "casual", "killer")
}

fn assert_lang(q: &str, expected_tag: &str) {
    let r = code(q);
    assert!(
        !r.contains("Connect an LLM") && !r.contains("ready to write your function"),
        "GENERIC STUB for: \"{}\"\nGot: {}", q, &r[..r.len().min(200)]
    );
    let tag = format!("```{}", expected_tag);
    assert!(
        r.contains(&tag),
        "WRONG LANG TAG for: \"{}\" — expected ```{} but got:\n{}", q, expected_tag, &r[..r.len().min(300)]
    );
    assert!(r.len() > 50, "TOO SHORT for: \"{}\" ({}chars)", q, r.len());
}

// ═══════════════════════════════════════════════════════════════════════════════
// JAVA (14 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn java_hello_world()    { assert_lang("write java hello world", "java"); }
#[test] fn java_for_loop()       { assert_lang("write java for loop program", "java"); }
#[test] fn java_if_else()        { assert_lang("write java if else program", "java"); }
#[test] fn java_array()          { assert_lang("write java array program", "java"); }
#[test] fn java_class()          { assert_lang("write java class program", "java"); }
#[test] fn java_interface()      { assert_lang("write java interface example", "java"); }
#[test] fn java_enum()           { assert_lang("write java enum example", "java"); }
#[test] fn java_fibonacci()      { assert_lang("write java fibonacci program", "java"); }
#[test] fn java_calculator()     { assert_lang("write java calculator program", "java"); }
#[test] fn java_star_pattern()   { assert_lang("write java star pattern program", "java"); }
#[test] fn java_try_catch()      { assert_lang("write java try catch example", "java"); }
#[test] fn java_thread()         { assert_lang("write java multithreading example", "java"); }
#[test] fn java_file_io()        { assert_lang("write java file read write program", "java"); }
#[test] fn java_generics()       { assert_lang("write java generics example", "java"); }

// ═══════════════════════════════════════════════════════════════════════════════
// PYTHON (14 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn python_hello_world()  { assert_lang("write python hello world", "python"); }
#[test] fn python_for_loop()     { assert_lang("write python for loop program", "python"); }
#[test] fn python_if_else()      { assert_lang("write python if else example", "python"); }
#[test] fn python_list()         { assert_lang("write python list example", "python"); }
#[test] fn python_class()        { assert_lang("write python class program", "python"); }
#[test] fn python_dictionary()   { assert_lang("write python dictionary program", "python"); }
#[test] fn python_fibonacci()    { assert_lang("write python fibonacci program", "python"); }
#[test] fn python_calculator()   { assert_lang("write python calculator program", "python"); }
#[test] fn python_regex()        { assert_lang("write python regex validation", "python"); }
#[test] fn python_exception()    { assert_lang("write python exception handling", "python"); }
#[test] fn python_threading()    { assert_lang("write python threading program", "python"); }
#[test] fn python_file_io()      { assert_lang("write python file handling program", "python"); }
#[test] fn python_decorator()    { assert_lang("write python decorator example", "python"); }
#[test] fn python_api()          { assert_lang("write python api request program", "python"); }

// ═══════════════════════════════════════════════════════════════════════════════
// RUST (10 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn rust_hello_world()    { assert_lang("write rust hello world", "rust"); }
#[test] fn rust_for_loop()       { assert_lang("write rust loop example", "rust"); }
#[test] fn rust_struct()         { assert_lang("write rust struct example", "rust"); }
#[test] fn rust_enum()           { assert_lang("write rust enum example", "rust"); }
#[test] fn rust_trait()          { assert_lang("write rust trait example", "rust"); }
#[test] fn rust_thread()         { assert_lang("write rust thread example", "rust"); }
#[test] fn rust_fibonacci()      { assert_lang("write rust fibonacci program", "rust"); }
#[test] fn rust_error_handling() { assert_lang("write rust error handling example", "rust"); }
#[test] fn rust_guessing_game()  { assert_lang("write rust guessing game", "rust"); }
#[test] fn rust_cli()            { assert_lang("write rust command line tool", "rust"); }

// ═══════════════════════════════════════════════════════════════════════════════
// C++ (10 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn cpp_hello_world()     { assert_lang("write c++ hello world", "cpp"); }
#[test] fn cpp_for_loop()        { assert_lang("write c++ for loop example", "cpp"); }
#[test] fn cpp_class()           { assert_lang("write c++ class program", "cpp"); }
#[test] fn cpp_array()           { assert_lang("write c++ array program", "cpp"); }
#[test] fn cpp_fibonacci()       { assert_lang("write c++ fibonacci program", "cpp"); }
#[test] fn cpp_linked_list()     { assert_lang("write c++ linked list", "cpp"); }
#[test] fn cpp_sort()            { assert_lang("write c++ bubble sort", "cpp"); }
#[test] fn cpp_matrix()          { assert_lang("write c++ matrix multiplication", "cpp"); }
#[test] fn cpp_thread()          { assert_lang("write c++ thread example", "cpp"); }
#[test] fn cpp_error_handling()  { assert_lang("write c++ try catch example", "cpp"); }

// ═══════════════════════════════════════════════════════════════════════════════
// C (8 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn c_hello_world()       { assert_lang("write c language hello world", "c"); }
#[test] fn c_for_loop()          { assert_lang("write c language for loop", "c"); }
#[test] fn c_struct()            { assert_lang("write c language struct example", "c"); }
#[test] fn c_array()             { assert_lang("write c language array program", "c"); }
#[test] fn c_fibonacci()         { assert_lang("write c language fibonacci", "c"); }
#[test] fn c_prime()             { assert_lang("write c language prime number", "c"); }
#[test] fn c_calculator()        { assert_lang("write c language calculator", "c"); }
#[test] fn c_file_io()           { assert_lang("write c language file read write", "c"); }

// ═══════════════════════════════════════════════════════════════════════════════
// JAVASCRIPT (10 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn js_hello_world()      { assert_lang("write javascript hello world", "javascript"); }
#[test] fn js_for_loop()         { assert_lang("write javascript for loop", "javascript"); }
#[test] fn js_class()            { assert_lang("write javascript class program", "javascript"); }
#[test] fn js_array()            { assert_lang("write javascript array program", "javascript"); }
#[test] fn js_fibonacci()        { assert_lang("write javascript fibonacci", "javascript"); }
#[test] fn js_api_fetch()        { assert_lang("write javascript fetch api", "javascript"); }
#[test] fn js_websocket()        { assert_lang("write javascript websocket server", "javascript"); }
#[test] fn js_map_filter()       { assert_lang("write javascript map filter reduce", "javascript"); }
#[test] fn js_json()             { assert_lang("write javascript json parse program", "javascript"); }
#[test] fn js_async()            { assert_lang("write javascript async program", "javascript"); }

// ═══════════════════════════════════════════════════════════════════════════════
// TYPESCRIPT (5 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn ts_hello_world()      { assert_lang("write typescript hello world", "typescript"); }
#[test] fn ts_interface()        { assert_lang("write typescript interface example", "typescript"); }
#[test] fn ts_class()            { assert_lang("write typescript class program", "typescript"); }
#[test] fn ts_for_loop()         { assert_lang("write typescript for loop", "typescript"); }
#[test] fn ts_fibonacci()        { assert_lang("write typescript fibonacci", "typescript"); }

// ═══════════════════════════════════════════════════════════════════════════════
// GO (8 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn go_hello_world()      { assert_lang("write golang hello world", "go"); }
#[test] fn go_for_loop()         { assert_lang("write golang for loop", "go"); }
#[test] fn go_struct()           { assert_lang("write golang struct example", "go"); }
#[test] fn go_fibonacci()        { assert_lang("write golang fibonacci", "go"); }
#[test] fn go_sort()             { assert_lang("write golang bubble sort", "go"); }
#[test] fn go_thread()           { assert_lang("write golang thread example", "go"); }
#[test] fn go_cli()              { assert_lang("write golang command line tool", "go"); }
#[test] fn go_error_handling()   { assert_lang("write golang error handling", "go"); }

// ═══════════════════════════════════════════════════════════════════════════════
// C# (8 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn csharp_hello_world()  { assert_lang("write c# hello world", "csharp"); }
#[test] fn csharp_for_loop()     { assert_lang("write c# for loop", "csharp"); }
#[test] fn csharp_class()        { assert_lang("write c# class program", "csharp"); }
#[test] fn csharp_fibonacci()    { assert_lang("write c# fibonacci", "csharp"); }
#[test] fn csharp_array()        { assert_lang("write c# array program", "csharp"); }
#[test] fn csharp_enum()         { assert_lang("write c# enum example", "csharp"); }
#[test] fn csharp_interface()    { assert_lang("write c# interface example", "csharp"); }
#[test] fn csharp_error()        { assert_lang("write c# try catch example", "csharp"); }

// ═══════════════════════════════════════════════════════════════════════════════
// KOTLIN (6 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn kotlin_hello_world()  { assert_lang("write kotlin hello world", "kotlin"); }
#[test] fn kotlin_for_loop()     { assert_lang("write kotlin for loop", "kotlin"); }
#[test] fn kotlin_class()        { assert_lang("write kotlin class program", "kotlin"); }
#[test] fn kotlin_fibonacci()    { assert_lang("write kotlin fibonacci", "kotlin"); }
#[test] fn kotlin_enum()         { assert_lang("write kotlin enum example", "kotlin"); }
#[test] fn kotlin_sort()         { assert_lang("write kotlin bubble sort", "kotlin"); }

// ═══════════════════════════════════════════════════════════════════════════════
// SWIFT (6 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn swift_hello_world()   { assert_lang("write swift hello world", "swift"); }
#[test] fn swift_for_loop()      { assert_lang("write swift for loop", "swift"); }
#[test] fn swift_class()         { assert_lang("write swift class program", "swift"); }
#[test] fn swift_fibonacci()     { assert_lang("write swift fibonacci", "swift"); }
#[test] fn swift_enum()          { assert_lang("write swift enum example", "swift"); }
#[test] fn swift_sort()          { assert_lang("write swift bubble sort", "swift"); }

// ═══════════════════════════════════════════════════════════════════════════════
// RUBY (6 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn ruby_hello_world()    { assert_lang("write ruby hello world", "ruby"); }
#[test] fn ruby_for_loop()       { assert_lang("write ruby for loop", "ruby"); }
#[test] fn ruby_class()          { assert_lang("write ruby class program", "ruby"); }
#[test] fn ruby_fibonacci()      { assert_lang("write ruby fibonacci", "ruby"); }
#[test] fn ruby_sort()           { assert_lang("write ruby bubble sort", "ruby"); }
#[test] fn ruby_array()          { assert_lang("write ruby array program", "ruby"); }

// ═══════════════════════════════════════════════════════════════════════════════
// PHP (6 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn php_hello_world()     { assert_lang("write php hello world", "php"); }
#[test] fn php_for_loop()        { assert_lang("write php for loop", "php"); }
#[test] fn php_class()           { assert_lang("write php class program", "php"); }
#[test] fn php_fibonacci()       { assert_lang("write php fibonacci", "php"); }
#[test] fn php_sort()            { assert_lang("write php bubble sort", "php"); }
#[test] fn php_array()           { assert_lang("write php array program", "php"); }

// ═══════════════════════════════════════════════════════════════════════════════
// SCALA (4 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn scala_hello_world()   { assert_lang("write scala hello world", "scala"); }
#[test] fn scala_for_loop()      { assert_lang("write scala for loop", "scala"); }
#[test] fn scala_class()         { assert_lang("write scala class program", "scala"); }
#[test] fn scala_fibonacci()     { assert_lang("write scala fibonacci", "scala"); }

// ═══════════════════════════════════════════════════════════════════════════════
// DART (4 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn dart_hello_world()    { assert_lang("write dart hello world", "dart"); }
#[test] fn dart_for_loop()       { assert_lang("write dart for loop", "dart"); }
#[test] fn dart_class()          { assert_lang("write dart class program", "dart"); }
#[test] fn dart_fibonacci()      { assert_lang("write dart fibonacci", "dart"); }

// ═══════════════════════════════════════════════════════════════════════════════
// BASH / SHELL (5 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn bash_hello_world()    { assert_lang("write bash hello world", "bash"); }
#[test] fn bash_for_loop()       { assert_lang("write bash for loop", "bash"); }
#[test] fn bash_fibonacci()      { assert_lang("write bash fibonacci", "bash"); }
#[test] fn bash_file_io()        { assert_lang("write bash file read write", "bash"); }
#[test] fn bash_calculator()     { assert_lang("write bash calculator", "bash"); }

// ═══════════════════════════════════════════════════════════════════════════════
// HTML (3 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn html_hello_world()    { assert_lang("write html hello world", "html"); }
#[test] fn html_page()           { assert_lang("write html example", "html"); }
#[test] fn html_form()           { assert_lang("write html form example", "html"); }

// ═══════════════════════════════════════════════════════════════════════════════
// SQL (3 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn sql_create_table()    { assert_lang("create table for student management", "sql"); }
#[test] fn sql_queries()         { assert_lang("write sql queries for employee database", "sql"); }
#[test] fn sql_joins()           { assert_lang("write sql join example", "sql"); }

// ═══════════════════════════════════════════════════════════════════════════════
// KILLER (our own language!) (5 topics)
// ═══════════════════════════════════════════════════════════════════════════════
#[test] fn killer_hello_world()  { assert_lang("write killer hello world", "killer"); }
#[test] fn killer_for_loop()     { assert_lang("write killer for loop", "killer"); }
#[test] fn killer_class()        { assert_lang("write killer class program", "killer"); }
#[test] fn killer_fibonacci()    { assert_lang("write killer fibonacci", "killer"); }
#[test] fn killer_sort()         { assert_lang("write killer bubble sort", "killer"); }

// GRAND TOTAL: ~155 tests across 17 languages
