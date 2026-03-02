use tauri::WebviewWindow;

#[tauri::command]
pub fn set_titlebar_opacity(window: WebviewWindow, opacity: f64) {
    #[cfg(target_os = "macos")]
    {
        let _ = window.with_webview(move |webview| unsafe {
            use objc2_app_kit::{NSView, NSWindow, NSWindowButton};

            let ns_window_ptr = webview.ns_window() as *const NSWindow;
            if ns_window_ptr.is_null() {
                return;
            }
            let ns_window = &*ns_window_ptr;

            // The traffic light buttons live inside a container view:
            // button -> superview (NSTitlebarButtonsContainerView) -> superview (NSTitlebarContainerView)
            // Setting alpha on the container hides/shows all traffic lights together.
            if let Some(close_button) =
                ns_window.standardWindowButton(NSWindowButton::CloseButton)
            {
                let button_view: &NSView = &close_button;
                if let Some(container) = button_view.superview() {
                    if let Some(titlebar_container) = container.superview() {
                        titlebar_container.setAlphaValue(opacity);
                    }
                }
            }
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, opacity);
    }
}
