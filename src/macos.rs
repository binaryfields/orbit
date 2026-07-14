use std::path::Path;
use std::process::Command;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationOptions, NSAutoresizingMaskOptions, NSRunningApplication,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
    NSWindowCollectionBehavior, NSWindowOrderingMode,
};
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

/// Let the window follow the user across Spaces (and over full-screen apps)
/// instead of yanking them back. Run before the tray item exists — it
/// applies to every window the app owns.
pub fn join_all_spaces() {
    let Some(app) = ns_app() else { return };
    for window in app.windows().iter() {
        let behavior = window.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary;
        window.setCollectionBehavior(behavior);
    }
}

/// Slide a blurred NSVisualEffectView behind the transparent egui content so
/// the panel shows real vibrancy instead of faking translucency with a flat
/// fill. Run before the tray item exists — it applies to every window the
/// app owns.
pub fn add_vibrancy() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    for window in app.windows().iter() {
        let Some(content) = window.contentView() else {
            continue;
        };
        let Some(frame_view) = (unsafe { content.superview() }) else {
            continue;
        };
        let effect = NSVisualEffectView::new(mtm);
        effect.setFrame(content.frame());
        effect.setMaterial(NSVisualEffectMaterial::HUDWindow);
        effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        effect.setState(NSVisualEffectState::Active);
        effect.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        effect.setWantsLayer(true);
        if let Some(layer) = effect.layer() {
            layer.setCornerRadius(f64::from(crate::PANEL_CORNER_RADIUS));
            layer.setMasksToBounds(true);
        }
        frame_view.addSubview_positioned_relativeTo(
            &effect,
            NSWindowOrderingMode::Below,
            Some(&content),
        );
    }
}

pub fn open(path: &Path) -> std::io::Result<()> {
    Command::new("open").arg(path).spawn().map(|_| ())
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
