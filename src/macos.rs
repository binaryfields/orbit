use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSApplicationActivationOptions, NSRunningApplication};
use objc2_service_management::{SMAppService, SMAppServiceStatus};
use winit::event_loop::EventLoopBuilder;
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

fn ns_app() -> Option<Retained<NSApplication>> {
    MainThreadMarker::new().map(NSApplication::sharedApplication)
}

/// Force Orbit to the foreground for keyboard focus. Accessory apps aren't
/// activated implicitly and macOS 14+ ignores polite requests, so pile on
/// every mechanism. Safe while hidden — per-window calls skip hidden windows.
pub fn activate_app() {
    let Some(app) = ns_app() else { return };
    app.unhide(None);

    #[allow(deprecated)]
    NSRunningApplication::currentApplication().activateWithOptions(
        NSApplicationActivationOptions::ActivateIgnoringOtherApps
            | NSApplicationActivationOptions::ActivateAllWindows,
    );
    // Deprecated since macOS 14 but still effective from hotkey contexts.
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);

    for window in app.windows().iter() {
        if window.isVisible() {
            window.makeKeyAndOrderFront(None);
            window.orderFrontRegardless();
        }
    }
}

pub fn hide_app() {
    let Some(app) = ns_app() else { return };
    app.hide(None);
}

pub fn set_accessory_activation_policy<T>(builder: &mut EventLoopBuilder<T>) {
    builder.with_activation_policy(ActivationPolicy::Accessory);
}

fn login_service() -> Retained<SMAppService> {
    unsafe { SMAppService::mainAppService() }
}

pub fn login_enabled() -> bool {
    let status = unsafe { login_service().status() };
    status == SMAppServiceStatus::Enabled
}

pub fn set_login(enable: bool) -> bool {
    let service = login_service();
    let result = unsafe {
        if enable {
            service.registerAndReturnError()
        } else {
            service.unregisterAndReturnError()
        }
    };
    if let Err(err) = result {
        let verb = if enable { "enable" } else { "disable" };
        eprintln!("orbit: could not {verb} start-at-login: {err}");
    }

    let status = unsafe { service.status() };
    if enable && status == SMAppServiceStatus::RequiresApproval {
        unsafe { SMAppService::openSystemSettingsLoginItems() };
    }
    status == SMAppServiceStatus::Enabled
}
