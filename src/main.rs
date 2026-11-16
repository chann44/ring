use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame, Terminal,
};
use reqwest::Client;
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

#[derive(Clone, PartialEq)]
enum TestPhase {
    Idle,
    Ping,
    Download,
    Upload,
    Done,
    Error(String),
}

enum SpeedEvent {
    Phase(TestPhase),
    PingResult(f64),
    DownloadTick { progress: u16, mbps: f64 },
    UploadTick { progress: u16, mbps: f64 },
}

struct App {
    phase: TestPhase,
    ping_ms: Option<f64>,
    down_mbps: f64,
    up_mbps: f64,
    down_progress: u16,
    up_progress: u16,
    down_history: Vec<u64>,
    up_history: Vec<u64>,
}

impl App {
    fn new() -> Self {
        Self {
            phase: TestPhase::Idle,
            ping_ms: None,
            down_mbps: 0.0,
            up_mbps: 0.0,
            down_progress: 0,
            up_progress: 0,
            down_history: vec![0; 60],
            up_history: vec![0; 60],
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal).await;

    // Always restore terminal state even if an error occurs
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
    }
    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    let (tx, mut rx) = mpsc::unbounded_channel::<SpeedEvent>();

    tokio::spawn(run_speed_test(tx.clone()));

    loop {
        // Drain any pending updates from the background network task
        while let Ok(ev) = rx.try_recv() {
            match ev {
                SpeedEvent::Phase(p) => app.phase = p,
                SpeedEvent::PingResult(ms) => app.ping_ms = Some(ms),
                SpeedEvent::DownloadTick { progress, mbps } => {
                    app.down_progress = progress;
                    app.down_mbps = mbps;
                    app.down_history.push(mbps.round() as u64);
                    if app.down_history.len() > 60 {
                        app.down_history.remove(0);
                    }
                }
                SpeedEvent::UploadTick { progress, mbps } => {
                    app.up_progress = progress;
                    app.up_mbps = mbps;
                    app.up_history.push(mbps.round() as u64);
                    if app.up_history.len() > 60 {
                        app.up_history.remove(0);
                    }
                }
            }
        }

        terminal.draw(|f| ui(f, &app))?;

        if event::poll(Duration::from_millis(30))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char('r') => {
                            if matches!(app.phase, TestPhase::Done | TestPhase::Error(_)) {
                                app.reset();
                                tokio::spawn(run_speed_test(tx.clone()));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

async fn run_speed_test(tx: mpsc::UnboundedSender<SpeedEvent>) {
    let client = Client::new();

    let _ = tx.send(SpeedEvent::Phase(TestPhase::Ping));
    let ping_start = Instant::now();
    if let Err(e) = client.get("https://speed.cloudflare.com/__down?bytes=0").send().await {
        let _ = tx.send(SpeedEvent::Phase(TestPhase::Error(e.to_string())));
        return;
    }
    let ping_ms = ping_start.elapsed().as_secs_f64() * 1000.0;
    let _ = tx.send(SpeedEvent::PingResult(ping_ms));

    let _ = tx.send(SpeedEvent::Phase(TestPhase::Download));
    let total_down_bytes: u64 = 25_000_000;
    let down_url = format!("https://speed.cloudflare.com/__down?bytes={}", total_down_bytes);

    let start_down = Instant::now();
    let mut last_tick = Instant::now();
    let mut downloaded: u64 = 0;

    match client.get(&down_url).send().await {
        Ok(mut resp) => {
            while let Ok(Some(chunk)) = resp.chunk().await {
                downloaded += chunk.len() as u64;
                if last_tick.elapsed() >= Duration::from_millis(120) {
                    let secs = start_down.elapsed().as_secs_f64().max(0.001);
                    let mbps = (downloaded as f64 * 8.0) / (secs * 1_000_000.0);
                    let progress = ((downloaded as f64 / total_down_bytes as f64) * 100.0).min(100.0) as u16;
                    let _ = tx.send(SpeedEvent::DownloadTick { progress, mbps });
                    last_tick = Instant::now();
                }
            }
            let final_secs = start_down.elapsed().as_secs_f64().max(0.001);
            let final_mbps = (downloaded as f64 * 8.0) / (final_secs * 1_000_000.0);
            let _ = tx.send(SpeedEvent::DownloadTick { progress: 100, mbps: final_mbps });
        }
        Err(e) => {
            let _ = tx.send(SpeedEvent::Phase(TestPhase::Error(e.to_string())));
            return;
        }
    }

    // 3. Upload Test (10 MB in 2 MB chunks for live progress updates)
    let _ = tx.send(SpeedEvent::Phase(TestPhase::Upload));
    let total_up_bytes: u64 = 10_000_000;
    let chunk_size: usize = 2_000_000;
    let payload = vec![0u8; chunk_size];
    let mut uploaded: u64 = 0;
    let start_up = Instant::now();

    for _ in 0..(total_up_bytes / chunk_size as u64) {
        match client.post("https://speed.cloudflare.com/__up").body(payload.clone()).send().await {
            Ok(_) => {
                uploaded += chunk_size as u64;
                let secs = start_up.elapsed().as_secs_f64().max(0.001);
                let mbps = (uploaded as f64 * 8.0) / (secs * 1_000_000.0);
                let progress = ((uploaded as f64 / total_up_bytes as f64) * 100.0).min(100.0) as u16;
                let _ = tx.send(SpeedEvent::UploadTick { progress, mbps });
            }
            Err(e) => {
                let _ = tx.send(SpeedEvent::Phase(TestPhase::Error(e.to_string())));
                return;
            }
        }
    }

    let _ = tx.send(SpeedEvent::Phase(TestPhase::Done));
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(5), // Top metric cards
            Constraint::Length(3), // Download gauge
            Constraint::Min(5),    // Download sparkline
            Constraint::Length(3), // Upload gauge
            Constraint::Min(5),    // Upload sparkline
            Constraint::Length(2), // Footer
        ])
        .split(f.area());

    let status_text = match &app.phase {
        TestPhase::Idle => Span::styled(" READY ", Style::default().fg(Color::Black).bg(Color::Gray)),
        TestPhase::Ping => Span::styled(" MEASURING PING... ", Style::default().fg(Color::Black).bg(Color::Yellow)),
        TestPhase::Download => Span::styled(" TESTING DOWNLOAD... ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        TestPhase::Upload => Span::styled(" TESTING UPLOAD... ", Style::default().fg(Color::Black).bg(Color::Magenta)),
        TestPhase::Done => Span::styled(" COMPLETE ", Style::default().fg(Color::Black).bg(Color::Green)),
        TestPhase::Error(e) => Span::styled(format!(" ERROR: {} ", e), Style::default().fg(Color::White).bg(Color::Red)),
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" ⚡ WIFI SPEEDTEST  ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        status_text,
    ]))
    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    f.render_widget(header, chunks[0]);

    let card_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(chunks[1]);

    let ping_str = app.ping_ms.map(|p| format!("{:.1} ms", p)).unwrap_or_else(|| "-- ms".into());
    render_stat_card(f, card_chunks[0], " ⏱ LATENCY ", &ping_str, Color::Yellow);
    render_stat_card(f, card_chunks[1], " ↓ DOWNLOAD ", &format!("{:.2} Mbps", app.down_mbps), Color::Cyan);
    render_stat_card(f, card_chunks[2], " ↑ UPLOAD ", &format!("{:.2} Mbps", app.up_mbps), Color::Magenta);

    let down_gauge = Gauge::default()
        .block(Block::default().title(" Download Progress ").borders(Borders::ALL).border_type(BorderType::Rounded))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
        .percent(app.down_progress);
    f.render_widget(down_gauge, chunks[2]);

}

fn render_stat_card(f: &mut Frame, area: Rect, title: &str, value: &str, color: Color) {
    let card = Paragraph::new(value)
        .alignment(Alignment::Center)
        .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
    f.render_widget(card, area);
}
