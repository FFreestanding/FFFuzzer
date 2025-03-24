use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet};
use std::net::{TcpListener, TcpStream};
use std::result::Result::Ok;
use lazy_static::lazy_static;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use anyhow::{Context, Result};
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

#[derive(Serialize)]
struct StatusData {
    uptime_seconds: u64,
    memory_usage_mb: f64,
    cpu_usage_percent: f64,
    requests_processed: u64,
}

#[derive(Default, Debug, Deserialize, Clone)]
struct Config {
    qemu_path: String,
    bzimage_path: String,
    image_path: String,
    dict_path: String,
    procs: usize,
    kernel_src_dir: String,
    work_dir: String,
    agent_dir: String,
    corpus_dir: String,
}

enum CMD {
    NeedCov = 0x66
}

static SOCKET_PORT_BASE: usize = 8888;
static CONFIG_INFO: OnceLock<Config> = OnceLock::new();
// static PCS: Mutex<HashSet<u64>> = Mutex::new(HashSet::new());
lazy_static! {
    static ref PCS: Mutex<HashSet<u64>> = Mutex::new(HashSet::new());
}

fn read_config(file_path: &str) -> Result<Config> {
    let config_str = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read config file: {}", file_path))?;
    let config: Config = toml::from_str(&config_str)
        .with_context(|| "Failed to parse TOML config")?;
    Ok(config)
}

fn check_kvm_support() -> bool {
    // 检查 /dev/kvm 是否存在
    let kvm_path = "/dev/kvm";
    if !Path::new(kvm_path).exists() {
        return false;
    }
    // 尝试打开 /dev/kvm 检查权限
    if open(kvm_path, OFlag::O_RDWR, Mode::empty()).is_ok(){
        true
    } else {
        false
    }
}

fn run_qemu(config: Config, i: usize, qemu_snap_args: String, port: usize) {
    let process_dir = format!("{}/fuzz_{}", &config.work_dir, i);
    let process_dir_path = Path::new(&process_dir);
    if !process_dir_path.exists() {
        fs::create_dir(&process_dir).expect(&format!("create_dir {} error\n", &process_dir));
        println!("Created directory: {}", &process_dir);
    }
    let log_file_path = format!("{}/log_{}", &process_dir, i);
    let log_file = File::create(&log_file_path)
        .with_context(|| format!("Failed to create log file: {}", &log_file_path));
    let log_file = log_file.unwrap();
    let _ = thread::spawn(move || {
        let handle = Command::new(&config.qemu_path)
        .args(&[
            "-rss_limit_mb=8096",
            "-use_value_profile=1",
            "-detect_leaks=0",
            &format!("-dict={}", &config.dict_path),
            "-len_control=200", "-reload=60",
            &config.corpus_dir,
        ])
        .env("QEMU_SNAP_ARGS", &qemu_snap_args)
        .env("FUZZ_ID", i.to_string())
        .env("PORT", port.to_string())
        .env("WORK_DIR", &config.work_dir)
        .env("FUZZ_TRACE_PC", "1")
        .env("PRINT_ALL_PCS", "1")
        .env("MUTATE_SYSCALLS", "1")
        .env("PRINT_CRASHES", "1")
        .stdout(Stdio::from(log_file.try_clone().expect("Failed to clone log file for stdout")))
        .stderr(Stdio::from(log_file.try_clone().expect("Failed to clone log file for stderr")))
        .spawn().expect(&format!("Spawned QEMU process {} error", i));
        println!("Spawned QEMU process {} (PID: {})", i, handle.id());
    });
}

fn spawn_qemu_processes(config: Config) -> web::Data<Vec<Arc<Mutex<std::net::TcpStream>>>> {
    let qemu_snap_args;
    if check_kvm_support() {
        println!("KVM is supported");
        // qemu_snap_args = format!("-cpu host,kvm=on,svm=on \
        //     -machine q35,vmport=off,smbus=off,acpi=off,usb=off,graphics=off -m 1G \
        //     -kernel {} \
        //     -append 'root=/dev/vda earlyprintk=ttyS0 console=ttyS0 nokaslr silent notsc acpi=off \
        //     kvm-intel.nested=1 kvm-intel.unrestricted_guest=1 kvm-intel.vmm_exclusive=1 kvm-intel.fasteoi=1 \
        //     kvm-intel.ept=1 kvm-intel.flexpriority=1 kvm-intel.vpid=1 kvm-intel.emulate_invalid_guest_state=1 \
        //     kvm-intel.eptad=1 kvm-intel.enable_shadow_vmcs=1 kvm-intel.pml=1 kvm-intel.enable_apicv=1' \
        //     -drive file={},id=dr0,format=raw,if=none \
        //     -virtfs local,path={},mount_tag=host0,security_model=none,id=host0,readonly=on \
        //     -device virtio-blk-pci,drive=dr0 \
        //     -nographic -accel kvm -nodefaults \
        //     -drive file=null-co://,if=none,id=nvm  -vga virtio \
        //     -device megasas,id=scsi0 \
        //     -device scsi-hd,drive=drive0,bus=scsi0.0,channel=0,scsi-id=0,lun=0 \
        //     -drive file=null-co://,if=none,id=drive0 \
        //     -device nvme,serial=deadbeef,drive=nvm \
        //     -serial none -snapshot -cdrom /dev/null",
        //     &config.bzimage_path,
        //     &config.image_path,
        //     &config.agent_dir);
        qemu_snap_args = format!("-cpu host,kvm=on,svm=on \
        -machine q35,vmport=off,smbus=off,acpi=off,usb=off,graphics=off -m 1G \
        -kernel {} \
        -append 'root=/dev/vda earlyprintk=ttyS0 console=ttyS0 nokaslr silent notsc acpi=off \
        kvm-amd.nested=1 kvm-amd.npt=1 kvm-amd.avic=1 kvm-amd.vls=1 kvm-amd.sev=0' \
        -drive file={},id=dr0,format=raw,if=none \
        -virtfs local,path={},mount_tag=host0,security_model=none,id=host0,readonly=on \
        -device virtio-blk-pci,drive=dr0 \
        -nographic -accel kvm -nodefaults \
        -drive file=null-co://,if=none,id=nvm  -vga virtio \
        -device megasas,id=scsi0 \
        -device scsi-hd,drive=drive0,bus=scsi0.0,channel=0,scsi-id=0,lun=0 \
        -drive file=null-co://,if=none,id=drive0 \
        -device nvme,serial=deadbeef,drive=nvm \
        -serial none -snapshot -cdrom /dev/null -serial stdio",// -serial stdio
        &config.bzimage_path,
        &config.image_path,
        &config.agent_dir);
        // println!("{}", qemu_snap_args);
    } else {
        panic!("KVM is not supported");
    }

    let mut clients = Vec::new();
    for i in 0..config.procs {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", SOCKET_PORT_BASE+i)).expect("TcpListener::bind error\n");
        println!("Controller listening on 127.0.0.1:{}", SOCKET_PORT_BASE+i);
        run_qemu(config.clone(), i, qemu_snap_args.clone(), SOCKET_PORT_BASE+i);
        let (stream, _) = listener.accept().expect("listener accept error\n");
        clients.push(Arc::new(Mutex::new(stream)));
    }
    web::Data::new(clients)
}

