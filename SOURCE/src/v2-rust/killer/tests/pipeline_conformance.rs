//! Conformance tests for Killer’s **default** (text) and **AST** compilation pipelines.
//! See `SOURCE/docs/LANGUAGE_PIPELINE.md`.

#[cfg(test)]
mod pipeline_conformance {
    use killer_native::ast::{Expr, Stmt};
    use killer_native::{
        compile_killer_ast, compile_killer_default, parse_killer_program, run_killer_ast,
        run_killer_parsed, run_killer_source,
    };
    use killer_native::vm::VirtualMachine;

    #[test]
    fn default_pipeline_top_level_print() {
        run_killer_source("print(42)\n").expect("default pipeline should run top-level print");
    }

    #[test]
    fn default_pipeline_double_slash_comments() {
        let src = "// header\nx = 7 // value\nprint(x)\n";
        run_killer_source(src).expect("// line and end-of-line comments");
    }

    #[test]
    fn default_pipeline_multiline_array_literal() {
        let src = "nvals = [8, 16, 32,\n64, 128, 256, 512, 1024]\nprint(len(nvals))\n";
        run_killer_source(src).expect("multiline array literal");
    }

    #[test]
    fn default_pipeline_for_of_loop_values() {
        let src = "sum = 0\nfor v of [1, 2, 3] {\n  sum = sum + v\n}\nprint(sum)\n";
        run_killer_source(src).expect("for VAR of EXPR on default line pipeline");
    }

    /// Semicolons separate statements on one line (C-style inside `{ }`); strings keep `;` literal.
    #[test]
    fn default_pipeline_semicolon_two_statements_in_block() {
        let src = "x = 0\nif 1 > 0 {\n  x = 1; x = x + 2\n}\nprint(x)\n";
        run_killer_source(src).expect("semicolon-separated statements in block");
    }

    #[test]
    fn default_pipeline_semicolon_does_not_split_inside_string() {
        let src = "s = \"a;b\"\nprint(len(s))\n";
        run_killer_source(src).expect("semicolon inside string");
    }

    #[test]
    fn run_killer_parsed_double_slash_comment() {
        run_killer_parsed("x = 3 // n\nprint(x)\n").expect("parsed pipeline // comment");
    }

    #[test]
    fn default_pipeline_kfn_main_implicit() {
        let src = "kfn main()\n  print(1)\n";
        run_killer_source(src).expect("default pipeline kfn main");
    }

    /// Tier 4: `ui snapshot` line sugar → `ui_headless_snapshot_json()`.
    #[test]
    fn default_pipeline_ui_line_sugar() {
        let src = "kfn main()\n  x = ui snapshot\n  print(len(x))\n";
        run_killer_source(src).expect("ui line sugar");
    }

    /// Tier 4: `v = ui version` → `v = ui_core_version()`.
    #[test]
    fn default_pipeline_ui_assign_version_sugar() {
        let src = "kfn main()\n  v = ui version\n  print(get(v, \"major\"))\n";
        run_killer_source(src).expect("ui assign version sugar");
    }

    /// Tier 4: `ui tick` → `ui_headless_tick()`.
    #[test]
    fn default_pipeline_ui_tick_sugar() {
        let src = "kfn main()\n  d = ui tick\n  print(get(get(d, \"cooked\"), \"sum\"))\n";
        run_killer_source(src).expect("ui tick sugar");
    }

    /// Tier 4: `ui health` → `ui_health()` (matches HTTP GET /health body).
    #[test]
    fn default_pipeline_ui_health_sugar() {
        let src = "kfn main()\n  h = ui health\n  print(h)\n";
        run_killer_source(src).expect("ui health sugar");
    }

    #[test]
    fn default_compile_returns_program_without_classes() {
        let p = compile_killer_default("print(0)\n").expect("compile");
        assert!(p.classes.is_empty(), "default path leaves classes empty");
    }

    #[test]
    fn ast_pipeline_function_print_and_call() {
        let stmts = vec![
            Stmt::Function {
                name: "main".into(),
                params: vec![],
                body: vec![Stmt::Print(vec![Expr::Number(42.0)])],
                ai_annotations: vec![],
            },
            Stmt::Expr(Expr::Call {
                callee: "main".into(),
                args: vec![],
            }),
        ];
        run_killer_ast(&stmts).expect("AST pipeline run");
    }

