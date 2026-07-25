//! MEGA TEST: Every common program request people ask for.
//! Goal: 100% — zero generic stubs, every request returns real code.

use killer_native::builtin::BuiltinFunctions;

fn ask(q: &str) -> String {
    BuiltinFunctions::kala_dispatch("code", q, "casual", "killer")
}

fn assert_real_code(q: &str) {
    let r = ask(q);
    assert!(
        !r.contains("Connect an LLM") && !r.contains("ready to write your function"),
        "GENERIC STUB for: \"{}\"\nGot: {}",
        q, &r[..r.len().min(200)]
    );
    assert!(r.contains("```"), "NO CODE BLOCK for: \"{}\"", q);
}

// ═══════════════════════════════════════════════════════════════════════
// BASIC PROGRAMS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn b01_hello_world_java() { assert_real_code("write java hello world"); }
#[test] fn b02_hello_world_python() { assert_real_code("write python hello world"); }
#[test] fn b03_hello_world_rust() { assert_real_code("write rust hello world"); }
#[test] fn b04_hello_world_cpp() { assert_real_code("write c++ hello world"); }
#[test] fn b05_hello_world_go() { assert_real_code("write golang hello world"); }
#[test] fn b06_hello_world_js() { assert_real_code("write javascript hello world"); }
#[test] fn b07_hello_world_csharp() { assert_real_code("write c# hello world"); }
#[test] fn b08_hello_world_kotlin() { assert_real_code("write kotlin hello world"); }
#[test] fn b09_hello_world_swift() { assert_real_code("write swift hello world"); }

// ═══════════════════════════════════════════════════════════════════════
// LOOPS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn l01_java_for_loop() { assert_real_code("write java for loop program"); }
#[test] fn l02_python_while_loop() { assert_real_code("write python while loop program"); }
#[test] fn l03_cpp_for_loop() { assert_real_code("write c++ for loop example"); }
#[test] fn l04_rust_loop() { assert_real_code("write rust loop example"); }
#[test] fn l05_js_for_loop() { assert_real_code("write javascript for loop"); }
#[test] fn l06_java_iterate() { assert_real_code("write java iterate over array"); }

// ═══════════════════════════════════════════════════════════════════════
// CONDITIONALS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn c01_java_if_else() { assert_real_code("write java if else program"); }
#[test] fn c02_python_if_else() { assert_real_code("write python if else example"); }
#[test] fn c03_java_switch_case() { assert_real_code("write java switch case example"); }

// ═══════════════════════════════════════════════════════════════════════
// DATA STRUCTURES
// ═══════════════════════════════════════════════════════════════════════
#[test] fn d01_java_array() { assert_real_code("write java array program"); }
#[test] fn d02_python_list() { assert_real_code("write python list example"); }
#[test] fn d03_java_arraylist() { assert_real_code("write java arraylist program"); }
#[test] fn d04_java_linked_list() { assert_real_code("write java linked list"); }
#[test] fn d05_java_stack() { assert_real_code("write java stack implementation"); }
#[test] fn d06_java_queue() { assert_real_code("write java queue program"); }
#[test] fn d07_java_hashmap() { assert_real_code("write java hashmap example"); }
#[test] fn d08_python_dictionary() { assert_real_code("write python dictionary program"); }
#[test] fn d09_java_binary_tree() { assert_real_code("write java binary tree"); }
#[test] fn d10_python_set() { assert_real_code("write python set program"); }

// ═══════════════════════════════════════════════════════════════════════
// ALGORITHMS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn a01_java_bubble_sort() { assert_real_code("write java bubble sort"); }
#[test] fn a02_python_quicksort() { assert_real_code("write python quicksort"); }
#[test] fn a03_java_binary_search() { assert_real_code("write java binary search"); }
#[test] fn a04_python_merge_sort() { assert_real_code("write python merge sort"); }
#[test] fn a05_java_fibonacci() { assert_real_code("write java fibonacci program"); }
#[test] fn a06_python_factorial() { assert_real_code("write python factorial program"); }
#[test] fn a07_java_palindrome() { assert_real_code("write java palindrome program"); }
#[test] fn a08_python_fizzbuzz() { assert_real_code("write python fizzbuzz"); }
#[test] fn a09_java_gcd_lcm() { assert_real_code("write java gcd lcm program"); }
#[test] fn a10_java_dijkstra() { assert_real_code("write java dijkstra shortest path"); }
#[test] fn a11_python_two_sum() { assert_real_code("write python two sum"); }
#[test] fn a12_java_selection_sort() { assert_real_code("write java selection sort"); }
#[test] fn a13_recursion_example() { assert_real_code("write java recursion example"); }

