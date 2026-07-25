//! TEST: Kala project scaffolding — multi-file project generation.
//! Each test verifies that a project request returns multi-file output with correct structure.

use killer_native::builtin::BuiltinFunctions;

fn code(q: &str) -> String {
    BuiltinFunctions::kala_dispatch("code", q, "casual", "killer")
}

fn assert_project(q: &str, required_files: &[&str], required_content: &[&str]) {
    let r = code(q);
    assert!(r.len() > 200, "Too short for project: \"{}\" ({} chars)", q, r.len());
    for file in required_files {
        assert!(r.contains(file), "Missing file '{}' in project: \"{}\"\nGot (first 500):\n{}", file, q, &r[..r.len().min(500)]);
    }
    for content in required_content {
        assert!(r.contains(content), "Missing content '{}' in project: \"{}\"\n", content, q);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FRONTEND FRAMEWORKS
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn project_react_app() {
    assert_project("create react app project",
        &["package.json", "App.jsx", "index.jsx", "App.css", "Header.jsx", ".gitignore"],
        &["react", "react-dom", "react-router-dom"]);
}

#[test]
fn project_nextjs_app() {
    assert_project("create next.js app project",
        &["package.json", "layout.js", "page.js", "route.js", "globals.css"],
        &["next", "tailwind"]);
}

#[test]
fn project_vue_app() {
    assert_project("create vue project",
        &["package.json", "App.vue", "main.js", "router.js", "Home.vue", "NavBar.vue"],
        &["vue", "vite"]);
}

#[test]
fn project_angular_app() {
    assert_project("create angular app project",
        &["app.component.ts", "app.routes.ts", "home.component.ts"],
        &["@angular", "RouterOutlet"]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// BACKEND FRAMEWORKS
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn project_express_api() {
    assert_project("create express api project",
        &["package.json", "server.js", "items.js", "errorHandler.js", "Dockerfile", ".env.example"],
        &["express", "cors"]);
}

#[test]
fn project_django_app() {
    assert_project("create django project",
        &["requirements.txt", "manage.py", "settings.py", "models.py", "views.py", "urls.py", "admin.py", "Dockerfile"],
        &["django", "gunicorn"]);
}

#[test]
fn project_fastapi_app() {
    assert_project("create fastapi project",
        &["requirements.txt", "main.py", "models.py", "database.py", "Dockerfile"],
        &["fastapi", "uvicorn", "sqlalchemy"]);
}

#[test]
fn project_flask_app() {
    assert_project("create flask project",
        &["requirements.txt", "app.py", "base.html", "home.html", "style.css", "Dockerfile"],
        &["flask", "gunicorn"]);
}

#[test]
fn project_spring_boot() {
    assert_project("create spring boot java project",
        &["pom.xml", "Application.java", "ItemController.java", "Item.java", "ItemRepository.java", "application.properties", "Dockerfile"],
        &["spring-boot", "JpaRepository", "@RestController"]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// LANGUAGE-SPECIFIC PROJECTS
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn project_rust() {
    assert_project("create rust project",
        &["Cargo.toml", "main.rs", "lib.rs", "integration_test.rs", "Dockerfile"],
        &["serde", "fn main"]);
}

#[test]
fn project_rust_web() {
    assert_project("create rust web api project",
        &["Cargo.toml", "main.rs", "Dockerfile"],
        &["actix", "HttpServer"]);
}

#[test]
fn project_go() {
    assert_project("create golang project",
        &["go.mod", "main.go", "items.go", "Dockerfile"],
        &["net/http", "json"]);
}

#[test]
fn project_python_package() {
    assert_project("create python package project",
        &["pyproject.toml", "__init__.py", "core.py", "cli.py", "test_core.py"],
        &["setuptools", "def greet"]);
}

#[test]
fn project_node_cli() {
    assert_project("create node cli tool project",
        &["package.json", "cli.js", "greet.js", "calc.js", "utils.js"],
        &["#!/usr/bin/env node", "mycli"]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATIC & DEPLOYMENT
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn project_html_website() {
    assert_project("create html website project",
        &["index.html", "about.html", "style.css", "app.js"],
        &["<!DOCTYPE html>", "var(--primary)"]);
}

#[test]
fn project_docker_compose() {
    assert_project("create docker compose project",
        &["docker-compose.yml", "Dockerfile", "nginx.conf", "main.py", ".env.example"],
        &["services:", "postgres"]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// FULL STACK
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn project_react_express_fullstack() {
    assert_project("create full stack react express project",
        &["backend/server.js", "frontend/src/App.jsx", "docker-compose.yml"],
        &["proxy", "cors"]);
}

#[test]
fn project_django_react_fullstack() {
    assert_project("create django react full stack project",
        &["backend/requirements.txt", "backend/api/views.py", "frontend/src/App.jsx"],
        &["rest_framework", "ViewSet"]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// VARIANT QUERIES — natural language phrasing
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn variant_build_react_app()   { assert_project("build a react app", &["App.jsx"], &["react"]); }
#[test]
fn variant_make_express_app()  { assert_project("make an express app", &["server.js"], &["express"]); }
#[test]
fn variant_generate_django()   { assert_project("generate django application", &["manage.py"], &["django"]); }
#[test]
fn variant_setup_fastapi()     { assert_project("setup fastapi app", &["main.py"], &["fastapi"]); }
#[test]
fn variant_scaffold_flask()    { assert_project("scaffold a flask project", &["app.py"], &["flask"]); }
#[test]
fn variant_new_vue_app()       { assert_project("build vue.js web app", &["App.vue"], &["vue"]); }
#[test]
fn variant_create_go_webapp()  { assert_project("create go web app", &["main.go"], &["net/http"]); }

// ═══════════════════════════════════════════════════════════════════════════════
// EXISTING TESTS STILL WORK — code gen is not broken
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn single_file_still_works_java() {
    let r = code("write java for loop program");
    assert!(r.contains("```java"), "Single-file code gen broken: {}", &r[..r.len().min(200)]);
    assert!(!r.contains("package.json"), "Single-file should not have project structure");
}

#[test]
fn single_file_still_works_python() {
    let r = code("write python fibonacci");
    assert!(r.contains("```python") || r.contains("```killer"), "Single-file code gen broken: {}", &r[..r.len().min(200)]);
}
