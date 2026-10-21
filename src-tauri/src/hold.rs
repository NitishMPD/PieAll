//! Hold-to-open for the plain backtick key, anywhere in Windows.
//!
//! A low-level keyboard hook holds back each unmodified ` press and reports
//! `Hold::Start` at once, so the ring appears on key-down. Releasing within
//! `HOLD_MS`, or typing another key first, reports `Hold::Tap` and replays the
//! backtick, so typing is unchanged. Holding past `HOLD_MS` reports
//! `Hold::Confirm`; the release after that reports `Hold::End`. Modified presses
//! (Shift for ~, Ctrl+`) and injected input pass through untouched.
//!
//! Over apps where ` is a menu key rather than text (the `instant` predicate,
//! e.g. Figma or Explorer), the press is confirmed immediately with no tap
//! detection, like AutoHotPie: there a plain ` is never typed.

use pieall::platform::INJECTED_MARKER;
use std::{
    cell::Cell,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc, OnceLock,
    },
};
use windows_sys::Win32::{
    Foundation::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

pub enum Hold {
    /// ` went down: show the ring now.
    Start,
    /// It is a hold, not a typed backtick: safe to read the selection.
    Confirm,
    /// It was typing after all: drop the ring; the backtick is replayed.
    Tap,
    End,
    /// Escape while held: close without running anything.
    Cancel,
}

const HOLD_MS: u32 = 150;

/// Virtual-key code of the hold key; the backtick unless the user picked another.
static TRIGGER: AtomicU32 = AtomicU32::new(VK_OEM_3 as u32);
static ENABLED: AtomicBool = AtomicBool::new(true);

pub fn enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::SeqCst);
}

/// Changes the hold key. Only keys that have no modifier-only meaning are accepted.
pub fn set_trigger(vk: u32) -> Result<(), String> {
    if !matches!(vk, 0xC0 | 0x14 | 0x13 | 0x91 | 0xE2 | 0x70..=0x7B) {
        return Err("That key cannot open the ring.".into());
    }
    TRIGGER.store(vk, Ordering::SeqCst);
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Idle,
    /// ` is down and held back with the ring showing; a timer decides tap vs. hold.
    Pending,
    /// The ring is open; the release will be reported.
    Holding,
    /// The press became ordinary typing; let its release through.
    Passthrough,
}

static EVENTS: OnceLock<mpsc::Sender<Hold>> = OnceLock::new();
type Instant = Box<dyn Fn() -> bool + Send + Sync>;
static INSTANT: OnceLock<Instant> = OnceLock::new();
thread_local! {
    static STATE: Cell<State> = const { Cell::new(State::Idle) };
    static TIMER: Cell<usize> = const { Cell::new(0) };
    static SCAN: Cell<u16> = const { Cell::new(0) };
}

/// Installs the hook on its own message-loop thread. Events are delivered in order
/// on a separate thread, so slow handlers never stall system keyboard input.
pub fn install(
    handler: impl Fn(Hold) + Send + 'static,
    instant: impl Fn() -> bool + Send + Sync + 'static,
) -> Result<(), String> {
    let _ = INSTANT.set(Box::new(instant));
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
        emit(Hold::Confirm);
    }
}

/// Returns true when the event must be swallowed.
fn handle(k: &KBDLLHOOKSTRUCT, down: bool) -> bool {
    let trigger = TRIGGER.load(Ordering::SeqCst) as u16;
    let backtick = k.vkCode == trigger as u32;
    let scan = SCAN.get();
    match STATE.get() {
        State::Idle => {
            if !(enabled() && backtick && down) || modifiers_down() {
                return false;
            }
            SCAN.set(k.scanCode as u16);
            emit(Hold::Start);
            if INSTANT.get().is_some_and(|instant| instant()) {
                STATE.set(State::Holding);
                emit(Hold::Confirm);
                return true;
            }
            STATE.set(State::Pending);
            TIMER.set(unsafe { SetTimer(std::ptr::null_mut(), 0, HOLD_MS, None) });
            true
        }
        State::Pending if backtick => {
            if !down {
                // A quick tap: type the backtick after all.
                stop_timer();
                STATE.set(State::Idle);
                emit(Hold::Tap);
                send(&[key(trigger, scan, 0), key(trigger, scan, KEYEVENTF_KEYUP)]);
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
            emit(Hold::Tap);
            let extended = if k.flags & LLKHF_EXTENDED != 0 {
                KEYEVENTF_EXTENDEDKEY
            } else {
                0
            };
            send(&[
                key(trigger, scan, 0),
                key(k.vkCode as u16, k.scanCode as u16, extended),
            ]);
            true
        }
        State::Holding => {
            if backtick && !down {
                STATE.set(State::Idle);
                emit(Hold::End);
            }
            // The ring may not have focus, so Escape is taken here instead of
            // reaching the app underneath. Other keys pass through.
            if k.vkCode == VK_ESCAPE as u32 {
                if down {
                    emit(Hold::Cancel);
                }
                return true;
            }
            // Let Windows clear async key state on physical release, even after a swallowed down.
            backtick && down
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_shortcut_passes_trigger_and_escape_through() {
        set_enabled(false);
        for vk in [TRIGGER.load(Ordering::SeqCst), VK_ESCAPE as u32] {
            let event = KBDLLHOOKSTRUCT {
                vkCode: vk,
                ..unsafe { std::mem::zeroed() }
            };
            assert!(!handle(&event, true));
            assert!(!handle(&event, false));
            assert!(STATE.get() == State::Idle);
        }
        // Disabling must still end an intercepted hold and let Windows see its release.
        STATE.set(State::Holding);
        let release = KBDLLHOOKSTRUCT {
            vkCode: TRIGGER.load(Ordering::SeqCst),
            ..unsafe { std::mem::zeroed() }
        };
        assert!(!handle(&release, false));
        assert!(STATE.get() == State::Idle);
        set_enabled(true);
        assert!(enabled());
    }

    #[test]
    fn held_trigger_blocks_repeats_but_passes_physical_release() {
        let event = KBDLLHOOKSTRUCT {
            vkCode: TRIGGER.load(Ordering::SeqCst),
            ..unsafe { std::mem::zeroed() }
        };
        STATE.set(State::Holding);
        assert!(handle(&event, true));
        assert!(STATE.get() == State::Holding);
        assert!(!handle(&event, false));
        assert!(STATE.get() == State::Idle);
    }
}