// ═══════════════════════════════════════════════════════════════════════
// OOP CONCEPTS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn o01_java_class() { assert_real_code("write java class program"); }
#[test] fn o02_java_inheritance() { assert_real_code("write java inheritance example"); }
#[test] fn o03_java_interface() { assert_real_code("write java interface example"); }
#[test] fn o04_java_abstract_class() { assert_real_code("write java abstract class"); }
#[test] fn o05_java_polymorphism() { assert_real_code("write java polymorphism example"); }
#[test] fn o06_java_encapsulation() { assert_real_code("write java encapsulation example"); }
#[test] fn o07_java_enum() { assert_real_code("write java enum example"); }
#[test] fn o08_java_constructor() { assert_real_code("write java constructor example"); }
#[test] fn o09_java_getter_setter() { assert_real_code("write java getter setter"); }
#[test] fn o10_java_method_overloading() { assert_real_code("write java method overloading"); }
#[test] fn o11_java_method_overriding() { assert_real_code("write java method overriding"); }
#[test] fn o12_java_generics() { assert_real_code("write java generics example"); }
#[test] fn o13_python_class() { assert_real_code("write python class program"); }
#[test] fn o14_rust_struct() { assert_real_code("write rust struct example"); }
#[test] fn o15_rust_trait() { assert_real_code("write rust trait example"); }
#[test] fn o16_rust_enum() { assert_real_code("write rust enum example"); }
#[test] fn o17_typescript_interface() { assert_real_code("write typescript interface example"); }

// ═══════════════════════════════════════════════════════════════════════
// GAMES
// ═══════════════════════════════════════════════════════════════════════
#[test] fn g01_tic_tac_toe() { assert_real_code("write python tic tac toe game"); }
#[test] fn g02_number_guessing() { assert_real_code("write java number guessing game"); }
#[test] fn g03_rock_paper_scissors() { assert_real_code("write python rock paper scissors"); }
#[test] fn g04_dice_game() { assert_real_code("write java dice game"); }
#[test] fn g05_quiz_game() { assert_real_code("write python quiz game"); }
#[test] fn g06_hangman() { assert_real_code("write python hangman game"); }
#[test] fn g07_snake_game() { assert_real_code("write python snake game"); }

// ═══════════════════════════════════════════════════════════════════════
// PATTERNS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn p01_star_pattern() { assert_real_code("write java star pattern program"); }
#[test] fn p02_pyramid() { assert_real_code("write python pyramid pattern"); }
#[test] fn p03_diamond() { assert_real_code("write java diamond pattern program"); }
#[test] fn p04_number_pattern() { assert_real_code("write java number pattern program"); }

// ═══════════════════════════════════════════════════════════════════════
// MATH PROGRAMS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn m01_calculator() { assert_real_code("write java calculator program"); }
#[test] fn m02_prime_number() { assert_real_code("write java prime number program"); }
#[test] fn m03_even_odd() { assert_real_code("write java even odd program"); }
#[test] fn m04_swap_numbers() { assert_real_code("write java swap two numbers"); }
#[test] fn m05_armstrong_number() { assert_real_code("write java armstrong number"); }
#[test] fn m06_area_calculator() { assert_real_code("write java area of circle program"); }
#[test] fn m07_temp_converter() { assert_real_code("write python temperature converter"); }
#[test] fn m08_leap_year() { assert_real_code("write java leap year program"); }
#[test] fn m09_power() { assert_real_code("write java power of number program"); }
#[test] fn m10_matrix_multiply() { assert_real_code("write java matrix multiplication"); }
#[test] fn m11_random_number() { assert_real_code("write java random number program"); }

// ═══════════════════════════════════════════════════════════════════════
// STRING PROGRAMS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn s01_string_reverse() { assert_real_code("write java string reverse program"); }
#[test] fn s02_palindrome_string() { assert_real_code("write python palindrome check"); }
#[test] fn s03_string_operations() { assert_real_code("write java string manipulation"); }
#[test] fn s04_anagram() { assert_real_code("write java anagram program"); }
#[test] fn s05_count_vowels() { assert_real_code("write java count vowels program"); }
#[test] fn s06_remove_duplicates() { assert_real_code("write java remove duplicates from string"); }

// ═══════════════════════════════════════════════════════════════════════
// FILE & IO
// ═══════════════════════════════════════════════════════════════════════
#[test] fn f01_file_read_write() { assert_real_code("write java file read write program"); }
#[test] fn f02_python_file() { assert_real_code("write python file handling program"); }
#[test] fn f03_csv_read() { assert_real_code("write python csv read write"); }

