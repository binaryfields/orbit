use std::path::Path;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSAutoresizingMaskOptions, NSRunningApplication, NSVisualEffectBlendingMode,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindowCollectionBehavior,
    NSWindowOrderingMode, NSWorkspace, NSWorkspaceOpenConfiguration,
};
use objc2_foundation::{NSError, NSString, NSURL};
use objc2_service_management::{SMAppService, SMAppServiceStatus};
use winit::event_loop::EventLoopBuilder;
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

// ---- Startup ----

pub fn set_accessory_activation_policy<T>(builder: &mut EventLoopBuilder<T>) {
    builder.with_activation_policy(ActivationPolicy::Accessory);
}

pub fn configure_windows() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    for window in app.windows().iter() {
        window.setCollectionBehavior(
            window.collectionBehavior()
                | NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
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

pub fn open(path: &Path) {
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    let display = path.display().to_string();
    let on_done = RcBlock::new(move |_: *mut NSRunningApplication, err: *mut NSError| {
        if let Some(err) = unsafe { err.as_ref() } {
            let reason = err.localizedDescription();
            eprintln!("orbit: failed to launch {display}: {reason}");
        }
    });
    NSWorkspace::sharedWorkspace().openApplicationAtURL_configuration_completionHandler(
        &url,
        &NSWorkspaceOpenConfiguration::configuration(),
        Some(&on_done),
    );
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