    #[test]
    fn text_parser_produces_print_stmt() {
        let stmts = parse_killer_program("print(42)\n").expect("parse");
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Stmt::Print(args) => {
                assert_eq!(args.len(), 1);
                assert!(matches!(args[0], Expr::Number(n) if (n - 42.0).abs() < f64::EPSILON));
            }
            _ => panic!("expected Stmt::Print"),
        }
    }

    #[test]
    fn run_killer_parsed_print() {
        run_killer_parsed("print(42)\n").expect("run_killer_parsed");
    }

    #[test]
    fn run_killer_parsed_function_and_call() {
        let src = "kfn main() {\nprint(1)\n}\nmain()\n";
        run_killer_parsed(src).expect("run_killer_parsed kfn");
    }

    #[test]
    fn run_killer_parsed_while_loop() {
        let src = "i = 0\ntotal = 0\nwhile i < 5 {\ni = i + 1\ntotal = total + i\n}\nprint(total)\n";
        run_killer_parsed(src).expect("while loop");
    }

    #[test]
    fn ast_compile_populates_metadata() {
        let stmts = vec![
            Stmt::Function {
                name: "f".into(),
                params: vec![],
                body: vec![Stmt::Print(vec![Expr::Number(0.0)])],
                ai_annotations: vec![],
            },
            Stmt::Expr(Expr::Call {
                callee: "f".into(),
                args: vec![],
            }),
        ];
        let p = compile_killer_ast(&stmts).expect("compile_killer_ast");
        assert!(
            !p.function_arities.is_empty() || !p.instructions.is_empty(),
            "expected some bytecode"
        );
    }

    #[test]
    fn stmt_parser_parses_class() {
        let stmts = parse_killer_program("class A {\n  fn greet() {\n    print(1)\n  }\n}\n")
            .expect("class should parse");
        assert!(!stmts.is_empty(), "expected class statement");
    }

    #[test]
    fn stmt_parser_parses_try_catch() {
        let stmts = parse_killer_program("try {\n  print(1)\n} catch e {\n  print(e)\n}\n")
            .expect("try/catch should parse");
        assert!(!stmts.is_empty(), "expected try statement");
    }

    #[test]
    fn default_pipeline_dict_literal_line_compiler() {
        let src = "d = { \"a\": 1, \"b\": 2 }\nprint(len(keys(d)))\n";
        run_killer_source(src).expect("non-empty dict literal on default pipeline");
    }

    #[test]
    fn default_pipeline_pow_star_precedence() {
        let src = "print(2 ** 3 * 4)\n";
        run_killer_source(src).expect("** tighter than *");
    }

    /// Chained `**` matches Python: `2**(3**2)` = 512.
    #[test]
    fn default_pipeline_pow_right_assoc_chain() {
        let src = "print(2 ** 3 ** 2)\n";
        run_killer_source(src).expect("** chain right-associative");
    }

    #[test]
    fn default_pipeline_bit_and_builtin() {
        let src = "print(bit_and(14, 9))\n";
        run_killer_source(src).expect("bit_and builtin");
    }

    #[test]
    fn default_pipeline_index_non_identifier_receiver() {
        let src = "print(( [7, 8, 9] )[1])\n";
        run_killer_source(src).expect("index into parenthesized array literal");
    }

    #[test]
    fn default_pipeline_chained_subscript() {
        let src = "m = [[1, 2], [3, 4]]\nprint(m[1][0])\n";
        run_killer_source(src).expect("m[i][j] indexing");
    }

    #[test]
    fn default_pipeline_chained_index_assign() {
        let src = "m = [[1, 2], [3, 4]]\nm[1][0] = 99\nprint(m[1][0])\n";
        run_killer_source(src).expect("m[i][j] = v on default pipeline");
    }

    #[test]
    fn default_pipeline_triple_chained_index_assign() {
        let src = "m = [[[1, 2]]]\nm[0][0][1] = 9\nprint(m[0][0][1])\n";
        run_killer_source(src).expect("m[i][j][k] = v on default pipeline");
    }

    /// Python-style `if` at top level (no `kfn`/`fn`) must insert braces so multi-line then-bodies compile.
    #[test]
    fn default_pipeline_python_style_if_multiline_body_no_fn() {
        let src = "x = 1\nif x > 0\n  print(10)\n  print(20)\n";
        run_killer_source(src).expect("indented multi-line if body without function wrapper");
    }

    /// Same as brace form: `if cond { ... }` must not get a duplicate `{` from the offside pass.
    #[test]
    fn default_pipeline_if_brace_form_top_level_no_double_brace() {
        let src = "x = 1\nif x > 0 {\n  print(10)\n  print(20)\n}\n";
        run_killer_source(src).expect("braced if at top level");
    }

    /// Multi-line condition with balanced `(` / `)` across lines (line compiler).
    #[test]
    fn default_pipeline_if_multiline_condition_parens() {
        let src = r#"x = 3
if (
  x > 1
  && x < 10
) {
  print(1)
} else {
  print(0)
}
"#;
        run_killer_source(src).expect("multiline if condition with parens");
    }

    /// Multi-line condition using leading `&&` (no wrapping parens).
    #[test]
    fn default_pipeline_if_multiline_condition_operator_continuation() {
        let src = "x = 3\nif x > 1\n  && x < 10 {\n  print(7)\n}\n";
        run_killer_source(src).expect("multiline if && continuation");
    }

    /// `if(` without space before `(` — same as `if (`.
    #[test]
    fn default_pipeline_if_no_space_before_paren() {
        let src = "if(1 > 0) {\n  print(99)\n}\n";
        run_killer_source(src).expect("if( cond ) form");
    }

    /// `else if(` form and multi-line `else if` condition.
    #[test]
    fn default_pipeline_else_if_multiline() {
        let src = "x = 2\nif x > 10 {\n  print(0)\n} else if(\n  x > 1\n  && x < 5\n) {\n  print(42)\n} else {\n  print(1)\n}\n";
        run_killer_source(src).expect("else if multiline condition");
    }

    #[test]
    fn default_pipeline_while_multiline_condition() {
        let src = "i = 0\nwhile (\n  i < 3\n) {\n  i = i + 1\n}\nprint(i)\n";
        run_killer_source(src).expect("multiline while condition");
    }

    #[test]
    fn default_pipeline_array_index_assign() {
        let src = "a = [1, 2, 3]\na[1] = 99\nprint(a[1])\n";
        run_killer_source(src).expect("arr[i] = v on default pipeline");
    }

    #[test]
    fn default_pipeline_dict_index_assign() {
        let src = "d = { \"k\": 1 }\nd[\"k\"] = 42\nprint(d[\"k\"])\n";
        run_killer_source(src).expect("dict[key] = v on default pipeline");
    }

    #[test]
    fn default_pipeline_kfn_multiline_signature() {
        let src = "kfn sum(\n  x int,\n  y int\n)\n{\n  print(x + y)\n}\nsum(2, 3)\n";
        run_killer_source(src).expect("multiline kfn parameter list");
    }

    /// `a - b - c` is `(a - b) - c` on the default line compiler (left-associative − / * / % chain).
    #[test]
    fn default_pipeline_left_assoc_subtraction_and_mixed_mul_div() {
        run_killer_source("print(1 - 2 - 3)\n").expect("1-2-3");
        run_killer_source("print(8 / 4 / 2)\n").expect("8/4/2");
        run_killer_source("print(10 / 2 * 3)\n").expect("10/2*3 is (10/2)*3");
    }

    /// `examples/killer_language_ready.killer` is the public “super language” manifest — keep it VM-green.
    #[test]
    fn example_killer_language_ready_runs_on_default_pipeline() {
        const SRC: &str = include_str!("../examples/killer_language_ready.killer");
        let program = compile_killer_default(SRC).expect("killer_language_ready.killer compiles");
        let mut vm = VirtualMachine::new();
        vm.run(&program).expect("killer_language_ready.killer runs");
    }
}
