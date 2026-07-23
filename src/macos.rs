use std::path::Path;
use std::process::Command;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSWindowCollectionBehavior};
use objc2_service_management::{SMAppService, SMAppServiceStatus};
use winit::event_loop::EventLoopBuilder;
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

// ---- Startup ----

pub fn set_accessory_activation_policy<T>(builder: &mut EventLoopBuilder<T>) {
    builder.with_activation_policy(ActivationPolicy::Accessory);
}

pub fn show_on_all_spaces() {
    let Some(app) = ns_app() else { return };
    for window in app.windows().iter() {
        let behavior = window.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary;
        window.setCollectionBehavior(behavior);
    }
}

// ---- Visibility ----

pub fn activate_app() {
    let Some(app) = ns_app() else { return };
    app.unhide(None);

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

fn ns_app() -> Option<Retained<NSApplication>> {
    MainThreadMarker::new().map(NSApplication::sharedApplication)
}

// ---- Launching ----

pub fn open(path: &Path) -> std::io::Result<()> {
    Command::new("open").arg(path).spawn().map(|_| ())
}

// ---- Login item ----

pub fn starts_at_login() -> bool {
    let status = unsafe { login_service().status() };
    status == SMAppServiceStatus::Enabled
}

pub fn set_start_at_login(enable: bool) -> bool {
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

fn login_service() -> Retained<SMAppService> {
    unsafe { SMAppService::mainAppService() }
}
