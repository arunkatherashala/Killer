//! The built-in SQL engine: joins, table aliases, qualified columns, subqueries, HAVING, DISTINCT, BETWEEN,
//! CASE and a few scalar functions. Each script runs through the real `killer run` binary; `show` prints one
//! line per row with the cells joined by `|` (null prints as `null`).
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const PRELUDE: &str = r#"fn show(db, q, cols) {
  for r in sql_query(db, q) {
    parts = []
    for c in cols {
      parts = parts + [str(r[c])]
    }
    println(join(parts, "|"))
  }
}
db = sql_open(":memory:")
sql_exec(db, "CREATE TABLE dept (id INTEGER, name TEXT)")
sql_exec(db, "INSERT INTO dept VALUES (1, 'Eng'), (2, 'Ops'), (3, 'Sales'), (4, 'Empty')")
sql_exec(db, "CREATE TABLE emp (id INTEGER, name TEXT, dept_id INTEGER, salary REAL, boss_id INTEGER)")
sql_exec(db, "INSERT INTO emp VALUES (1,'Ann',1,100.5,NULL),(2,'Bo',1,80,1),(3,'Cy',2,70,1),(4,'Di',2,NULL,3),(5,'Ed',3,60,3),(6,'Flo',NULL,50,2),(7,'Gus',1,80,2)")
sql_exec(db, "CREATE TABLE proj (id INTEGER, title TEXT, dept_id INTEGER)")
sql_exec(db, "INSERT INTO proj VALUES (10,'Alpha',1),(11,'Beta',1),(12,'Gamma',2),(13,'Delta',9)")
sql_exec(db, "CREATE TABLE asg (emp_id INTEGER, proj_id INTEGER, hours INTEGER)")
sql_exec(db, "INSERT INTO asg VALUES (1,10,5),(1,11,3),(2,10,8),(3,12,4),(5,12,NULL),(7,10,2)")
"#;

