//! Leitura do tempo de inatividade (idle) do sistema no macOS.
//!
//! Usa `CGEventSourceSecondsSinceLastEventType` (CoreGraphics) pra saber há quantos
//! segundos não há input de teclado/mouse. É a MESMA API que o Jbuddy antigo usava —
//! só que agora pra medir a verdade, em vez de falsear atividade.
//!
//! Importante: essa leitura NÃO exige permissão de Acessibilidade (é só leitura do
//! tempo de idle do HID), diferente de *injetar* eventos como o app antigo fazia.

use std::os::raw::c_double;

/// `kCGEventSourceStateHIDSystemState` — estado do HID combinado do sistema.
const HID_SYSTEM_STATE: u32 = 1;
/// `kCGAnyInputEventType` — qualquer tipo de evento de input (definido como `~0`).
const ANY_INPUT_EVENT_TYPE: u32 = 0xFFFF_FFFF;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: u32, event_type: u32) -> c_double;
}

/// Segundos desde o último input de teclado/mouse.
pub fn seconds_since_last_input() -> f64 {
    // SAFETY: chamada FFI a uma função read-only do CoreGraphics; sem ponteiros.
    unsafe { CGEventSourceSecondsSinceLastEventType(HID_SYSTEM_STATE, ANY_INPUT_EVENT_TYPE) }
}
