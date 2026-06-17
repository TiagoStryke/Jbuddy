//! Detecção de "em reunião" — heurística pelo microfone em uso (CoreAudio).
//!
//! Usa `kAudioDevicePropertyDeviceIsRunningSomewhere` no dispositivo de entrada
//! padrão: verdadeiro quando QUALQUER app está captando o mic (Teams/Zoom/Meet
//! em call, gravação, etc.). É API pública — sem necessidade de API privada nem
//! de ler a agenda (que, no caso do Tiago, fica no Teams, não no Calendar).
//!
//! Limite conhecido: uma call só-câmera com mic mudo pode não acusar; na prática
//! o mic costuma estar "rodando" mesmo mutado. Bom o suficiente pra segurar lembrete.

use std::os::raw::c_void;

#[repr(C)]
struct AudioObjectPropertyAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

// Four-char codes do CoreAudio.
const SYSTEM_OBJECT: u32 = 1; // kAudioObjectSystemObject
const DEFAULT_INPUT_DEVICE: u32 = 0x6449_6E20; // 'dIn '
const SCOPE_GLOBAL: u32 = 0x676C_6F62; // 'glob'
const IS_RUNNING_SOMEWHERE: u32 = 0x676F_6E65; // 'gone'
const ELEMENT_MAIN: u32 = 0;

#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    fn AudioObjectGetPropertyData(
        in_object_id: u32,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        io_data_size: *mut u32,
        out_data: *mut c_void,
    ) -> i32;
}

/// `true` se o microfone está sendo usado por algum app (provável reunião).
pub fn microphone_in_use() -> bool {
    unsafe {
        // 1. dispositivo de entrada padrão
        let addr = AudioObjectPropertyAddress {
            selector: DEFAULT_INPUT_DEVICE,
            scope: SCOPE_GLOBAL,
            element: ELEMENT_MAIN,
        };
        let mut device: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let st = AudioObjectGetPropertyData(
            SYSTEM_OBJECT,
            &addr,
            0,
            std::ptr::null(),
            &mut size,
            &mut device as *mut u32 as *mut c_void,
        );
        if st != 0 || device == 0 {
            return false;
        }

        // 2. esse dispositivo está rodando em algum lugar?
        let addr2 = AudioObjectPropertyAddress {
            selector: IS_RUNNING_SOMEWHERE,
            scope: SCOPE_GLOBAL,
            element: ELEMENT_MAIN,
        };
        let mut running: u32 = 0;
        let mut size2 = std::mem::size_of::<u32>() as u32;
        let st2 = AudioObjectGetPropertyData(
            device,
            &addr2,
            0,
            std::ptr::null(),
            &mut size2,
            &mut running as *mut u32 as *mut c_void,
        );
        st2 == 0 && running != 0
    }
}
