mod app;
<<<<<<< HEAD
mod cache;
mod events;
mod fuzzy;
mod git;
mod history;
mod keys;
mod nav;
mod porcelain;
mod scan;
mod theme;
mod ui;
mod ui_bar;
mod ui_detail;
mod ui_folder;
mod ui_list;
mod ui_mosaic;

use app::App;
use keys::Exit;
use ratatui::{
    Terminal,
    crossterm::{
        event::{self, Event, KeyEventKind},
=======
mod porcelain;
mod scan;
mod ui;

use app::App;
use ratatui::{
    Terminal,
    crossterm::{
        event::{self, Event, KeyCode::*, KeyEventKind, KeyModifiers},
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
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
<<<<<<< HEAD
    time::{Duration, Instant},
=======
    time::Duration,
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
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
<<<<<<< HEAD
    let mut app = App::new(root);
    let res = Terminal::new(CrosstermBackend::new(io::stderr()))
        .and_then(|mut t| run(&mut t, &mut app, rx, tx));
    restore();
    cache::save(&app.root, &app.repos, &app.pulses);
=======
    let res = Terminal::new(CrosstermBackend::new(io::stderr()))
        .and_then(|mut t| run(&mut t, App::new(root), rx, tx));
    restore();
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
    if let Some(p) = res? {
        println!("{}", p.display());
    }
    Ok(())
}

fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stderr(), LeaveAlternateScreen);
}

<<<<<<< HEAD
const FRAME: Duration = Duration::from_millis(80);

fn run(
    t: &mut Term,
    app: &mut App,
    rx: Receiver<Msg>,
    tx: Sender<Msg>,
) -> io::Result<Option<PathBuf>> {
    // Draw on change, plus spinner frames only while work is in flight.
    let (mut redraw, mut last) = (true, Instant::now());
    loop {
        for m in rx.try_iter() {
            app.apply(m);
            redraw = true;
        }
        let animating =
            app.scanning || !app.busy.is_empty() || app.repos.values().any(Option::is_none);
        if animating && last.elapsed() >= FRAME {
            app.tick += 1;
            redraw = true;
        }
        if redraw {
            app.want_info(&tx);
            t.draw(|f| app.draw(f))?;
            (redraw, last) = (false, Instant::now());
        }
        if !event::poll(Duration::from_millis(30))? {
            continue;
        }
        redraw = true;
=======
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
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
        let Event::Key(k) = event::read()? else {
            continue;
        };
        if k.kind != KeyEventKind::Press {
            continue;
        }
<<<<<<< HEAD
        match app.key(k, &tx) {
            Some(Exit::Quit) => return Ok(None),
            Some(Exit::Cd(p)) => return Ok(Some(p)),
            None => {}
=======
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
>>>>>>> ce532fa2a485cac25364c698a4d1c4994bba308d
        }
    }
}
