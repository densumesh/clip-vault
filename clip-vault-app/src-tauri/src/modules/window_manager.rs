use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn show_search_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("search") {
        window.show().ok();
        window.set_focus().ok();
    } else {
        let _unused_window =
            WebviewWindowBuilder::new(app, "search", WebviewUrl::App("static/index.html".into()))
                .title("Clip Vault Search")
                .inner_size(1000.0, 600.0)
                .min_inner_size(800.0, 500.0)
                .center()
                .resizable(true)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .transparent(true)
                .build()
                .expect("Failed to create search window");
    }
}

pub fn show_settings_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().ok();
        window.set_focus().ok();
    } else {
        let _unused_window = WebviewWindowBuilder::new(
            app,
            "settings",
            WebviewUrl::App("static/settings.html".into()),
        )
        .title("Clip Vault Settings")
        .inner_size(500.0, 600.0)
        .center()
        .resizable(false)
        .decorations(true)
        .always_on_top(false)
        .skip_taskbar(false)
        .build()
        .expect("Failed to create settings window");
    }
}
