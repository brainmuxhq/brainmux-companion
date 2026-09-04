//! Yerel çalıştırma mantığı (ADR-0011) — dizin provizyonu + core/console spawn + temiz kapanış.
//! Tauri kabuğu bunu çağırır; UI native pencerede (webview → localhost konsol). İş mantığı core'da (SSoT).
//! Ollama/model provizyonu BURADA DEĞİL — on-demand core bootstrap'ta (RAG yeteneği kullanılınca,
//! modül "Kur" → /components/local-rag/provision). App açılışı hızlı; 1GB model açılışta inmez (ADR-0013).
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

pub const CORE_ADDR: &str = "127.0.0.1:8787";
pub const CONSOLE_ADDR: &str = "127.0.0.1:3100";
pub const CONSOLE_URL: &str = "http://127.0.0.1:3100/console/moduller";

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

/// Gömülü çekirdek: paketlenmiş portable-python (ADR-0011 §7). Yoksa dev fallback (uv-repo).
/// Modüller de gömülü resource'tan (`<res>/bundle/modules`); yoksa core repo'dan çözer.
pub struct Bundle {
    pub core_python: Option<PathBuf>,
    pub modules_dir: Option<PathBuf>,
    // console_dist (gömülü static konsol) = Faz-2b'de eklenecek (şimdi konsol repo'dan npm dev).
}

fn core_command(b: &Bundle) -> Command {
    match &b.core_python {
        Some(py) => {
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

fn console_command(_b: &Bundle) -> Command {
    // TODO(Faz-2b): console_dist varsa gömülü static konsol'u yerel statik sunucuyla serve et.
    let mut c = Command::new("npm");
    c.args(["--prefix", "apps/app", "run", "dev", "--", "-p", "3100"]).current_dir(repo());
    c.env("NEXT_PUBLIC_CORE_URL", "http://127.0.0.1:8787");
    // Desktop'ta konsol WEB-GATE KAPALI (127.0.0.1, dev-bypass) — kimlik app-seviyesi (üyelik,
    // sistem-tarayıcı OAuth → deep-link; Faz-4). Web Supabase gate 127.0.0.1'de çerez tutmaz → login
    // döngüsü olur. Boş bırakınca middleware dev-bypass'a düşer (ADR-0008).
    c.env("NEXT_PUBLIC_SUPABASE_URL", "").env("NEXT_PUBLIC_SUPABASE_ANON_KEY", "");
    c
}

/// Dizinleri hazırla + core + console spawn (süreç-grubu pid'leri döner; teardown için).
/// Ollama/model burada İNMEZ — on-demand (modül "Kur" → core bootstrap). Konsol :3100 hazır → `true`.
pub fn start(bundle: &Bundle) -> (Vec<i32>, bool) {
    let mut pids = Vec::new();
    provision_dirs();

    match spawn_grouped(core_command(bundle)) {
        Ok(c) => {
            pids.push(c.id() as i32);
            std::mem::forget(c);
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