fn run(body: &str) -> (bool, String, String) {
    let dir: PathBuf = std::env::temp_dir().join(format!("killer_sqljoin_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("t.killer");
    std::fs::write(&file, format!("{}{}", PRELUDE, body)).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).current_dir(&dir).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let s = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (out.status.success(), s(&out.stdout).trim_end_matches('\n').to_string(), s(&out.stderr))
}

/// One query: (sql, columns to print, expected lines joined by newline).
fn check(cases: &[(&str, &[&str], &str)]) {
    for (sql, cols, want) in cases {
        let list = cols.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ");
        let (ok, stdout, stderr) = run(&format!("show(db, \"{}\", [{}])\n", sql, list));
        assert!(ok, "query failed: {}\n{}", sql, stderr);
        assert_eq!(&stdout, want, "query: {}", sql);
    }
}

/// A query that must fail with a message containing `msg`.
fn check_err(cases: &[(&str, &str)]) {
    for (sql, msg) in cases {
        let (ok, stdout, stderr) = run(&format!("sql_query(db, \"{}\")\n", sql));
        assert!(!ok, "expected an error from: {}\n(stdout {:?})", sql, stdout);
        assert!(stderr.contains(msg), "query: {}\nexpected the error to mention {:?}, got: {}", sql, msg, stderr);
    }
}

#[test]
fn inner_and_left_joins() {
    check(&[
        ("SELECT e.name, d.name AS dept FROM emp e JOIN dept d ON e.dept_id = d.id ORDER BY e.id", &["name", "dept"], "Ann|Eng\nBo|Eng\nCy|Ops\nDi|Ops\nEd|Sales\nGus|Eng"),
        ("SELECT e.name, d.name AS dept FROM emp e LEFT JOIN dept d ON e.dept_id = d.id ORDER BY e.id", &["name", "dept"], "Ann|Eng\nBo|Eng\nCy|Ops\nDi|Ops\nEd|Sales\nFlo|null\nGus|Eng"),
        ("SELECT d.name AS dept, e.name AS emp FROM dept d LEFT OUTER JOIN emp e ON e.dept_id = d.id WHERE e.id IS NULL", &["dept", "emp"], "Empty|null"),
        ("SELECT e.name, d.name AS dept FROM emp e LEFT JOIN dept d ON e.dept_id = d.id AND d.name = 'Eng' ORDER BY e.id", &["name", "dept"], "Ann|Eng\nBo|Eng\nCy|null\nDi|null\nEd|null\nFlo|null\nGus|Eng"),
        ("SELECT a.name AS emp, b.name AS boss FROM emp a LEFT JOIN emp b ON a.boss_id = b.id ORDER BY a.id", &["emp", "boss"], "Ann|null\nBo|Ann\nCy|Ann\nDi|Cy\nEd|Cy\nFlo|Bo\nGus|Bo"),
        ("SELECT e.name, p.title, a.hours FROM emp e LEFT JOIN asg a ON a.emp_id = e.id LEFT JOIN proj p ON p.id = a.proj_id ORDER BY e.id, p.id", &["name", "title", "hours"], "Ann|Alpha|5\nAnn|Beta|3\nBo|Alpha|8\nCy|Gamma|4\nDi|null|null\nEd|Gamma|null\nFlo|null|null\nGus|Alpha|2"),
        ("SELECT e.name, p.title FROM emp e JOIN asg a ON a.emp_id = e.id JOIN proj p ON p.id = a.proj_id AND p.dept_id = e.dept_id ORDER BY e.id, p.id", &["name", "title"], "Ann|Alpha\nAnn|Beta\nBo|Alpha\nCy|Gamma\nGus|Alpha"),
    ]);
}

#[test]
fn cross_joins_and_comma_joins() {
    check(&[
        ("SELECT e.name AS n, d.name AS d FROM emp e CROSS JOIN dept d WHERE e.id <= 2 ORDER BY e.id, d.id", &["n", "d"], "Ann|Eng\nAnn|Ops\nAnn|Sales\nAnn|Empty\nBo|Eng\nBo|Ops\nBo|Sales\nBo|Empty"),
        ("SELECT e.name AS n, d.name AS d FROM emp e, dept d WHERE e.dept_id = d.id AND d.name = 'Ops' ORDER BY e.id", &["n", "d"], "Cy|Ops\nDi|Ops"),
    ]);
}

#[test]
fn star_with_qualifiers_and_duplicate_names() {
    check(&[
        ("SELECT e.* FROM emp e JOIN dept d ON e.dept_id = d.id WHERE d.name = 'Sales'", &["id", "name", "dept_id", "salary", "boss_id"], "5|Ed|3|60|3"),
        ("SELECT d.*, e.name AS emp FROM dept d JOIN emp e ON e.dept_id = d.id WHERE e.id = 1", &["id", "name", "emp"], "1|Eng|Ann"),
        // columns that share a name come out as alias.column so that neither is lost
        ("SELECT * FROM dept d JOIN proj p ON p.dept_id = d.id WHERE p.id = 12", &["d.id", "name", "p.id", "title", "dept_id"], "2|Ops|12|Gamma|2"),
    ]);
}

#[test]
fn group_by_having_over_joins() {
    check(&[
        ("SELECT d.name AS dept, COUNT(e.id) AS n FROM dept d LEFT JOIN emp e ON e.dept_id = d.id GROUP BY d.name ORDER BY d.name", &["dept", "n"], "Empty|0\nEng|3\nOps|2\nSales|1"),
        ("SELECT d.name AS dept, COUNT(*) AS n FROM dept d LEFT JOIN emp e ON e.dept_id = d.id GROUP BY d.name ORDER BY d.name", &["dept", "n"], "Empty|1\nEng|3\nOps|2\nSales|1"),
        ("SELECT d.name AS dept, SUM(e.salary) AS s, AVG(e.salary) AS a, MIN(e.salary) AS lo, MAX(e.salary) AS hi FROM dept d JOIN emp e ON e.dept_id = d.id GROUP BY d.name ORDER BY d.name", &["dept", "s", "a", "lo", "hi"], "Eng|260.5|86.83333333333333|80|100.5\nOps|70|70|70|70\nSales|60|60|60|60"),
        ("SELECT d.name AS dept, COUNT(*) AS n FROM dept d JOIN emp e ON e.dept_id = d.id GROUP BY d.name HAVING COUNT(*) > 1 ORDER BY dept", &["dept", "n"], "Eng|3\nOps|2"),
        ("SELECT dept_id, COUNT(*) AS n FROM emp GROUP BY dept_id HAVING n >= 2 ORDER BY n DESC, dept_id", &["dept_id", "n"], "1|3\n2|2"),
        ("SELECT dept_id, COUNT(*) AS n FROM emp GROUP BY dept_id ORDER BY COUNT(*) DESC, dept_id", &["dept_id", "n"], "1|3\n2|2\nnull|1\n3|1"),
        ("SELECT e.name, SUM(a.hours) AS hrs FROM emp e JOIN asg a ON a.emp_id = e.id GROUP BY e.name HAVING SUM(a.hours) > 4 ORDER BY hrs DESC, e.name", &["name", "hrs"], "Ann|8\nBo|8"),
        ("SELECT p.title, COUNT(a.emp_id) AS staff, SUM(a.hours) AS hrs FROM proj p LEFT JOIN asg a ON a.proj_id = p.id GROUP BY p.title ORDER BY p.title", &["title", "staff", "hrs"], "Alpha|3|15\nBeta|1|3\nDelta|0|null\nGamma|2|4"),
        ("SELECT COUNT(*) AS n, SUM(salary) AS s FROM emp WHERE id > 100", &["n", "s"], "0|null"),
    ]);
}

#[test]
fn subqueries() {
    check(&[
        ("SELECT name FROM emp WHERE dept_id IN (SELECT id FROM dept WHERE name = 'Eng' OR name = 'Ops') ORDER BY id", &["name"], "Ann\nBo\nCy\nDi\nGus"),
        ("SELECT name FROM emp WHERE dept_id NOT IN (SELECT id FROM dept WHERE name = 'Eng') ORDER BY id", &["name"], "Cy\nDi\nEd"),
        // a NULL in the NOT IN list makes the test unknown, so no row qualifies
        ("SELECT name FROM dept WHERE id NOT IN (SELECT dept_id FROM emp)", &["name"], ""),
        ("SELECT name FROM emp WHERE salary > (SELECT AVG(salary) FROM emp) ORDER BY id", &["name"], "Ann\nBo\nGus"),
        ("SELECT name, (SELECT COUNT(*) FROM asg WHERE asg.emp_id = emp.id) AS n FROM emp ORDER BY id", &["name", "n"], "Ann|2\nBo|1\nCy|1\nDi|0\nEd|1\nFlo|0\nGus|1"),
        ("SELECT d.name FROM dept d WHERE EXISTS (SELECT 1 FROM emp e WHERE e.dept_id = d.id) ORDER BY d.id", &["name"], "Eng\nOps\nSales"),
        ("SELECT d.name FROM dept d WHERE NOT EXISTS (SELECT 1 FROM emp e WHERE e.dept_id = d.id) ORDER BY d.id", &["name"], "Empty"),
        ("SELECT e.name FROM emp e WHERE EXISTS (SELECT 1 FROM asg a WHERE a.emp_id = e.id AND a.hours > 4) ORDER BY e.id", &["name"], "Ann\nBo"),
        ("SELECT name FROM emp e WHERE salary = (SELECT MAX(salary) FROM emp e2 WHERE e2.dept_id = e.dept_id) ORDER BY id", &["name"], "Ann\nCy\nEd"),
        ("SELECT name FROM emp WHERE id IN (SELECT emp_id FROM asg WHERE proj_id IN (SELECT id FROM proj WHERE dept_id = 1)) ORDER BY id", &["name"], "Ann\nBo\nGus"),
        ("SELECT (SELECT MAX(salary) FROM emp) AS top, (SELECT COUNT(*) FROM dept) AS nd", &["top", "nd"], "100.5|4"),
        ("SELECT t.dept, t.n FROM (SELECT dept_id AS dept, COUNT(*) AS n FROM emp GROUP BY dept_id) t WHERE t.n > 1 ORDER BY t.dept", &["dept", "n"], "1|3\n2|2"),
        ("SELECT d.name AS dept, t.total FROM dept d JOIN (SELECT dept_id, SUM(salary) AS total FROM emp GROUP BY dept_id) t ON t.dept_id = d.id ORDER BY d.id", &["dept", "total"], "Eng|260.5\nOps|70\nSales|60"),
        ("SELECT x.name FROM (SELECT * FROM emp WHERE salary >= 80) AS x ORDER BY x.id", &["name"], "Ann\nBo\nGus"),
    ]);
}

#[test]
fn distinct_between_like_case_and_functions() {
    check(&[
        ("SELECT DISTINCT dept_id FROM emp ORDER BY dept_id", &["dept_id"], "null\n1\n2\n3"),
        ("SELECT COUNT(DISTINCT dept_id) AS n FROM emp", &["n"], "3"),
        ("SELECT COUNT(DISTINCT salary) AS n, COUNT(salary) AS m, COUNT(*) AS k FROM emp", &["n", "m", "k"], "5|6|7"),
        ("SELECT name FROM emp WHERE salary BETWEEN 60 AND 80 ORDER BY id", &["name"], "Bo\nCy\nEd\nGus"),
        ("SELECT name FROM emp WHERE salary NOT BETWEEN 60 AND 80 ORDER BY id", &["name"], "Ann\nFlo"),
        ("SELECT name FROM emp WHERE name LIKE '%o%' ORDER BY id", &["name"], "Bo\nFlo"),
        ("SELECT name FROM emp ORDER BY dept_id DESC, salary ASC, id", &["name"], "Ed\nDi\nCy\nBo\nGus\nAnn\nFlo"),
        ("SELECT name FROM emp ORDER BY id LIMIT 3 OFFSET 2", &["name"], "Cy\nDi\nEd"),
        ("SELECT name, CASE WHEN salary >= 80 THEN 'high' WHEN salary IS NULL THEN 'none' ELSE 'low' END AS band FROM emp ORDER BY id", &["name", "band"], "Ann|high\nBo|high\nCy|low\nDi|none\nEd|low\nFlo|low\nGus|high"),
        ("SELECT name, COALESCE(salary, 0) AS s, IFNULL(dept_id, -1) AS d FROM emp WHERE id IN (4, 6) ORDER BY id", &["name", "s", "d"], "Di|0|2\nFlo|50|-1"),
        ("SELECT UPPER(name) AS u, LOWER(name) AS l, LENGTH(name) AS n, SUBSTR(name, 2) AS s2, SUBSTR(name, 1, 2) AS s3 FROM emp WHERE id = 6", &["u", "l", "n", "s2", "s3"], "FLO|flo|3|lo|Fl"),
        ("SELECT ABS(-3) AS a, ROUND(2.567, 2) AS r, NULLIF(1, 1) AS z", &["a", "r", "z"], "3|2.57|null"),
    ]);
}

#[test]
fn dml_with_subqueries_and_insert_select() {
    let (ok, out, err) = run(r#"println(sql_exec(db, "UPDATE emp SET salary = salary + 1 WHERE dept_id IN (SELECT id FROM dept WHERE name = 'Ops')"))
println(sql_exec(db, "DELETE FROM asg WHERE emp_id NOT IN (SELECT id FROM emp WHERE dept_id = 1)"))
sql_exec(db, "CREATE TABLE names (n TEXT)")
println(sql_exec(db, "INSERT INTO names SELECT name FROM emp WHERE dept_id = 1"))
println(sql_exec(db, "INSERT INTO names (n) VALUES ((SELECT MAX(name) FROM dept))"))
show(db, "SELECT n FROM names", ["n"])
show(db, "SELECT name, salary FROM emp WHERE id IN (3, 4)", ["name", "salary"])
show(db, "SELECT COUNT(*) AS n FROM asg", ["n"])
"#);
    assert!(ok, "{}", err);
    assert_eq!(out, "2\n2\n3\n1\nAnn\nBo\nGus\nSales\nCy|71\nDi|null\n4");
}

#[test]
fn clear_error_messages() {
    check_err(&[
        ("SELECT nope FROM emp", "no column named nope"),
        ("SELECT e.nope FROM emp e", "no column named nope in e"),
        ("SELECT x.name FROM emp e", "no table or alias named x"),
        ("SELECT name FROM emp e JOIN dept d ON e.dept_id = d.id", "ambiguous column name name"),
        ("SELECT * FROM nosuch", "no table named nosuch"),
        ("SELECT * FROM emp JOIN dept ON emp.nope = dept.id", "no column named nope in emp"),
        ("SELECT 1 FROM emp JOIN emp ON 1", "used twice in FROM"),
        ("SELECT name FROM emp WHERE id IN (SELECT id, name FROM dept)", "must return one column"),
        ("SELECT name FROM emp WHERE COUNT(*) > 1", "not allowed in WHERE"),
        ("SELECT * FROM emp RIGHT JOIN dept ON 1", "RIGHT JOIN is not supported"),
        ("SELECT * FROM emp LEFT JOIN dept", "needs an ON condition"),
        ("SELECT frob(1)", "unknown function frob()"),
        ("SELECT z.* FROM emp", "no table or alias named z"),
    ]);
}

#[test]
fn persistence_format_is_unchanged() {
    // a database file written by the earlier single-table engine still opens, and joins work on it
    let (ok, out, err) = run(r#"f = "killer_sql_joins_compat.db"
d = sql_open(f)
sql_exec(d, "CREATE TABLE a (k, v)")
sql_exec(d, "CREATE TABLE b (k, w)")
sql_exec(d, "INSERT INTO a VALUES (1, 'x'), (2, 'y')")
sql_exec(d, "INSERT INTO b VALUES (2, 'Y')")
sql_close(d)
d2 = sql_open(f)
show(d2, "SELECT a.k, a.v, b.w FROM a LEFT JOIN b ON a.k = b.k ORDER BY a.k", ["k", "v", "w"])
sql_close(d2)
println(readFile(f))
"#);
    assert!(ok, "{}", err);
    assert_eq!(out, "1|x|null\n2|y|Y\nKILLERSQL 1\nT\ta\tk\tv\nR\tn1.0\tsx\nR\tn2.0\tsy\nT\tb\tk\tw\nR\tn2.0\tsY");
}
