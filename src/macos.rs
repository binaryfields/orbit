use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSApplication;
use winit::event_loop::EventLoopBuilder;
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

pub fn set_accessory_activation_policy<T>(builder: &mut EventLoopBuilder<T>) {
    builder.with_activation_policy(ActivationPolicy::Accessory);
}

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
