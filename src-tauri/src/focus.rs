//! Best-effort focus detection: is the system-level focused UI element an
//! editable text field?
//!
//! `None` = indeterminate (API unavailable, missing permission, unsupported
//! platform). Detection has false negatives (real fields invisible to the
//! accessibility API): it must NEVER block dictation, only trigger a warning
//! and the clipboard fallback.

#[cfg(target_os = "macos")]
pub fn editable_focused() -> Option<bool> {
    use accessibility_sys::{
        kAXErrorSuccess, AXUIElementCopyAttributeValue, AXUIElementCreateSystemWide,
        AXUIElementIsAttributeSettable, AXUIElementRef,
    };
    use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};

    unsafe {
        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            return None;
        }
        let attr = CFString::from_static_string("AXFocusedUIElement");
        let mut focused: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(system, attr.as_concrete_TypeRef(), &mut focused);
        CFRelease(system as CFTypeRef);
        if err != kAXErrorSuccess || focused.is_null() {
            // Missing permission or no identifiable element: indeterminate.
            return None;
        }
        let focused = focused as AXUIElementRef;

        // 1. Standard text roles.
        let role_attr = CFString::from_static_string("AXRole");
        let mut role_ref: CFTypeRef = std::ptr::null();
        let err =
            AXUIElementCopyAttributeValue(focused, role_attr.as_concrete_TypeRef(), &mut role_ref);
        let mut editable = false;
        if err == kAXErrorSuccess && !role_ref.is_null() {
            let role = CFString::wrap_under_create_rule(role_ref as CFStringRef).to_string();
            editable = matches!(
                role.as_str(),
                "AXTextField" | "AXTextArea" | "AXSearchField" | "AXComboBox"
            );
        }

        // 2. Fallback: many web/custom views expose a settable AXValue.
        if !editable {
            let value_attr = CFString::from_static_string("AXValue");
            let mut settable: u8 = 0;
            let err = AXUIElementIsAttributeSettable(
                focused,
                value_attr.as_concrete_TypeRef(),
                &mut settable,
            );
            editable = err == kAXErrorSuccess && settable != 0;
        }

        CFRelease(focused as CFTypeRef);
        Some(editable)
    }
}

#[cfg(target_os = "windows")]
pub fn editable_focused() -> Option<bool> {
    use uiautomation::controls::ControlType;
    use uiautomation::UIAutomation;

    let automation = UIAutomation::new().ok()?;
    let element = automation.get_focused_element().ok()?;
    let control_type = element.get_control_type().ok()?;
    Some(matches!(
        control_type,
        ControlType::Edit | ControlType::Document | ControlType::ComboBox
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn editable_focused() -> Option<bool> {
    // AT-SPI is too inconsistent across environments: indeterminate.
    None
}
