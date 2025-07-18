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

pub fn show_toast_window(app: &AppHandle) {
    use std::time::Duration;
    use tokio::time::sleep;

    // Close any existing toast window
    if let Some(window) = app.get_webview_window("toast") {
        window.close().ok();
    }

    // Get screen dimensions to position toast at bottom
    let screen_height = f64::from(
        app.get_webview_window("search")
            .unwrap()
            .inner_size()
            .unwrap()
            .height,
    );
    let screen_width = f64::from(
        app.get_webview_window("search")
            .unwrap()
            .inner_size()
            .unwrap()
            .width,
    );
    let toast_height = 100.0;
    let toast_width = 200.0;

    // Position at bottom center of screen
    let x = (screen_width - toast_width) / 2.0 - 150.0;
    let y = screen_height - toast_height - 100.0; // 50px from bottom

    let window =
        WebviewWindowBuilder::new(app, "toast", WebviewUrl::App("static/toast.html".into()))
            .title("Toast")
            .inner_size(toast_width, toast_height)
            .position(x, y)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .transparent(true)
            .focused(true)
            .build()
            .expect("Failed to create toast window");

    // Show the window
    window.show().ok();

    // Auto-hide after 2.5 seconds
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        sleep(Duration::from_millis(2500)).await;
        if let Some(toast_window) = app_handle.get_webview_window("toast") {
            toast_window.close().ok();
        }
    });
}