async fn coverage_handler(clients: web::Data<Vec<Arc<Mutex<TcpStream>>>>) -> impl Responder {
    println!("coverage_handler");
    for client in clients.iter() {
        let mut stream = client.lock().unwrap();
        if let Err(e) = stream.write_all(&[CMD::NeedCov as u8]) {
            println!("Write failed: {}", e);
            continue;
        }
        stream.flush().unwrap();
    }

    for client in clients.iter() {
        let mut stream = client.lock().unwrap();
        let mut len_buf = [0; 8];
        if let Err(e) = stream.read_exact(&mut len_buf) {
            println!("Read length failed: {}", e);
            continue;
        }
        let data_len = usize::from_le_bytes(len_buf); // Match QEMU's little-endian
        println!("Receiving data ({} bytes)", data_len);

        if data_len % 8 != 0 {
            println!("Invalid data length: {}", data_len);
            continue;
        }

        let mut data = vec![0u8; data_len];
        if let Err(e) = stream.read_exact(&mut data) {
            println!("Read data failed: {}", e);
            continue;
        }
        println!("Received data ({} bytes)", data_len);

        let mut pcs = PCS.lock().unwrap();
        for chunk in data.chunks_exact(8) {
            let pc = u64::from_le_bytes(chunk.try_into().unwrap());
            pcs.insert(pc);
        }
    }
    let coverage_file_path = format!("{}/coverage.txt", &CONFIG_INFO.get().unwrap().work_dir);

    let mut file = OpenOptions::new()
    .write(true)
    .append(true)  // Enable append mode
    .create(true)  // Create file if it doesn't exist
    .open(coverage_file_path)
    .expect("无法创建文件");

    let g = PCS.lock().unwrap();
    for pc in g.iter() {
        let output = Command::new("addr2line")
        .args(&[
            "-e",
            &format!("{}/{}", &CONFIG_INFO.get().unwrap().kernel_src_dir, "vmlinux"),
            &format!("{:x}", pc),
        ])
        .stdout(Stdio::piped())
        .output()  // Execute the command and get the output
        .expect("Failed to execute addr2line");
        if !output.stderr.is_empty() {
            println!("write coverage file error: {:?}", output.stderr);
        } else {
            println!("{}", &format!("{:x}", pc));
            // Write the command output to the file
            file.write_all(&output.stdout)
                .expect("Failed to write stdout to file");
        }

    }
    let pcs_count = g.len();
    HttpResponse::Ok().body(format!("cov: {}", pcs_count))
}

async fn index() -> impl Responder {
    match std::fs::read_to_string("index.html") {
        Ok(content) => HttpResponse::Ok()
            .content_type("text/html")
            .body(content),
        Err(_) => HttpResponse::InternalServerError().body("Failed to load index.html"),
    }
}

async fn get_status() -> impl Responder {
    // Simulated data (replace with actual metrics from your application)
    let start_time = SystemTime::now();
    let uptime_seconds = start_time
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let status_data = StatusData {
        uptime_seconds,
        memory_usage_mb: 512.3, // Replace with actual memory usage
        cpu_usage_percent: 45.6, // Replace with actual CPU usage
        requests_processed: 12345, // Replace with actual request count
    };

    HttpResponse::Ok().json(status_data)
}

#[actix_web::main]
async fn main() {
    let config = read_config("config.toml").unwrap();
    CONFIG_INFO.set(config.clone()).unwrap();
    if !Path::new(&config.qemu_path).exists() {
        println!("QEMU command {} does not exist", config.qemu_path);
    }
    if !Path::new(&config.work_dir).exists() {
        println!("Work directory {} does not exist", config.work_dir);
    }
    if !Path::new(&config.kernel_src_dir).exists() {
        println!("Kernel Source directory {} does not exist", config.kernel_src_dir);
    }
    let clients = spawn_qemu_processes(config);
    
    let _ = HttpServer::new( move || {
        App::new()
        .app_data(clients.clone())
        .route("/", web::get().to(index))
        .route("/cov", web::get().to(coverage_handler))
        .route("/status", web::get().to(get_status))
    })
    .bind("127.0.0.1:8080").expect("")
    .run()
    .await;

    // Ok(())
}