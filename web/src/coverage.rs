use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

/// Generates HTML coverage report from coverage data
pub fn generate_coverage_html(coverage_map: &HashMap<String, HashSet<u32>>, kernel_src_dir: &str, work_dir: &str) {
    let html_dir = format!("{}/coverage_html", work_dir);
    
    // Create directory for HTML files if it doesn't exist
    if !Path::new(&html_dir).exists() {
        fs::create_dir_all(&html_dir).expect("Failed to create HTML directory");
    }
    
    // Create a file tree structure for the HTML sidebar
    let mut file_tree: HashMap<String, (usize, usize)> = HashMap::new(); // (covered_lines, total_lines)
    
    // Process each file in the coverage map
    for (file_path, covered_lines) in coverage_map {
        let full_path = format!("{}/{}", kernel_src_dir, file_path);
        
        // Skip files that don't exist
        if !Path::new(&full_path).exists() {
            continue;
        }
        
        // Read the source file
        let source_content = match fs::read_to_string(&full_path) {
            Ok(content) => content,
            Err(e) => {
                println!("Failed to read source file {}: {}", full_path, e);
                continue;
            }
        };
        
        // Count total lines in the file
        let total_lines = source_content.lines().count();
        
        // Create entry in file tree
        let components: Vec<&str> = file_path.split('/').collect();
        let mut current_path = String::new();
        
        for (i, component) in components.iter().enumerate() {
            if i > 0 {
                current_path.push('/');
            }
            current_path.push_str(component);
            
            if i == components.len() - 1 {
                // This is the file
                file_tree.insert(current_path.clone(), (covered_lines.len(), total_lines));
            } else {
                // This is a directory
                file_tree.entry(current_path.clone()).or_insert((0, 0));
            }
        }
        
        // Generate HTML for this file
        let file_html_path = format!("{}/{}.html", html_dir, file_path.replace("/", "_"));
        let mut file_html = File::create(&file_html_path).expect("Failed to create HTML file");
        
        // Write HTML header
        file_html.write_all(b"<!DOCTYPE html>\n<html>\n<head>\n<title>Coverage Report - ")
            .expect("Failed to write to HTML file");
        file_html.write_all(file_path.as_bytes())
            .expect("Failed to write to HTML file");
        file_html.write_all(b"</title>\n<style>\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"body { font-family: monospace; margin: 0; padding: 0; display: flex; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"#sidebar { width: 300px; height: 100vh; overflow: auto; padding: 10px; background-color: #f0f0f0; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"#content { flex-grow: 1; height: 100vh; overflow: auto; padding: 10px; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b".covered { background-color: #90EE90; }\n") // Light green
            .expect("Failed to write to HTML file");
        file_html.write_all(b".line-number { color: #888; margin-right: 10px; user-select: none; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"pre { margin: 0; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b".file-link { text-decoration: none; color: blue; display: block; margin: 2px 0; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b".directory { font-weight: bold; margin-top: 5px; cursor: pointer; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b".coverage-good { color: green; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b".coverage-medium { color: orange; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b".coverage-bad { color: red; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b".tree-toggle { cursor: pointer; user-select: none; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b".tree-toggle::before { content: '▶'; display: inline-block; margin-right: 5px; transition: transform 0.2s; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b".tree-toggle.expanded::before { transform: rotate(90deg); }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b".tree-child { margin-left: 20px; display: none; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b".tree-child.expanded { display: block; }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"</style>\n</head>\n<body>\n")
            .expect("Failed to write to HTML file");
        
        // Write sidebar placeholder - will be filled in index.html
        file_html.write_all(b"<div id=\"sidebar\">Loading sidebar...</div>\n")
            .expect("Failed to write to HTML file");
        
        // Write content
        file_html.write_all(b"<div id=\"content\">\n<h2>")
            .expect("Failed to write to HTML file");
        file_html.write_all(file_path.as_bytes())
            .expect("Failed to write to HTML file");
        file_html.write_all(b"</h2>\n<pre>\n")
            .expect("Failed to write to HTML file");
        
        // Write line by line with coverage highlighting
        for (i, line) in source_content.lines().enumerate() {
            let line_num = (i + 1) as u32;
            let class = if covered_lines.contains(&line_num) { "covered" } else { "" };
            
            file_html.write_all(format!("<div class=\"{}\"><span class=\"line-number\">{}</span>{}</div>\n", 
                class, line_num, html_escape(line)).as_bytes())
                .expect("Failed to write to HTML file");
        }
        
        file_html.write_all(b"</pre>\n</div>\n")
            .expect("Failed to write to HTML file");
        
        // Write script to load sidebar
        file_html.write_all(b"<script>\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"window.onload = function() {\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"  fetch('index.html')\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"    .then(response => response.text())\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"    .then(html => {\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"      const parser = new DOMParser();\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"      const doc = parser.parseFromString(html, 'text/html');\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"      document.getElementById('sidebar').innerHTML = doc.getElementById('sidebar').innerHTML;\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"      setupTreeToggles();\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"    });\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"}\n\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"function setupTreeToggles() {\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"  const toggles = document.querySelectorAll('.tree-toggle');\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"  toggles.forEach(toggle => {\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"    toggle.addEventListener('click', function() {\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"      this.classList.toggle('expanded');\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"      const childrenContainer = this.nextElementSibling;\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"      if (childrenContainer && childrenContainer.classList.contains('tree-child')) {\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"        childrenContainer.classList.toggle('expanded');\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"      }\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"    });\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"  });\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"}\n")
            .expect("Failed to write to HTML file");
        file_html.write_all(b"</script>\n")
            .expect("Failed to write to HTML file");
        
        file_html.write_all(b"</body>\n</html>")
            .expect("Failed to write to HTML file");
    }
    
    // Create index.html with the file tree
    let index_html_path = format!("{}/index.html", html_dir);
    let mut index_html = File::create(&index_html_path).expect("Failed to create index HTML file");
    
    // Write HTML header
    index_html.write_all(b"<!DOCTYPE html>\n<html>\n<head>\n<title>Coverage Report</title>\n<style>\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"body { font-family: monospace; margin: 0; padding: 0; display: flex; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"#sidebar { width: 300px; height: 100vh; overflow: auto; padding: 10px; background-color: #f0f0f0; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"#content { flex-grow: 1; height: 100vh; overflow: auto; padding: 10px; display: flex; justify-content: center; align-items: center; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".covered { background-color: #90EE90; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".line-number { color: #888; margin-right: 10px; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"pre { margin: 0; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".file-link { text-decoration: none; color: blue; display: block; margin: 2px 0; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".directory { font-weight: bold; margin-top: 5px; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".coverage-good { color: green; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".coverage-medium { color: orange; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".coverage-bad { color: red; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".tree-toggle { cursor: pointer; user-select: none; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".tree-toggle::before { content: '▶'; display: inline-block; margin-right: 5px; transition: transform 0.2s; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".tree-toggle.expanded::before { transform: rotate(90deg); }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".tree-child { margin-left: 20px; display: none; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b".tree-child.expanded { display: block; }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"</style>\n</head>\n<body>\n")
        .expect("Failed to write to HTML file");
    
    // Write sidebar with file tree
    index_html.write_all(b"<div id=\"sidebar\">\n<h2>Coverage Report</h2>\n")
        .expect("Failed to write to HTML file");
    
    // Organize files into a proper tree structure
    let mut tree: HashMap<String, Vec<(String, usize, usize)>> = HashMap::new(); // path -> [(name, covered, total)]
    
    // First pass: identify all directories
    for (path, (covered, total)) in &file_tree {
        let components: Vec<&str> = path.split('/').collect();
        
        // Add all parent directories to the tree
        let mut parent_path = String::new();
        for (i, component) in components.iter().enumerate() {
            if i > 0 {
                parent_path.push('/');
            }
            parent_path.push_str(component);
            
            // Create entry for parent directories if they don't exist
            if i < components.len() - 1 {
                tree.entry(if i == 0 { String::new() } else { parent_path[..parent_path.rfind('/').unwrap_or(0)].to_string() })
                    .or_insert_with(Vec::new);
            }
        }
        
        // Add file to its parent directory
        if components.len() > 1 {
            let parent = parent_path[..parent_path.rfind('/').unwrap_or(0)].to_string();
            tree.entry(parent)
                .or_insert_with(Vec::new)
                .push((path.clone(), *covered, *total));
        } else {
            // Root level file
            tree.entry(String::new())
                .or_insert_with(Vec::new)
                .push((path.clone(), *covered, *total));
        }
    }
    
    // Recursively render the tree
    render_tree(&tree, "", &mut index_html, 0);
    
    index_html.write_all(b"</div>\n")
        .expect("Failed to write to HTML file");
    
    // Write content
    index_html.write_all(b"<div id=\"content\">\n<h1>Coverage Report</h1>\n<p>Select a file from the sidebar to view coverage details.</p>\n</div>\n")
        .expect("Failed to write to HTML file");
    
    // Add JavaScript for tree toggling
    index_html.write_all(b"<script>\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"window.onload = function() {\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"  setupTreeToggles();\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"}\n\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"function setupTreeToggles() {\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"  const toggles = document.querySelectorAll('.tree-toggle');\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"  toggles.forEach(toggle => {\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"    toggle.addEventListener('click', function() {\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"      this.classList.toggle('expanded');\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"      const childrenContainer = this.nextElementSibling;\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"      if (childrenContainer && childrenContainer.classList.contains('tree-child')) {\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"        childrenContainer.classList.toggle('expanded');\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"      }\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"    });\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"  });\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"}\n")
        .expect("Failed to write to HTML file");
    index_html.write_all(b"</script>\n")
        .expect("Failed to write to HTML file");
    
    index_html.write_all(b"</body>\n</html>")
        .expect("Failed to write to HTML file");
    
    println!("Generated HTML coverage report at {}/index.html", html_dir);
}

fn render_tree(
    tree: &HashMap<String, Vec<(String, usize, usize)>>, 
    current_path: &str, 
    index_html: &mut File,
    level: usize
) {
    if let Some(children) = tree.get(current_path) {
        // Sort children: directories first, then files
        let mut dirs: Vec<&str> = Vec::new();
        let mut files: Vec<(usize, &str, usize, usize)> = Vec::new(); // (index, name, covered, total)
        
        for (i, (path, covered, total)) in children.iter().enumerate() {
            if *total == 0 {
                // This is a directory
                let name = if current_path.is_empty() {
                    path
                } else {
                    &path[current_path.len() + 1..]
                };
                
                if !name.contains('/') {
                    dirs.push(name);
                }
            } else {
                // This is a file
                let name = path.split('/').last().unwrap_or(path);
                files.push((i, name, *covered, *total));
            }
        }
        
        dirs.sort();
        files.sort_by(|a, b| a.1.cmp(b.1));
        
        // Render directories
        for dir in dirs {
            let full_path = if current_path.is_empty() {
                dir.to_string()
            } else {
                format!("{}/{}", current_path, dir)
            };
            
            // Write directory with toggle
            index_html.write_all(format!("<div class=\"tree-toggle{}\">{}/</div>\n", 
                if level == 0 { " expanded" } else { "" }, dir).as_bytes())
                .expect("Failed to write to HTML file");
            
            // Write container for children
            index_html.write_all(format!("<div class=\"tree-child{}\">\n", 
                if level == 0 { " expanded" } else { "" }).as_bytes())
                .expect("Failed to write to HTML file");
            
            // Recursively render children
            render_tree(tree, &full_path, index_html, level + 1);
            
            index_html.write_all(b"</div>\n")
                .expect("Failed to write to HTML file");
        }
        
        // Render files
        for (_, name, covered, total) in files {
            let coverage_pct = if total > 0 { (covered as f64 / total as f64) * 100.0 } else { 0.0 };
            
            // Determine color class based on coverage percentage
            let color_class = if coverage_pct >= 80.0 {
                "coverage-good"
            } else if coverage_pct >= 50.0 {
                "coverage-medium"
            } else {
                "coverage-bad"
            };
            
            let path = if current_path.is_empty() {
                format!("{}", name)
            } else {
                format!("{}/{}", current_path, name)
            };
            
            let html_path = format!("{}.html", path.replace("/", "_"));
            
            index_html.write_all(format!("<div><a href=\"{}\" class=\"file-link\">{} <span class=\"{}\">({:.1}%)</span></a></div>\n",
                html_path, name, color_class, coverage_pct).as_bytes())
                .expect("Failed to write to HTML file");
        }
    }
}

/// Helper function to escape HTML special characters
pub fn html_escape(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#39;")
}

/// Returns the path to the coverage HTML report's index file
pub fn get_coverage_html_path(work_dir: &str) -> String {
    format!("{}/coverage_html/index.html", work_dir)
}

// Add tests module at the end of the file
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::path::Path;
    use std::io::Read;

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("normal text"), "normal text");
        assert_eq!(html_escape("<script>alert('XSS')</script>"), "&lt;script&gt;alert(&apos;XSS&apos;)&lt;/script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape(r#"<div class="test">"#), "&lt;div class=&quot;test&quot;");
    }

    #[test]
    fn test_get_coverage_html_path() {
        assert_eq!(get_coverage_html_path("/tmp"), "/tmp/coverage_html/index.html");
        assert_eq!(get_coverage_html_path("/var/log"), "/var/log/coverage_html/index.html");
    }

    #[test]
    fn test_generate_coverage_html() {
        // Create temporary test directory
        let temp_dir = format!("/tmp/coverage_test_{}", std::process::id());
        let kernel_src_dir = format!("{}/kernel", temp_dir);
        let kernel_file_path = format!("{}/test_file.c", kernel_src_dir);
        
        // Create test directories
        fs::create_dir_all(&kernel_src_dir).expect("Failed to create test directory");
        
        // Create a sample source file
        let source_content = "int main() {\n    int x = 1;\n    if (x > 0) {\n        return 0;\n    }\n    return 1;\n}\n";
        fs::write(&kernel_file_path, source_content).expect("Failed to write test file");

        // Create coverage data
        let mut coverage_map = HashMap::new();
        let mut covered_lines = HashSet::new();
        covered_lines.insert(1); // Line 1 is covered
        covered_lines.insert(2); // Line 2 is covered
        covered_lines.insert(3); // Line 3 is covered
        coverage_map.insert("test_file.c".to_string(), covered_lines);

        // Generate coverage HTML
        generate_coverage_html(&coverage_map, &kernel_src_dir, &temp_dir);

        // Check if files were created
        let html_dir = format!("{}/coverage_html", temp_dir);
        let index_path = format!("{}/index.html", html_dir);
        let file_path = format!("{}/test_file.c.html", html_dir);

        assert!(Path::new(&html_dir).exists(), "HTML directory was not created");
        assert!(Path::new(&index_path).exists(), "index.html was not created");
        assert!(Path::new(&file_path).exists(), "file HTML was not created");

        // Check content of the generated file
        let mut file_content = String::new();
        let mut file = fs::File::open(&file_path).expect("Failed to open generated HTML file");
        file.read_to_string(&mut file_content).expect("Failed to read HTML file");

        // Verify that the file contains our source code
        assert!(file_content.contains("int main()"), "Generated HTML doesn't contain source code");
        
        // Verify that covered lines are marked
        assert!(file_content.contains("class=\"covered\"><span class=\"line-number\">1</span>"), 
               "Line 1 is not marked as covered");
        assert!(file_content.contains("class=\"covered\"><span class=\"line-number\">3</span>"), 
               "Line 3 is not marked as covered");
        
        // Clean up test directory
        fs::remove_dir_all(temp_dir).expect("Failed to clean up test directory");
    }
} 