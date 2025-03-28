use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::error::Error;

pub fn addr2line(vmlinux_path: &Path, pcs: Vec<u64>, kernel_src_dir: &String) -> Result<HashMap<String, HashSet<u32>>, Box<dyn Error>> {
    let file = fs::File::open(vmlinux_path)?;
    let map = unsafe { memmap2::Mmap::map(&file)? };
    let object = object::File::parse(&*map)?;
    
    // Load debugging information
    let ctx = addr2line::Context::new(&object)?;
    
    let mut result: HashMap<String, HashSet<u32>> = HashMap::new();
    
    for pc in pcs {
        let mut frames = ctx.find_frames(pc).skip_all_loads().unwrap();
        while let Ok(Some(frame)) = frames.next() {
            let Some(location) = frame.location else { continue };
            let Some(file) = location.file else { continue };
            let Some(line) = location.line else { continue };
            if line > 0 && !file.contains("?") {
                // Extract relative path for the coverage file
                let relative_path = extract_relative_path(&file, kernel_src_dir).replace("./", "");
                result.entry(relative_path).or_insert(HashSet::new()).insert(line as u32);
            }
        }
    }
    
    Ok(result)
}

/// Generate a report from a coverage file
pub fn generate_report_from_file(coverage_file: &str, kernel_src_dir: &str, work_dir: &str) -> io::Result<String> {
    // Create the work directory if it doesn't exist
    if !Path::new(work_dir).exists() {
        fs::create_dir_all(work_dir)?;
    }
    
    // Parse the coverage file
    let coverage_map = parse_coverage_file(coverage_file)?;
    println!("Parsed coverage data for {} files", coverage_map.len());
    
    // Generate the HTML report
    generate_combined_html(&coverage_map, kernel_src_dir, work_dir);
    let html_path = get_combined_html_path(work_dir);
    println!("Generated combined HTML coverage report at {}", html_path);
    
    Ok(html_path)
}

/// Parse the coverage file into a map of file paths to covered line numbers
pub fn parse_coverage_file(file_path: &str) -> io::Result<HashMap<String, HashSet<u32>>> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);
    let mut coverage_map = HashMap::new();
    
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        
        // Split the line into path and line number
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() != 2 {
            eprintln!("Warning: Invalid format in line: {}", line);
            continue;
        }
        
        let full_path = parts[0];
        let line_number = match parts[1].trim().parse::<u32>() {
            Ok(num) => num,
            Err(_) => {
                eprintln!("Warning: Invalid line number: {}", parts[1]);
                continue;
            }
        };
        
        // Extract the relative path from the full path
        let rel_path = full_path.to_string();
        
        // Add to the coverage map
        coverage_map
            .entry(rel_path)
            .or_insert_with(HashSet::new)
            .insert(line_number);
    }
    
    Ok(coverage_map)
}

/// Extract the relative path from a full path
/// root_path[-1] == "/"
pub fn extract_relative_path(full_path: &str, root_path: &str) -> String {
    // and extract the part after it
    if let Some(pos) = full_path.find(root_path) {
        return full_path[pos+root_path.len()..].to_string();
    } else {
        return full_path.to_owned();
    }
}

/// Returns the path to the combined HTML file
pub fn get_combined_html_path(work_dir: &str) -> String {
    format!("{}/coverage_report.html", work_dir)
}

