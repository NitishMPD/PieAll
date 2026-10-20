//! Hold-to-open for the plain backtick key, anywhere in Windows.
//!
//! A low-level keyboard hook holds back each unmodified ` press. Releasing it
//! quickly, or typing another key first, replays the backtick so typing is
//! unchanged. Holding it past `HOLD_MS` reports `Hold::Start`, and releasing
//! it afterwards reports `Hold::End`. Modified presses (Shift for ~, Ctrl+`)
//! and injected input pass through untouched.

use pieall::platform::INJECTED_MARKER;
use std::{
    cell::Cell,
    sync::{mpsc, OnceLock},
};
use windows_sys::Win32::{
    Foundation::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

pub enum Hold {
    Start,
    End,
}

const HOLD_MS: u32 = 220;

#[derive(Clone, Copy, PartialEq)]
enum State {
    Idle,
    /// ` is down and held back; a timer decides tap vs. hold.
    Pending,
    /// The ring is open; the release will be reported.
    Holding,
    /// The press became ordinary typing; let its release through.
    Passthrough,
}

static EVENTS: OnceLock<mpsc::Sender<Hold>> = OnceLock::new();
thread_local! {
    static STATE: Cell<State> = const { Cell::new(State::Idle) };
    static TIMER: Cell<usize> = const { Cell::new(0) };
    static SCAN: Cell<u16> = const { Cell::new(0) };
}

/// Installs the hook on its own message-loop thread. Events are delivered in order
/// on a separate thread, so slow handlers never stall system keyboard input.
pub fn install(handler: impl Fn(Hold) + Send + 'static) -> Result<(), String> {
    let (events, inbox) = mpsc::channel();
    EVENTS
        .set(events)
        .map_err(|_| "Hold shortcut already installed.")?;
    std::thread::Builder::new()
        .name("pieall-hold-events".into())
        .spawn(move || {
            for event in inbox {
                handler(event);
            }
        })
        .map_err(|e| e.to_string())?;
    let (ready, installed) = mpsc::channel();
    std::thread::Builder::new()
        .name("pieall-hold-hook".into())
        .spawn(move || unsafe {
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(hook_proc),
                GetModuleHandleW(std::ptr::null()),
                0,
            );
            let _ = ready.send(!hook.is_null());
            if hook.is_null() {
                return;
            }
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                if msg.message == WM_TIMER && msg.hwnd.is_null() {
                    on_timer(msg.wParam);
                    continue;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            UnhookWindowsHookEx(hook);
        })
        .map_err(|e| e.to_string())?;
    match installed.recv() {
        Ok(true) => Ok(()),
        _ => Err("Windows refused the keyboard hook.".into()),
    }
}

fn emit(event: Hold) {
    if let Some(events) = EVENTS.get() {
        let _ = events.send(event);
    }
}

fn modifiers_down() -> bool {
    [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN]
        .iter()
        .any(|&vk| unsafe { GetAsyncKeyState(vk as i32) } < 0)
}

fn key(vk: u16, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECTED_MARKER,
            },
        },
    }
}

fn send(inputs: &[INPUT]) {
    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
    }
}

fn stop_timer() {
    let id = TIMER.replace(0);
    if id != 0 {
        unsafe { KillTimer(std::ptr::null_mut(), id) };
    }
}

fn on_timer(id: usize) {
    if id != TIMER.get() {
        return;
    }
    stop_timer();
    if STATE.get() == State::Pending {
        STATE.set(State::Holding);
        emit(Hold::Start);
    }
}

/// Returns true when the event must be swallowed.
fn handle(k: &KBDLLHOOKSTRUCT, down: bool) -> bool {
    let backtick = k.vkCode == VK_OEM_3 as u32;
    let scan = SCAN.get();
    match STATE.get() {
        State::Idle => {
            if !(backtick && down) || modifiers_down() {
                return false;
            }
            SCAN.set(k.scanCode as u16);
            STATE.set(State::Pending);
            TIMER.set(unsafe { SetTimer(std::ptr::null_mut(), 0, HOLD_MS, None) });
            true
        }
        State::Pending if backtick => {
            if !down {
                // A quick tap: type the backtick after all.
                stop_timer();
                STATE.set(State::Idle);
                send(&[key(VK_OEM_3, scan, 0), key(VK_OEM_3, scan, KEYEVENTF_KEYUP)]);
            }
            true // auto-repeat while undecided
        }
        State::Pending => {
            if !down {
                return false;
            }
            // Fast typing ("`a"): replay ` first, then this key, keeping their order.
            stop_timer();
            STATE.set(State::Passthrough);
            let extended = if k.flags & LLKHF_EXTENDED != 0 {
                KEYEVENTF_EXTENDEDKEY
            } else {
                0
            };
            send(&[
                key(VK_OEM_3, scan, 0),
                key(k.vkCode as u16, k.scanCode as u16, extended),
            ]);
            true
        }
        State::Holding => {
            if backtick && !down {
                STATE.set(State::Idle);
                emit(Hold::End);
            }
            // Other keys reach the ring (Escape, arrows, numbers).
            backtick
        }
        State::Passthrough => {
            if backtick && !down {
                STATE.set(State::Idle);
            }
            false
        }
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let k = &*(lparam as *const KBDLLHOOKSTRUCT);
        let down = matches!(wparam as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        if k.flags & LLKHF_INJECTED == 0 && handle(k, down) {
            return 1;
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}