// ═══════════════════════════════════════════════════════════════════════
// WEB / API / NETWORKING
// ═══════════════════════════════════════════════════════════════════════
#[test] fn w01_api_request() { assert_real_code("write python api request program"); }
#[test] fn w02_java_http() { assert_real_code("write java http request"); }
#[test] fn w03_web_scraping() { assert_real_code("write python web scraping"); }
#[test] fn w04_rest_api() { assert_real_code("write python rest api"); }
#[test] fn w05_websocket() { assert_real_code("write python websocket server"); }

// ═══════════════════════════════════════════════════════════════════════
// CONCURRENCY
// ═══════════════════════════════════════════════════════════════════════
#[test] fn t01_java_threading() { assert_real_code("write java multithreading example"); }
#[test] fn t02_python_threading() { assert_real_code("write python threading program"); }
#[test] fn t03_rust_threads() { assert_real_code("write rust thread example"); }
#[test] fn t04_python_async() { assert_real_code("write python async program"); }

// ═══════════════════════════════════════════════════════════════════════
// ERROR HANDLING
// ═══════════════════════════════════════════════════════════════════════
#[test] fn e01_java_try_catch() { assert_real_code("write java try catch example"); }
#[test] fn e02_python_exception() { assert_real_code("write python exception handling"); }
#[test] fn e03_java_custom_exception() { assert_real_code("write java custom exception"); }

// ═══════════════════════════════════════════════════════════════════════
// DATABASE
// ═══════════════════════════════════════════════════════════════════════
#[test] fn db01_python_sqlite() { assert_real_code("write python sqlite database"); }
#[test] fn db02_python_crud() { assert_real_code("write python crud program"); }

// ═══════════════════════════════════════════════════════════════════════
// DESIGN PATTERNS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn dp01_singleton() { assert_real_code("write python singleton pattern"); }
#[test] fn dp02_observer() { assert_real_code("write python observer pattern"); }
#[test] fn dp03_decorator() { assert_real_code("write python decorator example"); }

// ═══════════════════════════════════════════════════════════════════════
// FUNCTIONAL
// ═══════════════════════════════════════════════════════════════════════
#[test] fn fn01_lambda() { assert_real_code("write python lambda example"); }
#[test] fn fn02_map_filter() { assert_real_code("write python map filter reduce"); }
#[test] fn fn03_list_comp() { assert_real_code("write python list comprehension"); }

// ═══════════════════════════════════════════════════════════════════════
// MISC COMMON REQUESTS
// ═══════════════════════════════════════════════════════════════════════
#[test] fn x01_json_parse() { assert_real_code("write python json parse program"); }
#[test] fn x02_regex() { assert_real_code("write python regex validation"); }
#[test] fn x03_login_system() { assert_real_code("write python login system"); }
#[test] fn x04_cli_tool() { assert_real_code("write python command line tool"); }
#[test] fn x05_todo_app() { assert_real_code("write python todo app"); }
#[test] fn x06_student_mgmt() { assert_real_code("write java student management program"); }
#[test] fn x07_bank_account() { assert_real_code("write java bank account program"); }
#[test] fn x08_date_time() { assert_real_code("write python date time program"); }
#[test] fn x09_type_casting() { assert_real_code("write java type casting example"); }
#[test] fn x10_input_output() { assert_real_code("write java basic input output"); }
#[test] fn x11_collections() { assert_real_code("write java collections example"); }
#[test] fn x12_sorting_list() { assert_real_code("write python sort a list program"); }

// ═══════════════════════════════════════════════════════════════════════
// DIFFERENT ASK STYLES
// ═══════════════════════════════════════════════════════════════════════
#[test] fn y01_give_me() { assert_real_code("give me a java for loop"); }
#[test] fn y02_show_me() { assert_real_code("show me python fibonacci code"); }
#[test] fn y03_how_to() { assert_real_code("how to write for loop in java"); }
#[test] fn y04_create() { assert_real_code("create a python calculator"); }
#[test] fn y05_build() { assert_real_code("build a java linked list"); }
#[test] fn y06_generate() { assert_real_code("generate python hello world"); }
#[test] fn y07_implement() { assert_real_code("implement binary search in java"); }
#[test] fn y08_can_you_write() { assert_real_code("can you write a python program for fibonacci"); }
#[test] fn y09_code_for() { assert_real_code("code for bubble sort in java"); }
#[test] fn y10_simple() { assert_real_code("simple java program"); }

// ═══════════════════════════════════════════════════════════════════════
// SQL
// ═══════════════════════════════════════════════════════════════════════
#[test] fn sql01_create_table() { assert_real_code("create table for student management"); }
#[test] fn sql02_sql_queries() { assert_real_code("write sql queries for employee database"); }

// TOTAL: ~130 tests