/// Generates a single combined HTML coverage report from coverage data
pub fn generate_combined_html(coverage_map: &HashMap<String, HashSet<u32>>, kernel_src_dir: &str, work_dir: &str) {
    // Create a file tree structure
    let mut file_tree: HashMap<String, (usize, usize)> = HashMap::new(); // (covered_lines, total_lines)
    let mut total_covered = 0;
    let mut total_lines = 0;
    
    // Store file content and coverage data
    let mut file_data = Vec::new();
    
    // Process each file in the coverage map
    for (file_path, covered_lines) in coverage_map {
        let full_path = format!("{}/{}", kernel_src_dir, file_path);
        
        // Skip files that don't exist
        if !Path::new(&full_path).exists() {
            eprintln!("Warning: Source file not found: {}", full_path);
            continue;
        }
        
        // Read the source file
        let source_content = match fs::read_to_string(&full_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Failed to read source file {}: {}", full_path, e);
                continue;
            }
        };
        
        // Count total lines in the file
        let file_total_lines = source_content.lines().count();
        let file_covered_lines = covered_lines.len();
        
        // Update global stats
        total_covered += file_covered_lines;
        total_lines += file_total_lines;
        
        println!("Processing file: {} ({} of {} lines covered)", 
            file_path, file_covered_lines, file_total_lines);
        
        // Build file tree entries
        build_file_tree_entries(file_path, file_covered_lines, file_total_lines, &mut file_tree);
        
        // Process line coverage
        let coverage_pct = if file_total_lines > 0 { 
            (file_covered_lines as f64 / file_total_lines as f64) * 100.0 
        } else { 
            0.0 
        };
        
        // Store file data for later use in the HTML generation
        file_data.push((
            file_path.to_string(),
            source_content,
            covered_lines.clone(),
            file_covered_lines,
            file_total_lines,
            coverage_pct
        ));
    }
    
    // Create the combined HTML file
    let combined_html_path = format!("{}/coverage_report.html", work_dir);
    let mut html_file = File::create(&combined_html_path).expect("Failed to create combined HTML file");
    
    // Write HTML head with CSS and JavaScript
    write_combined_html_head(&mut html_file, &file_data)
        .expect("Failed to write HTML head");
    
    // Write body opening
    html_file.write_all(b"<body>\n").expect("Failed to write to HTML file");
    
    // Sidebar with file tree
    html_file.write_all(b"<div id=\"sidebar\" class=\"sidebar\">\n")
        .expect("Failed to write to HTML file");
    
    let overall_coverage = if total_lines > 0 { (total_covered as f64 / total_lines as f64) * 100.0 } else { 0.0 };
    
    html_file.write_all(format!(
        "<div class=\"coverage-header\">\n<h2>Coverage Report</h2>\n<div class=\"coverage-summary\">Overall: <span class=\"{}\">{:.1}%</span> ({} of {} lines)</div>\n</div>\n",
        get_coverage_class(overall_coverage),
        overall_coverage,
        total_covered,
        total_lines
    ).as_bytes()).expect("Failed to write to HTML file");
    
    // Organize files into a proper tree structure
    let mut tree: HashMap<String, Vec<(String, usize, usize)>> = HashMap::new();
    build_directory_tree(&file_tree, &mut tree);
    
    // Recursively render the tree
    render_combined_tree(&tree, "", &mut html_file, 0);
    
    html_file.write_all(b"</div>\n")
        .expect("Failed to write to HTML file");
    
    // Content area for displaying file content
    html_file.write_all(
        b"<div id=\"content\" class=\"content\">\n<div id=\"welcome\" class=\"welcome\">\n<h1>Coverage Report</h1>\n<p>Select a file from the sidebar to view coverage details.</p>\n<p>Generated with FFFuzzer coverage tool</p>\n</div>\n"
    ).expect("Failed to write to HTML file");
    
    // Create containers for each file's content (initially hidden)
    for (file_path, _, _, _, _, _) in &file_data {
        let file_id = file_path.replace("/", "_").replace(".", "_");
        html_file.write_all(format!(
            "<div id=\"file_{}\" class=\"file-content\" style=\"display:none;\"></div>\n",
            file_id
        ).as_bytes()).expect("Failed to write to HTML file");
    }
    
    html_file.write_all(b"</div>\n")
        .expect("Failed to write to HTML file");
    
    // Write JavaScript code for functions and data
    html_file.write_all(b"<script>\n").expect("Failed to write to HTML file");
    
    // File data objects
    html_file.write_all(b"const fileData = {\n").expect("Failed to write to HTML file");
    
    for (file_path, source_content, covered_lines, covered_count, total_lines, coverage_pct) in &file_data {
        let file_id = file_path.replace("/", "_").replace(".", "_");
        
        // Convert the covered lines to a JSON array
        let covered_lines_json = covered_lines.iter()
            .map(|line| line.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        
        // Prepare the source content lines for JSON
        // Don't html_escape here since we'll use innerHTML to render it properly
        let source_lines: Vec<String> = source_content.lines()
            .map(|line| line.replace("\\", "\\\\").replace("\"", "\\\""))
            .collect();
        
        let source_json = source_lines.iter()
            .map(|line| format!("\"{}\"", line))
            .collect::<Vec<String>>()
            .join(",\n        ");
        
        html_file.write_all(format!(
            "  \"{}\": {{\n    path: \"{}\",\n    covered: [{}],\n    totalLines: {},\n    coveredCount: {},\n    coveragePct: {:.1},\n    source: [\n        {}\n    ]\n  }},\n",
            file_id, file_path, covered_lines_json, total_lines, covered_count, coverage_pct, source_json
        ).as_bytes()).expect("Failed to write to HTML file");
    }
    
    html_file.write_all(b"};\n\n").expect("Failed to write to HTML file");
    
    // Write JavaScript functions
    html_file.write_all(r#"
// Function to safely display source code
function displaySourceSafely(text) {
  // First encode all HTML entities to prevent XSS attacks
  const encodedText = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
    
  // Replace encoded preprocessor directives to display them nicely
  // This handles #include<xxx> and #include <xxx> formats
  return encodedText
    .replace(/(#\s*include\s*)&lt;([^&]+)&gt;/g, '$1<span class="include-brackets">&lt;</span>$2<span class="include-brackets">&gt;</span>')
    .replace(/(#\s*define\s*[^&\s]+\s*)&lt;([^&]+)&gt;/g, '$1<span class="include-brackets">&lt;</span>$2<span class="include-brackets">&gt;</span>');
}

// Function to show a specific file
function showFile(fileId) {
  // Hide welcome message and all file content
  document.getElementById('welcome').style.display = 'none';
  const fileContainers = document.querySelectorAll('.file-content');
  fileContainers.forEach(container => {
    container.style.display = 'none';
  });
  
  // Get the file container
  const fileContainer = document.getElementById('file_' + fileId);
  if (!fileContainer) return;
  
  // If the file hasn't been loaded yet, generate the content
  if (fileContainer.innerHTML === '') {
    const data = fileData[fileId];
    if (!data) return;
    
    // Create file header
    const header = document.createElement('div');
    header.className = 'file-header';
    header.innerHTML = `
      <h2>${data.path}</h2>
      <div class=\"coverage-summary\">Coverage: <span class=\"${getCoverageClass(data.coveragePct)}\">${data.coveragePct.toFixed(1)}%</span> (${data.coveredCount} of ${data.totalLines} lines)</div>
    `;
    fileContainer.appendChild(header);
    
    // Create source code container
    const pre = document.createElement('pre');
    pre.className = 'source-code';
    
    // Add each line
    for (let i = 0; i < data.source.length; i++) {
      const lineNum = i + 1;
      const isCovered = data.covered.includes(lineNum);
      const lineDiv = document.createElement('div');
      lineDiv.className = 'line' + (isCovered ? ' covered' : '');
      
      const lineNumSpan = document.createElement('span');
      lineNumSpan.className = 'line-number';
      lineNumSpan.textContent = lineNum;
      
      const lineContentSpan = document.createElement('span');
      lineContentSpan.className = 'line-content';
      // Use our custom function to safely display source code with proper formatting
      lineContentSpan.innerHTML = displaySourceSafely(data.source[i]);
      
      lineDiv.appendChild(lineNumSpan);
      lineDiv.appendChild(lineContentSpan);
      pre.appendChild(lineDiv);
    }
    
    fileContainer.appendChild(pre);
  }
  
  // Show the file container
  fileContainer.style.display = 'block';
  
  // Highlight the selected file in the sidebar
  const fileLinks = document.querySelectorAll('.file-link');
  fileLinks.forEach(link => {
    link.parentElement.classList.remove('active');
    if (link.getAttribute('data-id') === fileId) {
      link.parentElement.classList.add('active');
      
      // Expand parent directories
      let parent = link.parentElement.parentElement;
      while (parent) {
        if (parent.classList.contains('tree-child')) {
          parent.classList.add('expanded');
          const toggle = parent.previousElementSibling;
          if (toggle && toggle.classList.contains('tree-toggle')) {
            toggle.classList.add('expanded');
          }
        }
        parent = parent.parentElement;
      }
    }
  });
}

// Function to get coverage class based on percentage
function getCoverageClass(percentage) {
  if (percentage >= 80.0) {
    return 'coverage-good';
  } else if (percentage >= 50.0) {
    return 'coverage-medium';
  } else {
    return 'coverage-bad';
  }
}

// Set up tree toggles
function setupTreeToggles() {
  const toggles = document.querySelectorAll('.tree-toggle');
  toggles.forEach(toggle => {
    toggle.addEventListener('click', function() {
      this.classList.toggle('expanded');
      const childrenContainer = this.nextElementSibling;
      if (childrenContainer && childrenContainer.classList.contains('tree-child')) {
        childrenContainer.classList.toggle('expanded');
      }
    });
  });
}

// Initialize when the page loads
window.onload = function() {
  setupTreeToggles();
};
"#.as_bytes()).expect("Failed to write to HTML file");
    
    html_file.write_all(b"</script>\n").expect("Failed to write to HTML file");
    
    // Close the HTML
    html_file.write_all(b"</body>\n</html>\n").expect("Failed to write to HTML file");
    
    println!("Coverage summary: {} of {} lines covered ({:.2}%)", 
        total_covered, total_lines, 
        if total_lines > 0 { (total_covered as f64 / total_lines as f64) * 100.0 } else { 0.0 });
}

/// Builds file tree entries for a given file path
fn build_file_tree_entries(file_path: &str, covered_lines: usize, total_lines: usize, file_tree: &mut HashMap<String, (usize, usize)>) {
    let components: Vec<&str> = file_path.split('/').collect();
    let mut current_path = String::new();
    
    for (i, component) in components.iter().enumerate() {
        if i > 0 {
            current_path.push('/');
        }
        current_path.push_str(component);
        
        if i == components.len() - 1 {
            // This is the file
            file_tree.insert(current_path.clone(), (covered_lines, total_lines));
        } else {
            // This is a directory - initialize if not exists
            file_tree.entry(current_path.clone()).or_insert((0, 0));
        }
    }
}

/// Builds a directory tree structure from file entries
fn build_directory_tree(file_tree: &HashMap<String, (usize, usize)>, tree: &mut HashMap<String, Vec<(String, usize, usize)>>) {
    // First pass: identify all directories
    for (path, (covered, total)) in file_tree {
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
                let parent_dir = if i == 0 { 
                    String::new() 
                } else { 
                    parent_path[..parent_path.rfind('/').unwrap_or(0)].to_string() 
                };
                tree.entry(parent_dir).or_insert_with(Vec::new);
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
}

/// Recursively renders the directory tree for the combined HTML
fn render_combined_tree(
    tree: &HashMap<String, Vec<(String, usize, usize)>>, 
    current_path: &str, 
    html_file: &mut File,
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
            html_file.write_all(format!(
                "<div class=\"directory\">\n<div class=\"tree-toggle{}\">{}/</div>\n",
                if level == 0 { " expanded" } else { "" }, dir
            ).as_bytes()).expect("Failed to write to HTML file");
            
            // Write container for children
            html_file.write_all(format!(
                "<div class=\"tree-child{}\">\n",
                if level == 0 { " expanded" } else { "" }
            ).as_bytes()).expect("Failed to write to HTML file");
            
            // Recursively render children
            render_combined_tree(tree, &full_path, html_file, level + 1);
            
            html_file.write_all(b"</div>\n</div>\n")
                .expect("Failed to write to HTML file");
        }
        
        // Render files
        for (_, name, covered, total) in files {
            let coverage_pct = if total > 0 { (covered as f64 / total as f64) * 100.0 } else { 0.0 };
            let color_class = get_coverage_class(coverage_pct);
            
            let path = if current_path.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", current_path, name)
            };
            
            let file_id = path.replace("/", "_").replace(".", "_");
            
            html_file.write_all(format!(
                "<div class=\"file-entry\"><a href=\"javascript:void(0)\" onclick=\"showFile('{}')\" class=\"file-link\" data-id=\"{}\">{} <span class=\"coverage-badge {}\">({:.1}%)</span></a></div>\n",
                file_id, file_id, name, color_class, coverage_pct
            ).as_bytes()).expect("Failed to write to HTML file");
        }
    }
}

/// Writes the HTML head with CSS styles for the combined HTML
fn write_combined_html_head(file: &mut File, file_data: &[(String, String, HashSet<u32>, usize, usize, f64)]) -> std::io::Result<()> {
    file.write_all(b"<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n<title>Combined Coverage Report</title>\n<style>\n")?;
    
    // Write CSS styles
    file.write_all(b"
:root {
    --bg-color: #fff;
    --text-color: #333;
    --sidebar-bg: #f5f5f5;
    --sidebar-hover: #e0e0e0;
    --line-highlight: #90EE90;
    --line-number-color: #888;
    --link-color: #0066cc;
    --border-color: #ddd;
    --toggle-color: #555;
    --good-color: #4caf50;
    --medium-color: #ff9800;
    --bad-color: #f44336;
    --header-bg: #f0f0f0;
}

@media (prefers-color-scheme: dark) {
    :root {
        --bg-color: #1e1e1e;
        --text-color: #e0e0e0;
        --sidebar-bg: #252525;
        --sidebar-hover: #333;
        --line-highlight: #2d4f2d;
        --line-number-color: #888;
        --link-color: #4b98e0;
        --border-color: #444;
        --toggle-color: #aaa;
        --good-color: #4caf50;
        --medium-color: #ff9800;
        --bad-color: #f44336;
        --header-bg: #2a2a2a;
    }
}

* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
    color: var(--text-color);
    background: var(--bg-color);
    display: flex;
    height: 100vh;
    overflow: hidden;
    margin: 0;
}

.sidebar {
    width: 300px;
    height: 100vh;
    overflow: auto;
    padding: 15px;
    background-color: var(--sidebar-bg);
    border-right: 1px solid var(--border-color);
    position: relative;
}

.content {
    flex-grow: 1;
    height: 100vh;
    overflow: auto;
    padding: 15px;
}

.coverage-header, .file-header {
    padding-bottom: 15px;
    margin-bottom: 15px;
    border-bottom: 1px solid var(--border-color);
}

.coverage-summary {
    margin-top: 8px;
    font-size: 14px;
}

.coverage-good { color: var(--good-color); }
.coverage-medium { color: var(--medium-color); }
.coverage-bad { color: var(--bad-color); }

.directory {
    margin: 4px 0;
}

.file-entry {
    margin: 4px 0;
    padding-left: 3px;
}

.file-entry.active .file-link {
    background-color: var(--sidebar-hover);
    font-weight: bold;
}

.file-link {
    text-decoration: none;
    color: var(--link-color);
    display: block;
    padding: 4px 8px;
    border-radius: 3px;
    transition: background-color 0.2s;
}

.file-link:hover {
    background-color: var(--sidebar-hover);
}

.coverage-badge {
    font-size: 0.85em;
    margin-left: 5px;
}

.tree-toggle {
    cursor: pointer;
    user-select: none;
    padding: 4px 8px;
    border-radius: 3px;
    transition: background-color 0.2s;
    position: relative;
    font-weight: 500;
}

.tree-toggle:hover {
    background-color: var(--sidebar-hover);
}

.tree-toggle::before {
    content: '\xE2\x96\xB6';
    display: inline-block;
    margin-right: 5px;
    font-size: 0.9em;
    transition: transform 0.2s;
    color: var(--toggle-color);
}

.tree-toggle.expanded::before {
    transform: rotate(90deg);
}

.tree-child {
    margin-left: 15px;
    display: none;
    border-left: 1px solid var(--border-color);
    padding-left: 10px;
}

.tree-child.expanded {
    display: block;
}

.source-code {
    margin: 0;
    font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
    background-color: var(--bg-color);
    line-height: 1.5;
    overflow-x: auto;
    tab-size: 4;
}

.line {
    display: flex;
    white-space: pre;
}

.line.covered {
    background-color: var(--line-highlight);
}

.line-number {
    color: var(--line-number-color);
    padding: 0 12px;
    margin-right: 12px;
    text-align: right;
    user-select: none;
    border-right: 1px solid var(--border-color);
    min-width: 40px;
}

.line-content {
    flex: 1;
}

.include-brackets {
    color: var(--text-color);
}

.welcome {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
}

.welcome h1 {
    margin-bottom: 20px;
}
")?;
    
    file.write_all(b"</style>\n</head>\n")?;
    Ok(())
}

/// Helper function to get CSS class based on coverage percentage
fn get_coverage_class(percentage: f64) -> &'static str {
    if percentage >= 80.0 {
        "coverage-good"
    } else if percentage >= 50.0 {
        "coverage-medium"
    } else {
        "coverage-bad"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_combined_coverage_html() {
        let coverage_file = "/home/xxx/FFFuzzer/work_dir/coverage_copy.txt";
        let kernel_src_dir = "/home/xxx/FFFuzzer/linux-6.13.8";
        let work_dir = "/home/xxx/FFFuzzer/work_dir";
        
        generate_report_from_file(coverage_file, kernel_src_dir, work_dir).unwrap();
    }
}