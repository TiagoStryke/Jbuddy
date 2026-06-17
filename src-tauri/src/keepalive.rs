//! Keep-alive da sessão — posta um scroll de delta ZERO (sem mover o cursor,
//! sem efeito visível) pra manter a sessão "ativa". Mesmo truque do Jiggy.
//!
//! Recurso PESSOAL, opt-in pela tela de configurações ("impede o notebook de
//! desligar a tela"). Requer permissão de Acessibilidade no macOS pra postar
//! eventos. O [`crate::tracker`] sabe quando foi ELE que postou e desconta esses
//! nudges do cálculo de foco real — então o tracking continua honesto.

use std::os::raw::c_void;

const HID_TAP: u32 = 0; // kCGHIDEventTap
const SCROLL_UNIT_LINE: u32 = 1; // kCGScrollEventUnitLine

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventCreateScrollWheelEvent(
        source: *const c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
    ) -> *const c_void;
    fn CGEventPost(tap: u32, event: *const c_void);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}

/// Posta um scroll de delta zero. No-op visual; só renova a atividade da sessão.
pub fn nudge() {
    // SAFETY: cria um evento de scroll (source nulo é válido), posta e libera.
    unsafe {
        let event = CGEventCreateScrollWheelEvent(std::ptr::null(), SCROLL_UNIT_LINE, 1, 0);
        if !event.is_null() {
            CGEventPost(HID_TAP, event);
            CFRelease(event);
        }
    }
}
