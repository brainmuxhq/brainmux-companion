//! brainmux Companion — Tauri desktop shell (ADR-0011 §7). Native pencere açılır (splash), arkada
//! yerel motor + konsol başlatılır, hazır olunca webview konsola (localhost:3100) gider. Çıkışta
//! süreç-grupları öldürülür (orphan yok). İş mantığı core'da (SSoT); kabuk sadece orkestre eder.
mod launcher;

use std::sync::Mutex;
use tauri::Manager;

/// Spawn edilen süreç-grubu pid'leri (çıkışta teardown).
struct Pids(Mutex<Vec<i32>>);

fn navigate_to_console(handle: &tauri::AppHandle) {
    if let Some(win) = handle.get_webview_window("main") {
        if let Ok(url) = launcher::CONSOLE_URL.parse::<tauri::Url>() {
            let _ = win.navigate(url);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
        .manage(Pids(Mutex::new(Vec::new())))
        .setup(|app| {
            let handle = app.handle().clone();
            // Paketlenmiş resource'lar (release) → gömülü çekirdek/modüller/konsol. Yoksa dev fallback (uv-repo).
            let res = handle.path().resource_dir().ok();
            // Array-form resources relative yolu korur → resource_dir/bundle/<...> (staging: src-tauri/bundle).
            let bundle = launcher::Bundle {
                core_python: res.as_ref().map(|r| r.join("bundle/core-bundle/py/bin/python3")).filter(|p| p.exists()),
                modules_dir: res.as_ref().map(|r| r.join("bundle/modules")).filter(|p| p.exists()),
            };
            // Ağır iş (provizyon + core + console) arka planda; pencere splash gösterir, bloklamaz.
            std::thread::spawn(move || {
                // Tek örnek: motor + konsol zaten açıksa yeniden başlatma, sadece konsola git.
                if launcher::port_open(launcher::CORE_ADDR) && launcher::port_open(launcher::CONSOLE_ADDR) {
                    navigate_to_console(&handle);
                    return;
                }
                launcher::notify("brainmux başlatılıyor", "Yerel motor hazırlanıyor…");
                let (pids, ready) = launcher::start(&bundle);
                if let Some(state) = handle.try_state::<Pids>() {
                    *state.0.lock().unwrap() = pids;
                }
                if ready {
                    launcher::notify("brainmux hazır", "Konsol açılıyor…");
                    navigate_to_console(&handle);
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|handle, event| {
            // Uygulama kapanırken çekirdek + konsol süreç-gruplarını öldür (orphan yok).
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = handle.try_state::<Pids>() {
                    let pids = state.0.lock().unwrap().clone();
                    launcher::teardown(&pids);
                }
            }
        });
}
