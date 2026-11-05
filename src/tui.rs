//! Standalone Rust terminal application; no development server or JavaScript VM.
use crossterm::{
    cursor,
    event::{
        self, DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use nsh_platform_tui::{
    AppOptions, AppView, Input, Modifiers, MouseButton, MouseInput, MouseKind, RunEvent,
};
use ratatui_core::terminal::Terminal;
use ratatui_crossterm::CrosstermBackend;
use std::{io, time::Duration};

struct Session;
impl Drop for Session {
    fn drop(&mut self) {
        restore();
    }
}
fn restore() {
    let _ = execute!(
        io::stdout(),
        DisableMouseCapture,
        DisableBracketedPaste,
        DisableFocusChange,
        LeaveAlternateScreen,
        cursor::Show
    );
    let _ = terminal::disable_raw_mode();
}

fn main() -> anyhow::Result<()> {
    if std::env::args()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V")
    {
        println!("omasend-tui {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    omasend::diagnostics::init();
    let executable = std::env::current_exe()?;
    {
        terminal::enable_raw_mode()?;
        let _session = Session;
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            restore();
            previous(panic);
        }));
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste,
            EnableFocusChange,
            cursor::Hide
        )?;
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let mut options = AppOptions::default();
        options.limits = nsh_platform_tui::Limits::standalone();
        options.data_dir = dirs::data_local_dir();
        let mut view = AppView::new(options);
        terminal.draw(|frame| view.render(frame, frame.area()))?;
        view.load(omasend::app_root())?;
        nsh_platform_tui::run_app(&mut terminal, &mut view, poll)?;
    }
    if omasend::updates::RESTART_REQUESTED.swap(false, std::sync::atomic::Ordering::SeqCst) {
        omasend::updates::install::restart(&executable)?;
    }
    Ok(())
}

fn poll(timeout: Duration) -> io::Result<RunEvent> {
    if event::poll(timeout)? {
        Ok(convert(event::read()?))
    } else {
        Ok(RunEvent::Idle)
    }
}
fn modifiers(keys: event::KeyModifiers) -> Modifiers {
    Modifiers {
        ctrl: keys.contains(event::KeyModifiers::CONTROL),
        alt: keys.contains(event::KeyModifiers::ALT),
        shift: keys.contains(event::KeyModifiers::SHIFT),
        cmd: keys.contains(event::KeyModifiers::SUPER),
    }
}
fn button(button: event::MouseButton) -> MouseButton {
    match button {
        event::MouseButton::Left => MouseButton::Left,
        event::MouseButton::Right => MouseButton::Right,
        event::MouseButton::Middle => MouseButton::Middle,
    }
}
fn convert(event: event::Event) -> RunEvent {
    match event {
        event::Event::Resize(..) => RunEvent::Redraw,
        event::Event::FocusGained => RunEvent::Input(Input::Focus(true)),
        event::Event::FocusLost => RunEvent::Input(Input::Focus(false)),
        event::Event::Paste(text) => RunEvent::Input(Input::Paste(text)),
        event::Event::Mouse(mouse) => {
            let mut input = MouseInput::default();
            input.column = mouse.column;
            input.row = mouse.row;
            input.modifiers = modifiers(mouse.modifiers);
            input.kind = match mouse.kind {
                event::MouseEventKind::Down(b) => MouseKind::Down(button(b)),
                event::MouseEventKind::Up(b) => MouseKind::Up(button(b)),
                event::MouseEventKind::Moved | event::MouseEventKind::Drag(_) => MouseKind::Move,
                event::MouseEventKind::ScrollUp => MouseKind::Wheel { x: 0, y: -1 },
                event::MouseEventKind::ScrollDown => MouseKind::Wheel { x: 0, y: 1 },
                event::MouseEventKind::ScrollLeft => MouseKind::Wheel { x: -1, y: 0 },
                event::MouseEventKind::ScrollRight => MouseKind::Wheel { x: 1, y: 0 },
            };
            RunEvent::Input(Input::Mouse(input))
        }
        event::Event::Key(key) => {
            let name = match key.code {
                event::KeyCode::Char(c) => c.to_string(),
                event::KeyCode::Esc => "Escape".into(),
                event::KeyCode::BackTab => "Tab".into(),
                event::KeyCode::F(number) => format!("F{number}"),
                other => format!("{other:?}"),
            };
            let mut held = modifiers(key.modifiers);
            if key.code == event::KeyCode::BackTab {
                held.shift = true;
            }
            if key.kind == event::KeyEventKind::Repeat {
                RunEvent::Input(Input::KeyRepeat {
                    key: name,
                    modifiers: held,
                })
            } else {
                RunEvent::Input(Input::Key {
                    key: name,
                    modifiers: held,
                    release: key.kind == event::KeyEventKind::Release,
                })
            }
        }
    }
}
