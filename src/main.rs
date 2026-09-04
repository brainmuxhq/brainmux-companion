//! brainmux Companion — tiny Rust bootstrapper (ADR-0011).
//! Job (4 parts): 1) filesystem provisioning · 2) manifest & state check · 3) auto-provisioning
//! (on-demand download) · 4) process management (run the local core, expose 127.0.0.1).
//! Heavy runtime (portable Python + core + model) is fetched/managed in ~/.brainmux, NOT bundled.
//! Like Cursor/VSCode/Ollama: light native launcher, heavy workload in a managed local dir.
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

const OLLAMA: &str = "http://127.0.0.1:11434";
const CORE_ADDR: &str = "127.0.0.1:8787";
const CONSOLE_ADDR: &str = "127.0.0.1:3100";
// Modüller — app kurulunca buraya düşer (guided home; ADR-0013). Motor bağlıysa katalog, değilse onboarding.
const CONSOLE_URL: &str = "http://127.0.0.1:3100/console/moduller";
const MODEL: &str = "bge-m3";

fn log(m: &str) {
    println!("[brainmux] {m}");
}

fn notify(title: &str, body: &str) {
    // Best-effort masaüstü bildirimi — AppImage terminalsiz çalışır, user görsel geri bildirim alsın.
    // notify-send yoksa sessiz geç (çalışmayı bloklamaz).
    let _ = Command::new("notify-send")
        .args(["-a", "brainmux", "-i", "brainmux", title, body])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn home() -> PathBuf {
    match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => {
            log("UYARI: HOME tanımlı değil → /tmp kullanılıyor");
            PathBuf::from("/tmp")
        }
    }
}
fn bmux_home() -> PathBuf {
    std::env::var("BRAINMUX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join(".brainmux"))
}
fn repo() -> PathBuf {
    std::env::var("BRAINMUX_REPO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join("Development/Projects/brainmux/brainmux"))
}

// ---- Part 1: FileSystem Provisioning ----
fn provision_dirs() {
    log("1/4 dizinler hazırlanıyor…");
    let base = bmux_home();
    for sub in ["", "logs", "models", "knowledge", "output", "runtime"] {
        let _ = fs::create_dir_all(base.join(sub));
    }
    log(&format!("  ~/.brainmux hazır: {}", base.display()));
}

// ---- Part 2: Manifest & State Check ----
// v1: local Ollama/model state. TODO(fill): CDN manifest.json + SHA256 for portable-python / core-zip / model.
fn state_check() {
    log("2/4 durum denetimi…");
    log(&format!("  ollama: {}", if ollama_up() { "çalışıyor" } else { "kapalı → kurulacak" }));
    log(&format!("  model {MODEL}: {}", if model_present() { "var" } else { "yok → inecek" }));
}

// ---- Part 3: Auto-Provisioning (on-demand) ----
fn ollama_up() -> bool {
    ureq::get(&format!("{OLLAMA}/api/version"))
        .timeout(Duration::from_secs(2))
        .call()
        .is_ok()
}
fn model_present() -> bool {
    match ureq::get(&format!("{OLLAMA}/api/tags")).timeout(Duration::from_secs(3)).call() {
        Ok(r) => r
            .into_json::<serde_json::Value>()
            .ok()
            .and_then(|v| {
                v["models"].as_array().map(|ms| {
                    ms.iter().any(|m| {
                        m["name"]
                            .as_str()
                            .map_or(false, |n| n == MODEL || n.split(':').next() == Some(MODEL))
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
    log("  Ollama başlatılıyor…");
    // TODO(fill): binary yoksa CDN/ollama.com'dan indir + kur.
    let _ = Command::new("ollama").arg("serve").stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(500));
        if ollama_up() {
            return;
        }
    }
    log("  UYARI: Ollama başlamadı (kurulu mu?)");
}
fn ensure_model() {
    if model_present() {
        return;
    }
    log(&format!("  {MODEL} indiriliyor (ilk sefer, ~1GB)…"));
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
fn auto_provision() {
    log("3/4 eksikler kuruluyor (zero-state)…");
    ensure_ollama();
    ensure_model();
    // TODO(fill): portable-python + core-zip'i CDN'den (% + web-sync) çek → ~/.brainmux/runtime.
    log("  hazır ✓");
}

// ---- Part 4: Process Management ----
fn spawn_grouped(mut cmd: Command) -> std::io::Result<Child> {
    // Own session/process-group → Ctrl+C teardown kills the whole tree (npm→node) = no orphans.
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    cmd.spawn()
}
fn kill_group(child: &Child) {
    let pid = child.id() as i32;
    unsafe {
        libc::kill(-pid, libc::SIGTERM);
    }
}
fn wait_port(addr: &str, name: &str) {
    for _ in 0..120 {
        if std::net::TcpStream::connect(addr).is_ok() {
            return;
        }
        thread::sleep(Duration::from_secs(1));
    }
    log(&format!("  UYARI: {name} açılmadı ({addr})"));
}

fn main() {
    log("brainmux Companion (Rust bootstrapper) başlıyor…");

    // Tek örnek: çekirdek zaten açıksa (kullanıcı ikona ikinci kez tıkladı) yeni kopya açma —
    // sadece konsolu öne getir. İki paralel çekirdek = port çakışması + kafa karışıklığı.
    if std::net::TcpStream::connect(CORE_ADDR).is_ok() {
        log("çekirdek zaten çalışıyor → konsol açılıyor.");
        notify("brainmux çalışıyor", "Konsol açılıyor…");
        let _ = Command::new("xdg-open").arg(CONSOLE_URL).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
        return;
    }
    notify("brainmux başlatılıyor", "Yerel motor hazırlanıyor…");

    provision_dirs(); // 1
    state_check(); // 2
    auto_provision(); // 3

    // 4: run the local core (dev: local repo via uv; prod fill: portable-python from ~/.brainmux/runtime)
    let repo = repo();
    log("4/4 çekirdek başlatılıyor (127.0.0.1:8787)…");
    let mut core_cmd = Command::new("uv");
    core_cmd
        .args([
            "run", "--project", "apps/core", "--env-file", ".env", "--", "uvicorn",
            "brainmux_core.api.app:app", "--host", "127.0.0.1", "--port", "8787", "--app-dir",
            "apps/core/src",
        ])
        .current_dir(&repo);
    let mut core = match spawn_grouped(core_cmd) {
        Ok(c) => c,
        Err(e) => {
            log(&format!("HATA: çekirdek başlatılamadı ({e}) — uv kurulu mu?"));
            std::process::exit(1);
        }
    };
    wait_port(CORE_ADDR, "çekirdek");
    log("  çekirdek yayında: 127.0.0.1:8787 ✓");

    // Console = web (ADR-0011). Dev convenience: start the local console; prod = hosted app.brainmux.com.
    let mut con_cmd = Command::new("npm");
    con_cmd
        .args(["--prefix", "apps/app", "run", "dev", "--", "-p", "3100"])
        .current_dir(&repo)
        .env("NEXT_PUBLIC_CORE_URL", "http://127.0.0.1:8787");
    let console = match spawn_grouped(con_cmd) {
        Ok(c) => Some(c),
        Err(e) => {
            log(&format!("UYARI: konsol başlatılamadı ({e}) — npm/Node var mı?"));
            None
        }
    };
    if console.is_some() {
        wait_port(CONSOLE_ADDR, "konsol");
    }

    log(&format!("hazır → {CONSOLE_URL}"));
    notify("brainmux hazır", "Konsol açılıyor — modülleri kullanabilirsiniz.");
    let _ = Command::new("xdg-open").arg(CONSOLE_URL).stdout(Stdio::null()).stderr(Stdio::null()).spawn();

    // Quit paths: (a) web konsol "Lokal Modu Durdur" → core POST /shutdown → core çıkar → wait döner;
    // (b) Ctrl+C (terminalden) → handler core grubunu öldürür → wait döner. Headless (AppImage) → (a).
    let core_pid = core.id() as i32;
    let _ = ctrlc::set_handler(move || unsafe {
        libc::kill(-core_pid, libc::SIGTERM);
    });
    log("çalışıyor. Konsoldan 'Lokal Modu Durdur' ya da Ctrl+C ile durdur.");
    let _ = core.wait(); // çekirdek çıkana dek bloklar (web shutdown veya Ctrl+C)

    log("çekirdek durdu — kapatılıyor…");
    if let Some(c) = &console {
        kill_group(c);
    }
    kill_group(&core);
}
