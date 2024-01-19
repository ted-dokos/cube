// This hides the console window when launching cube.exe,
// at the cost of suppressing println! statements.
/* #![windows_subsystem = "windows"] */

mod state;
mod texture;

use pollster::block_on;
use state::WebGPUState;
use std::mem::{self};
use windows::Win32::{Foundation::POINT, System::LibraryLoader::GetModuleHandleA};
use windows::{
    core::*,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::ValidateRect,
        UI::WindowsAndMessaging::*,
    },
};

fn main() -> windows::core::Result<()> {
    println!("Hello world!");

    unsafe {
        let hinstance = GetModuleHandleA(None)?;
        let window_class_name = s!("window");
        let wc = WNDCLASSA {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: hinstance.into(),
            lpszClassName: window_class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            cbWndExtra: std::mem::size_of::<*mut WebGPUState>() as i32,
            ..Default::default()
        };

        let atom = RegisterClassA(&wc);
        debug_assert!(atom != 0);

        let window = CreateWindowExA(
            WINDOW_EX_STYLE::default(),
            window_class_name,
            s!("My sample window"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            500,
            500,
            None,
            None,
            hinstance,
            None,
        );

        let state: WebGPUState = block_on(WebGPUState::new(window, hinstance.into()));
        SetWindowLongPtrA(
            window,
            WINDOW_LONG_PTR_INDEX(0),
            &state as *const WebGPUState as isize,
        );
        println!("initial state = {}", &state as *const WebGPUState as isize);

        let mut message = MSG::default();
        while GetMessageA(&mut message, None, 0, 0).into() {
            DispatchMessageA(&message);
        }

        Ok(())
    }
}

extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let getptr = GetWindowLongPtrA(window, WINDOW_LONG_PTR_INDEX(0));
        let state: *mut WebGPUState = getptr as *mut WebGPUState;
        match message {
            WM_PAINT => {
                println!("WM_PAINT");
                println!("state = {}", getptr);
                let _ = (*state).render();
                ValidateRect(window, None);
                // let mut paint = MaybeUninit::<PAINTSTRUCT>::uninit();
                // let device_context = BeginPaint(window, paint.as_mut_ptr());
                // let paint = paint.assume_init();

                // static mut OP : ROP_CODE = WHITENESS;
                // PatBlt(device_context,
                //   paint.rcPaint.left,
                //   paint.rcPaint.top,
                //   paint.rcPaint.right - paint.rcPaint.left,
                //   paint.rcPaint.bottom - paint.rcPaint.top,
                //   OP);
                // if OP == WHITENESS {
                //   OP = BLACKNESS;
                // } else {
                //   OP = WHITENESS;
                // }
                // EndPaint(window, &paint);
                LRESULT(0)
            }
            WM_DESTROY => {
                println!("WM_DESTROY");
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_SIZE => {
                println!("WM_SIZE");
                if !state.is_null() {
                    let mut rect: RECT = mem::zeroed();
                    let _ = GetClientRect(window, &mut rect);
                    (*state).resize(rect);
                }
                LRESULT(0)
            }
            WM_MOUSEACTIVATE => {
                println!("WM_MOUSEACTIVATE");
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                println!("WM_MOUSEMOVE");
                if !state.is_null() {
                    let mut pt: POINT = mem::zeroed();
                    let _ = GetCursorPos(&mut pt);
                    (*state).update_bg_color(&pt);
                }
                LRESULT(0)
            }
            _ => DefWindowProcA(window, message, wparam, lparam),
        }
    }
}
