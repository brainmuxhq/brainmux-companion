//! Yerel çalıştırma mantığı (ADR-0011) — provizyon + Zero-State + core/console spawn. Tauri kabuğu
//! bunu çağırır; UI native pencerede (webview → localhost konsol). İş mantığı core'da (SSoT), burada yok.
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

const OLLAMA: &str = "http://127.0.0.1:11434";
pub const CORE_ADDR: &str = "127.0.0.1:8787";
pub const CONSOLE_ADDR: &str = "127.0.0.1:3100";
pub const CONSOLE_URL: &str = "http://127.0.0.1:3100/console/moduller";
const MODEL: &str = "bge-m3";

fn log(m: &str) {
    println!("[brainmux] {m}");
}

pub fn notify(title: &str, body: &str) {
    let _ = Command::new("notify-send")
        .args(["-a", "brainmux", "-i", "brainmux", title, body])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"))
}
fn bmux_home() -> PathBuf {
    std::env::var("BRAINMUX_HOME").map(PathBuf::from).unwrap_or_else(|_| home().join(".brainmux"))
}
fn repo() -> PathBuf {
    std::env::var("BRAINMUX_REPO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join("Development/Projects/brainmux/brainmux"))
}

fn provision_dirs() {
    let base = bmux_home();
    for sub in ["", "logs", "models", "knowledge", "output", "runtime"] {
        let _ = fs::create_dir_all(base.join(sub));
    }
}

fn ollama_up() -> bool {
    ureq::get(&format!("{OLLAMA}/api/version")).timeout(Duration::from_secs(2)).call().is_ok()
}
fn model_present() -> bool {
    match ureq::get(&format!("{OLLAMA}/api/tags")).timeout(Duration::from_secs(3)).call() {
        Ok(r) => r
            .into_json::<serde_json::Value>()
            .ok()
            .and_then(|v| {
                v["models"].as_array().map(|ms| {
                    ms.iter().any(|m| {
                        m["name"].as_str().map_or(false, |n| n == MODEL || n.split(':').next() == Some(MODEL))
                    })
                })
            })
            .unwrap_or(false),
        Err(_) => false,
    }
}
fn ensure_ollama() {
    if ollama_up() {
        return;
    }
    let _ = Command::new("ollama").arg("serve").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(500));
        if ollama_up() {
            return;
        }
    }
}
fn ensure_model() {
    if model_present() {
        return;
    }
    notify("brainmux — ilk kurulum", "Yapay zeka modeli iniyor (~1GB, birkaç dakika)…");
    if let Ok(resp) = ureq::post(&format!("{OLLAMA}/api/pull")).send_json(serde_json::json!({"model": MODEL})) {
        let reader = BufReader::new(resp.into_reader());
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                if let (Some(c), Some(t)) = (v["completed"].as_f64(), v["total"].as_f64()) {
                    if t > 0.0 {
                        print!("\r[brainmux]   {} %{:.0}     ", v["status"].as_str().unwrap_or(""), c / t * 100.0);
                        let _ = std::io::stdout().flush();
                    }
                }
            }
        }
        println!();
    }
}

fn spawn_grouped(mut cmd: Command) -> std::io::Result<Child> {
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    cmd.spawn()
}
fn kill_pid_group(pid: i32) {
    unsafe {
        libc::kill(-pid, libc::SIGTERM);
    }
}
pub fn teardown(pids: &[i32]) {
    for &p in pids {
        kill_pid_group(p);
    }
}
fn wait_port(addr: &str) -> bool {
    for _ in 0..120 {
        if std::net::TcpStream::connect(addr).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_secs(1));
    }
    false
}
pub fn port_open(addr: &str) -> bool {
    std::net::TcpStream::connect(addr).is_ok()
}

/// Gömülü çekirdek: paketlenmiş portable-python (ADR-0011 §7). `<res>/core-bundle/py/bin/python3`.
/// Yoksa dev fallback (uv-repo). Modüller de gömülü resource'tan (`<res>/modules`).
pub struct Bundle {
    pub core_python: Option<PathBuf>, // gömülü python; None → dev (uv-repo)
    pub modules_dir: Option<PathBuf>, // gömülü modules; None → core kendi çözer
    pub console_dist: Option<PathBuf>, // gömülü static konsol; None → dev (npm)
}

fn core_command(b: &Bundle) -> Command {
    match &b.core_python {
        Some(py) => {
            // Self-contained: repo/uv GEREKMEZ. Paket venv'de kurulu → --app-dir yok.
            let mut c = Command::new(py);
            c.args([
                "-m", "uvicorn", "brainmux_core.api.app:app", "--host", "127.0.0.1", "--port", "8787",
            ]);
            if let Some(md) = &b.modules_dir {
                c.env("BRAINMUX_MODULES_DIR", md);
            }
            c
        }
        None => {
            // Dev fallback: repo'dan uv.
            let mut c = Command::new("uv");
            c.args([
                "run", "--project", "apps/core", "--env-file", ".env", "--", "uvicorn",
                "brainmux_core.api.app:app", "--host", "127.0.0.1", "--port", "8787", "--app-dir",
                "apps/core/src",
            ])
            .current_dir(repo());
            c
        }
    }
}

fn console_command(b: &Bundle) -> Command {
    match &b.console_dist {
        // TODO(Faz-2b): gömülü static konsol'u yerel statik sunucuyla serve et.
        Some(_dist) => {
            let mut c = Command::new("npm");
            c.args(["--prefix", "apps/app", "run", "dev", "--", "-p", "3100"]).current_dir(repo());
            c.env("NEXT_PUBLIC_CORE_URL", "http://127.0.0.1:8787");
            c
        }
        None => {
            let mut c = Command::new("npm");
            c.args(["--prefix", "apps/app", "run", "dev", "--", "-p", "3100"]).current_dir(repo());
            c.env("NEXT_PUBLIC_CORE_URL", "http://127.0.0.1:8787");
            c
        }
    }
}

/// Provizyon + core + console. Spawn edilen süreç-grubu pid'lerini döndürür (teardown için).
/// Konsol :3100 hazır olunca `true` (webview oraya gidebilir).
pub fn start(bundle: &Bundle) -> (Vec<i32>, bool) {
    let mut pids = Vec::new();
    provision_dirs();
    ensure_ollama();
    ensure_model();

    match spawn_grouped(core_command(bundle)) {
        Ok(c) => {
            pids.push(c.id() as i32);
            std::mem::forget(c); // grup pid ile teardown'da öldürülür
        }
        Err(e) => log(&format!("HATA: çekirdek başlatılamadı ({e})")),
    }
    wait_port(CORE_ADDR);

    match spawn_grouped(console_command(bundle)) {
        Ok(c) => {
            pids.push(c.id() as i32);
            std::mem::forget(c);
        }
        Err(e) => log(&format!("UYARI: konsol başlatılamadı ({e})")),
    }
    let ready = wait_port(CONSOLE_ADDR);
    (pids, ready)
}
