use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

// Import the coverage module from our crate
use web::coverage;

fn main() {
    println!("Testing coverage.rs module...");
    
    // Create temporary test directories
    let temp_dir = format!("/tmp/coverage_test_{}", std::process::id());
    let kernel_src_dir = format!("{}/kernel", temp_dir);
    let kernel_file_paths = [
        format!("{}/drivers/net/ethernet/test_driver.c", kernel_src_dir),
        format!("{}/fs/ext4/test_fs.c", kernel_src_dir),
        format!("{}/kernel/sched/core.c", kernel_src_dir),
    ];
    
    // Create directory structure
    for path in &kernel_file_paths {
        let dir_path = Path::new(path).parent().unwrap();
        fs::create_dir_all(dir_path).expect("Failed to create test directory");
    }
    
    // Create some sample source files
    let source_contents = [
        "/* Test driver file */\nint init_module() {\n    int ret = 0;\n    if (probe_hardware()) {\n        ret = register_driver();\n    }\n    return ret;\n}\n",
        "/* Filesystem test */\nstruct file_operations ext4_file_ops = {\n    .read = ext4_read,\n    .write = ext4_write,\n    .open = ext4_open,\n};\n",
        "/* Scheduler core */\nvoid schedule() {\n    pick_next_task();\n    context_switch();\n}\n",
    ];
    
    for (i, path) in kernel_file_paths.iter().enumerate() {
        fs::write(path, source_contents[i]).expect("Failed to write test file");
    }
    
    // Create coverage data
    let mut coverage_map = HashMap::new();
    
    // Driver file - lines 1, 2, 3, 4, 5 covered
    let mut covered_lines = HashSet::new();
    for line in 1..=5 {
        covered_lines.insert(line);
    }
    coverage_map.insert("drivers/net/ethernet/test_driver.c".to_string(), covered_lines);
    
    // Filesystem file - lines 1, 2, 3 covered
    let mut covered_lines = HashSet::new();
    for line in 1..=3 {
        covered_lines.insert(line);
    }
    coverage_map.insert("fs/ext4/test_fs.c".to_string(), covered_lines);
    
    // Scheduler file - lines 1, 2 covered (line 3 not covered)
    let mut covered_lines = HashSet::new();
    for line in 1..=2 {
        covered_lines.insert(line);
    }
    coverage_map.insert("kernel/sched/core.c".to_string(), covered_lines);
    
    // Generate coverage HTML
    println!("Generating HTML coverage report...");
    coverage::generate_coverage_html(&coverage_map, &kernel_src_dir, &temp_dir);
    
    // Report path to coverage report
    let html_path = coverage::get_coverage_html_path(&temp_dir);
    println!("\nCoverage report generated at: {}", html_path);
    println!("Open it in your browser to view the report.\n");
    println!("To remove test files, run: rm -rf {}", temp_dir);
} 