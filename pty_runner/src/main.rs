//! PTY Runner - Cross-platform terminal state capture
//!
//! Runs a program in a PTY, captures output, and produces hex terminal state.
//! Uses portable-pty for cross-platform PTY and vt100 for terminal emulation.

use anyhow::{Context, Result};
use clap::Parser;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Normalize line endings: ensure all lines end with \r\n (CRLF) for Windows ConPTY
fn normalize_line_endings(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'\r' && i + 1 < data.len() && data[i + 1] == b'\n' {
            // Already CRLF, keep as is
            result.push(b'\r');
            result.push(b'\n');
            i += 2;
        } else if data[i] == b'\n' {
            // LF only, convert to CRLF
            result.push(b'\r');
            result.push(b'\n');
            i += 1;
        } else if data[i] == b'\r' {
            // CR only (rare), keep as is
            result.push(b'\r');
            i += 1;
        } else {
            // Normal character
            result.push(data[i]);
            i += 1;
        }
    }
    result
}

/// Filter out OSC (Operating System Command) sequences
/// OSC sequences start with ESC ] and end with BEL (0x07) or ESC \
/// These are often used for window titles and can differ between platforms
fn filter_osc_sequences(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut i = 0;
    
    while i < data.len() {
        // Check for OSC start: ESC ]
        if i + 1 < data.len() && data[i] == 0x1b && data[i + 1] == b']' {
            // Skip until we find BEL (0x07) or ESC \ (0x1b 0x5c)
            i += 2;
            while i < data.len() {
                if data[i] == 0x07 {
                    // Found BEL terminator
                    i += 1;
                    break;
                } else if i + 1 < data.len() && data[i] == 0x1b && data[i + 1] == b'\\' {
                    // Found ESC \ terminator
                    i += 2;
                    break;
                }
                i += 1;
            }
        } else {
            // Normal character, keep it
            result.push(data[i]);
            i += 1;
        }
    }
    
    result
}

