//! Terminal session recording

use crate::{Result, TerminalError, TerminalEvent, TerminalRecording};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Terminal recorder
pub struct Recorder {
    recording: TerminalRecording,
    output_path: PathBuf,
    recording_active: Arc<AtomicBool>,
    start_time: Option<Instant>,
}

impl Recorder {
    pub fn new(title: &str, shell: &str, output_dir: &PathBuf) -> Result<Self> {
        let recording = TerminalRecording::new(title, shell, 80, 24);
        let output_path = output_dir.join(format!("{}.cast", recording.id));

        Ok(Self {
            recording,
            output_path,
            recording_active: Arc::new(AtomicBool::new(false)),
            start_time: None,
        })
    }

    /// Start recording a terminal session
    pub async fn start(&mut self) -> Result<()> {
        self.recording_active.store(true, Ordering::SeqCst);
        self.start_time = Some(Instant::now());

        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: self.recording.height,
                cols: self.recording.width,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::Recording(e.to_string()))?;

        let cmd = CommandBuilder::new(&self.recording.shell);
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::Recording(e.to_string()))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| TerminalError::Recording(e.to_string()))?;

        let mut writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalError::Recording(e.to_string()))?;

        let recording_active = Arc::clone(&self.recording_active);
        let start_time = self.start_time.unwrap();
        let mut events = Vec::new();

        // Read output in background
        let output_handle = std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            let mut collected_events = Vec::new();

            while recording_active.load(Ordering::SeqCst) {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buffer[..n]).to_string();
                        let time = start_time.elapsed();
                        collected_events.push(TerminalEvent::Output { time, data });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }

            collected_events
        });

        // Handle stdin forwarding
        let stdin = std::io::stdin();
        let mut stdin_handle = stdin.lock();
        let mut input_buffer = [0u8; 1024];

        loop {
            match stdin_handle.read(&mut input_buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let data = &input_buffer[..n];
                    writer.write_all(data).ok();
                    writer.flush().ok();

                    let time = start_time.elapsed();
                    let data_str = String::from_utf8_lossy(data).to_string();
                    events.push(TerminalEvent::Input { time, data: data_str });
                }
                Err(_) => break,
            }

            // Check if child exited
            if child.try_wait().map_err(|e| TerminalError::Recording(e.to_string()))?.is_some() {
                break;
            }
        }

        self.recording_active.store(false, Ordering::SeqCst);

        // Collect output events
        let output_events = output_handle.join().unwrap_or_default();

        // Merge and sort events
        events.extend(output_events);
        events.sort_by_key(|e| match e {
            TerminalEvent::Output { time, .. } => *time,
            TerminalEvent::Input { time, .. } => *time,
            TerminalEvent::Resize { time, .. } => *time,
            TerminalEvent::Marker { time, .. } => *time,
        });

        for event in events {
            self.recording.add_event(event);
        }

        // Save recording
        self.recording.save(&self.output_path)?;

        Ok(())
    }

    /// Stop recording
    pub fn stop(&self) {
        self.recording_active.store(false, Ordering::SeqCst);
    }

    /// Get the recording
    pub fn get_recording(&self) -> &TerminalRecording {
        &self.recording
    }

    /// Get output path
    pub fn get_output_path(&self) -> &PathBuf {
        &self.output_path
    }
}

/// Terminal session player for replaying recordings
pub struct Player {
    recording: TerminalRecording,
    current_index: usize,
    speed: f64,
}

impl Player {
    pub fn new(recording: TerminalRecording) -> Self {
        Self {
            recording,
            current_index: 0,
            speed: 1.0,
        }
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed.max(0.1).min(10.0);
    }

    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Get next event with timing
    pub fn next_event(&mut self) -> Option<(Duration, &TerminalEvent)> {
        if self.current_index >= self.recording.events.len() {
            return None;
        }

        let event = &self.recording.events[self.current_index];
        let time = match event {
            TerminalEvent::Output { time, .. } => *time,
            TerminalEvent::Input { time, .. } => *time,
            TerminalEvent::Resize { time, .. } => *time,
            TerminalEvent::Marker { time, .. } => *time,
        };

        // Calculate delay from previous event
        let delay = if self.current_index > 0 {
            let prev_event = &self.recording.events[self.current_index - 1];
            let prev_time = match prev_event {
                TerminalEvent::Output { time, .. } => *time,
                TerminalEvent::Input { time, .. } => *time,
                TerminalEvent::Resize { time, .. } => *time,
                TerminalEvent::Marker { time, .. } => *time,
            };
            time.saturating_sub(prev_time)
        } else {
            Duration::ZERO
        };

        self.current_index += 1;

        // Adjust delay for playback speed
        let adjusted_delay = Duration::from_secs_f64(delay.as_secs_f64() / self.speed);

        Some((adjusted_delay, event))
    }

    /// Play recording to stdout
    pub async fn play(&mut self) -> crate::Result<()> {
        use std::io::Write;

        self.reset();
        let mut stdout = std::io::stdout();

        while let Some((delay, event)) = self.next_event() {
            // Wait for the appropriate time
            tokio::time::sleep(delay).await;

            // Output the event
            if let TerminalEvent::Output { data, .. } = event {
                stdout.write_all(data.as_bytes()).ok();
                stdout.flush().ok();
            }
        }

        Ok(())
    }

    /// Get recording info
    pub fn info(&self) -> RecordingInfo {
        RecordingInfo {
            id: self.recording.id.clone(),
            title: self.recording.title.clone(),
            duration: self.recording.duration,
            event_count: self.recording.events.len(),
            recorded_at: self.recording.recorded_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecordingInfo {
    pub id: String,
    pub title: String,
    pub duration: Duration,
    pub event_count: usize,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}
