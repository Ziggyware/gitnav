mod app;
mod porcelain;
mod scan;
mod ui;

use app::App;
use ratatui::{
    Terminal,
    crossterm::{
        event::{self, Event, KeyCode::*, KeyEventKind, KeyModifiers},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::CrosstermBackend,
};
use scan::Msg;
use std::{
    io::{self, Stderr},
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

type Term = Terminal<CrosstermBackend<Stderr>>;

fn main() -> io::Result<()> {
    let root = std::path::absolute(
        std::env::args()
            .nth(1)
            .map_or(std::env::current_dir()?, PathBuf::from),
    )?;
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("not a directory: {}", root.display()),
        ));
    }
    let (tx, rx) = mpsc::channel();
    scan::spawn_scan(root.clone(), tx.clone());

    // TUI renders on stderr so stdout carries only the chosen path: `cd "$(gitnav ~/dev)"`.
    enable_raw_mode()?;
    execute!(io::stderr(), EnterAlternateScreen)?;
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |p| {
        restore();
        hook(p)
    }));
    let res = Terminal::new(CrosstermBackend::new(io::stderr()))
        .and_then(|mut t| run(&mut t, App::new(root), rx, tx));
    restore();
    if let Some(p) = res? {
        println!("{}", p.display());
    }
    Ok(())
}

fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stderr(), LeaveAlternateScreen);
}

fn run(
    t: &mut Term,
    mut app: App,
    rx: Receiver<Msg>,
    tx: Sender<Msg>,
) -> io::Result<Option<PathBuf>> {
    loop {
        for m in rx.try_iter() {
            match m {
                Msg::Found(rels) => {
                    app.scanning = false;
                    app.repos.extend(rels.into_iter().map(|r| (r, None)));
                }
                Msg::Status(rel, s) => _ = app.repos.insert(rel, Some(s)),
            }
        }
        t.draw(|f| app.draw(f))?;
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let Event::Key(k) = event::read()? else {
            continue;
        };
        if k.kind != KeyEventKind::Press {
            continue;
        }
        match k.code {
            Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => return Ok(None),
            Char('q') | Esc => return Ok(None),
            Char('j') | Down => app.step(1),
            Char('k') | Up => app.step(-1),
            PageDown => app.step(10),
            PageUp => app.step(-10),
            Enter | Right | Char('l') => {
                if let Some(p) = app.enter() {
                    return Ok(Some(p));
                }
            }
            Backspace | Left | Char('h') => app.up(),
            Char('c') => return Ok(app.selected().map(|e| app.abs(&e.rel))),
            Char('f') => app.refresh(true, &tx),
            Char('r') => app.refresh(false, &tx),
            Char('d') => app.only_drift ^= true,
            _ => {}
        }
    }
}
