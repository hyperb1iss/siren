use crate::models::{Framework, Language};
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};

/// Detect frameworks in a directory based on languages and files
pub fn detect_frameworks(dir: &Path, languages: &[Language]) -> Vec<Framework> {
    let mut frameworks = Vec::new();

    // Only try to detect frameworks if we have language files
    if languages.is_empty() {
        return frameworks;
    }

    // Check for React
    if (languages.contains(&Language::JavaScript) || languages.contains(&Language::TypeScript))
        && is_react_project(dir)
    {
        frameworks.push(Framework::React);
        debug!("Detected React framework");
    }

    // Check for Vue
    if (languages.contains(&Language::JavaScript) || languages.contains(&Language::TypeScript))
        && is_vue_project(dir)
    {
        frameworks.push(Framework::Vue);
        debug!("Detected Vue framework");
    }

    // Check for Angular
    if languages.contains(&Language::TypeScript) && is_angular_project(dir) {
        frameworks.push(Framework::Angular);
        debug!("Detected Angular framework");
    }

    // Check for Django
    if languages.contains(&Language::Python) && is_django_project(dir) {
        frameworks.push(Framework::Django);
        debug!("Detected Django framework");
    }

    // Check for Flask
    if languages.contains(&Language::Python) && is_flask_project(dir) {
        frameworks.push(Framework::Flask);
        debug!("Detected Flask framework");
    }

    // Check for Rails
    if languages.contains(&Language::Ruby) && is_rails_project(dir) {
        frameworks.push(Framework::Rails);
        debug!("Detected Rails framework");
    }

    frameworks
}

/// Check if a directory contains a React project
fn is_react_project(dir: &Path) -> bool {
    // Check for package.json with React dependency
    let package_json = dir.join("package.json");
    if package_json.exists() {
        if let Ok(content) = fs::read_to_string(&package_json) {
            if content.contains(r#""react":"#) || content.contains(r#""react-dom":"#) {
                return true;
            }
        }
    }

    // Check for common React files
    if dir.join("src").join("App.jsx").exists()
        || dir.join("src").join("App.js").exists()
        || dir.join("src").join("App.tsx").exists()
    {
        return true;
    }

    // Check for common React config files
    if dir.join(".babelrc").exists() && file_contains(dir.join(".babelrc"), "@babel/preset-react") {
        return true;
    }

    false
}

/// Check if a directory contains a Vue project
fn is_vue_project(dir: &Path) -> bool {
    // Check for package.json with Vue dependency
    let package_json = dir.join("package.json");
    if package_json.exists() {
        if let Ok(content) = fs::read_to_string(&package_json) {
            if content.contains(r#""vue":"#) {
                return true;
            }
        }
    }

    // Check for Vue files
    if dir.join("src").join("App.vue").exists() {
        return true;
    }

    // Check for vue.config.js
    if dir.join("vue.config.js").exists() {
        return true;
    }

    false
}

/// Check if a directory contains an Angular project
fn is_angular_project(dir: &Path) -> bool {
    // Check for Angular CLI config
    if dir.join("angular.json").exists() || dir.join(".angular-cli.json").exists() {
        return true;
    }

    // Check for package.json with Angular dependency
    let package_json = dir.join("package.json");
    if package_json.exists() {
        if let Ok(content) = fs::read_to_string(&package_json) {
            if content.contains(r#""@angular/core":"#) {
                return true;
            }
        }
    }

    false
}

/// Check if a directory contains a Django project
fn is_django_project(dir: &Path) -> bool {
    // Check for Django settings
    if dir.join("settings.py").exists()
        || dir.join("django.py").exists()
        || file_exists_recursive(dir, "settings.py", 3)
    {
        return true;
    }

    // Check for Django structure
    if dir.join("manage.py").exists() && file_contains(dir.join("manage.py"), "django") {
        return true;
    }

    // Check for requirements.txt with Django
    let requirements = dir.join("requirements.txt");
    if requirements.exists() && file_contains(requirements, "django") {
        return true;
    }

    false
}

/// Check if a directory contains a Flask project
fn is_flask_project(dir: &Path) -> bool {
    // Check for Flask app
    if dir.join("app.py").exists() && file_contains(dir.join("app.py"), "Flask") {
        return true;
    }

    // Check for wsgi.py with Flask
    if dir.join("wsgi.py").exists() && file_contains(dir.join("wsgi.py"), "Flask") {
        return true;
    }

    // Check for requirements.txt with Flask
    let requirements = dir.join("requirements.txt");
    if requirements.exists() && file_contains(requirements, "flask") {
        return true;
    }

    false
}

/// Check if a directory contains a Rails project
fn is_rails_project(dir: &Path) -> bool {
    // Check for Rails structure
    if dir.join("Gemfile").exists() && file_contains(dir.join("Gemfile"), "rails") {
        return true;
    }

    // Check for Rails directories
    if dir.join("app").join("controllers").exists()
        && dir.join("app").join("models").exists()
        && dir.join("config").join("routes.rb").exists()
    {
        return true;
    }

    false
}

/// Check if a file contains a string
fn file_contains(path: PathBuf, needle: &str) -> bool {
    if let Ok(content) = fs::read_to_string(path) {
        content.to_lowercase().contains(&needle.to_lowercase())
    } else {
        false
    }
}

/// Check if a file exists recursively up to a certain depth
fn file_exists_recursive(dir: &Path, filename: &str, max_depth: usize) -> bool {
    if max_depth == 0 {
        return false;
    }

    let dir_entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    for entry in dir_entries.flatten() {
        let path = entry.path();
        if (path.is_file() && path.file_name().and_then(|f| f.to_str()) == Some(filename)) ||
           (path.is_dir() && file_exists_recursive(&path, filename, max_depth - 1)) {
            return true;
        }
    }

    false
}