/// PTY Runner for terminal state testing
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to the executable to run
    #[arg(short, long)]
    executable: PathBuf,

    /// Path to keyboard input file (escape sequences sent to PTY)
    #[arg(short, long)]
    keyboard_input: Option<PathBuf>,

    /// Path to stdin file (piped to program's stdin)
    #[arg(short, long)]
    stdin_file: Option<PathBuf>,

    /// Terminal width
    #[arg(long, default_value = "80")]
    cols: u16,

    /// Terminal height
    #[arg(long, default_value = "25")]
    rows: u16,

    /// Output format: "hex", "text", or "raw"
    #[arg(short, long, default_value = "hex")]
    output: String,

    /// Timeout in milliseconds
    #[arg(short, long, default_value = "5000")]
    timeout: u64,

    /// Debug: print raw bytes to stderr
    #[arg(long, default_value = "false")]
    debug_raw: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    eprintln!("Starting PTY runner...");
    eprintln!("Executable: {:?}", args.executable);

    // Create PTY system
    let pty_system = native_pty_system();

    // Create PTY pair with specified size
    let pair = pty_system
        .openpty(PtySize {
            rows: args.rows,
            cols: args.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("Failed to open PTY")?;

    eprintln!("PTY opened successfully");

    // Build command with consistent TERM environment
    let mut cmd = CommandBuilder::new(&args.executable);
    cmd.env("TERM", "xterm"); // Ensure consistent terminal type across platforms

    // Spawn child process in PTY
    let mut child = pair
        .slave
        .spawn_command(cmd)
        .context("Failed to spawn command")?;

    eprintln!("Child process spawned");

    // Get master for I/O
    let master = pair.master;

    // Read keyboard input if provided
    let keyboard_input = if let Some(kb_path) = &args.keyboard_input {
        Some(
            fs::read(kb_path)
                .with_context(|| format!("Failed to read keyboard input: {:?}", kb_path))?,
        )
    } else {
        None
    };

    // Create vt100 parser for terminal emulation
    let mut parser = vt100::Parser::new(args.rows, args.cols, 0);

    // Clone reader for output capture thread
    let mut reader = master
        .try_clone_reader()
        .context("Failed to clone PTY reader")?;

    // Get writer for sending input
    let mut writer = master
        .take_writer()
        .context("Failed to get PTY writer")?;

    // Use a channel to communicate output chunks from the reader thread
    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    // Spawn thread to read output (this thread may block indefinitely on Windows)
    let _output_handle = thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break; // Receiver dropped
                    }
                }
                Err(_) => break, // Error, likely PTY closed
            }
        }
    });

    // Send stdin content if provided
    if let Some(stdin_path) = &args.stdin_file {
        let stdin_content = fs::read(stdin_path)?;
        // Convert LF to CRLF for Windows ConPTY compatibility
        let normalized = normalize_line_endings(&stdin_content);
        writer.write_all(&normalized)?;
    }

    // Small delay to let program start
    thread::sleep(Duration::from_millis(100));

    // Send keyboard input if provided
    if let Some(kb_data) = keyboard_input {
        // Convert LF to CRLF for Windows ConPTY compatibility
        let normalized = normalize_line_endings(&kb_data);
        writer.write_all(&normalized)?;
    }

    // Wait for child with timeout
    let timeout = Duration::from_millis(args.timeout);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                eprintln!("Child process exited");
                break; // Process exited
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    eprintln!("Timeout reached, killing process");
                    // Kill the process
                    let _ = child.kill();
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break,
        }
    }

    // Give more time for any final output and to drain the channel
    thread::sleep(Duration::from_millis(200));

    // Collect all output received so far (with a timeout per chunk)
    let mut output = Vec::new();
    let collect_deadline = std::time::Instant::now() + Duration::from_millis(300);
    while std::time::Instant::now() < collect_deadline {
        match rx.try_recv() {
            Ok(chunk) => output.extend(chunk),
            Err(mpsc::TryRecvError::Empty) => {
                // No data yet, wait a bit
                thread::sleep(Duration::from_millis(10));
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
    }

    // Drop writer and master (but don't wait for reader thread - it may hang on Windows)
    drop(writer);
    drop(master);

    eprintln!("Captured {} bytes of output", output.len());

    // Debug: print raw bytes if requested
    if args.debug_raw {
        eprintln!("Raw output bytes:");
        for (i, &byte) in output.iter().enumerate() {
            if i > 0 && i % 16 == 0 {
                eprintln!();
            }
            eprint!("{:02X} ", byte);
        }
        eprintln!();
    }

    // Filter out OS-specific sequences (e.g., window title OSC from Windows ConPTY)
    let filtered = filter_osc_sequences(&output);
    eprintln!("After filtering OSC: {} bytes", filtered.len());

    // Process output through terminal emulator
    parser.process(&filtered);

    // Generate output based on format
    if args.output == "hex" {
        print_hex_state(&parser, args.rows, args.cols);
    } else if args.output == "text" {
        print_text_state(&parser, args.rows, args.cols);
    } else if args.output == "raw" {
        // Just output the raw bytes
        std::io::stdout().write_all(&output)?;
    }

    // Exit explicitly since the reader thread may still be blocking
    std::process::exit(0);
}

/// Print terminal state as hex format
/// Format: 22 chars per cell = 8 (codepoint) + 6 (fg RGB) + 6 (bg RGB) + 2 (attrs)
fn print_hex_state(parser: &vt100::Parser, rows: u16, cols: u16) {
    let screen = parser.screen();

    for row in 0..rows {
        for col in 0..cols {
            let cell = screen.cell(row, col).unwrap();

            // Get character (first char of contents, or space if empty)
            let ch = cell.contents().chars().next().unwrap_or(' ');
            let codepoint = ch as u32;

            // Get foreground color
            let (fg_r, fg_g, fg_b) = match cell.fgcolor() {
                vt100::Color::Rgb(r, g, b) => (r, g, b),
                vt100::Color::Idx(idx) => ansi_to_rgb(idx),
                vt100::Color::Default => (240, 240, 240), // Default light gray
            };

            // Get background color
            let (bg_r, bg_g, bg_b) = match cell.bgcolor() {
                vt100::Color::Rgb(r, g, b) => (r, g, b),
                vt100::Color::Idx(idx) => ansi_to_rgb(idx),
                vt100::Color::Default => (0, 0, 0), // Default black
            };

            // Get attributes as a byte
            let attrs = {
                let mut a = 0u8;
                if cell.bold() {
                    a |= 0x01;
                }
                if cell.italic() {
                    a |= 0x02;
                }
                if cell.underline() {
                    a |= 0x04;
                }
                if cell.inverse() {
                    a |= 0x08;
                }
                a
            };

            // Print in hex format: CCCCCCCC RRGGBB RRGGBB AA
            print!(
                "{:08X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                codepoint, fg_r, fg_g, fg_b, bg_r, bg_g, bg_b, attrs
            );
        }
    }
}

/// Print terminal state as text (just the characters)
fn print_text_state(parser: &vt100::Parser, rows: u16, cols: u16) {
    let screen = parser.screen();

    for row in 0..rows {
        let mut line = String::new();
        for col in 0..cols {
            let cell = screen.cell(row, col).unwrap();
            let ch = cell.contents().chars().next().unwrap_or(' ');
            line.push(ch);
        }
        // Trim trailing spaces
        let trimmed = line.trim_end();
        println!("{}", trimmed);
    }
}

/// Convert ANSI color index to RGB
fn ansi_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        // Standard colors
        0 => (0, 0, 0),       // Black
        1 => (205, 49, 49),   // Red
        2 => (13, 188, 121),  // Green
        3 => (229, 229, 16),  // Yellow
        4 => (36, 114, 200),  // Blue
        5 => (188, 63, 188),  // Magenta
        6 => (17, 168, 205),  // Cyan
        7 => (229, 229, 229), // White
        // Bright colors
        8 => (102, 102, 102),  // Bright Black
        9 => (241, 76, 76),    // Bright Red
        10 => (35, 209, 139),  // Bright Green
        11 => (245, 245, 67),  // Bright Yellow
        12 => (59, 142, 234),  // Bright Blue
        13 => (214, 112, 214), // Bright Magenta
        14 => (41, 184, 219),  // Bright Cyan
        15 => (255, 255, 255), // Bright White
        // 216 color cube (16-231)
        16..=231 => {
            let n = idx - 16;
            let r = (n / 36) % 6;
            let g = (n / 6) % 6;
            let b = n % 6;
            let to_val = |x: u8| if x == 0 { 0 } else { 55 + x * 40 };
            (to_val(r), to_val(g), to_val(b))
        }
        // Grayscale (232-255)
        232..=255 => {
            let gray = 8 + (idx - 232) * 10;
            (gray, gray, gray)
        }
    }
}
