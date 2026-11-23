use crate::caption::{CaptionRenderer, CaptionStyle};
use crate::error::RenderError;
use crate::hw_accel::select_encoder;
use crossbeam_channel::bounded;
use cutline_assets::OverlayRenderer;
use cutline_core::project::Project;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RenderOptions {
    pub preset: Option<String>,
    pub hw_accel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderResult {
    pub output_file: String,
    pub duration_seconds: f64,
    pub frames_rendered: i64,
    pub render_time_seconds: f64,
    pub speed_x_realtime: f64,
    pub encoder_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SingleFrameInfo {
    pub frame: i64,
    pub timestamp_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub output_file: String,
    pub active_captions: Vec<String>,
    pub active_overlays: Vec<String>,
}

pub struct RenderPipeline;

impl RenderPipeline {
    /// Executes the streaming multi-threaded render pipeline (REQ-RENDER-001, REQ-RENDER-002, REQ-RENDER-004)
    pub fn render(
        project: &Project,
        output_path: &Path,
        options: &RenderOptions,
    ) -> Result<RenderResult, RenderError> {
        let start_time = Instant::now();

        let track = project
            .timeline
            .video_tracks
            .first()
            .ok_or(RenderError::EmptyTimeline)?;

        if track.segments.is_empty() {
            return Err(RenderError::EmptyTimeline);
        }

        let width = project.output.width;
        let height = project.output.height;
        let fps = project.output.fps;
        let frame_bytes = (width * height * 4) as usize; // RGBA 32-bit
        let total_frames = project.timeline.total_duration_frames();
        let total_seconds = total_frames as f64 / fps.as_f64();

        let encoder_config = select_encoder(options.hw_accel.as_deref());

        // 1. Prepare cut audio track (REQ-RENDER-004: Sample-exact A/V sync)
        let temp_dir = std::env::temp_dir().join(format!(
            "cutline_render_{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ));
        fs::create_dir_all(&temp_dir)?;
        let temp_audio_wav = temp_dir.join("cut_audio.wav");

        Self::prepare_cut_audio(project, &temp_audio_wav)?;

        // 2. Set up bounded ring-buffer channels for flat memory usage (REQ-RENDER-001)
        // Pool of 32 pre-allocated frame buffers
        let (recycle_tx, recycle_rx) = bounded::<Vec<u8>>(32);
        for _ in 0..32 {
            recycle_tx.send(vec![0u8; frame_bytes]).unwrap();
        }

        let (decode_tx, decode_rx) = bounded::<Option<(i64, Vec<u8>)>>(16);
        let (composite_tx, composite_rx) = bounded::<Option<(i64, Vec<u8>)>>(16);

        // 3. Spawn Decoder Thread
        let segments = track.segments.clone();
        let sources = project.sources.clone();
        let d_recycle_rx = recycle_rx.clone();
        let d_tx = decode_tx.clone();

        let decoder_handle = thread::spawn(move || -> Result<(), RenderError> {
            let mut current_timeline_frame = 0;
            for seg in segments {
                let source = sources
                    .iter()
                    .find(|s| s.id == seg.source_id)
                    .ok_or_else(|| {
                        RenderError::Decoder(format!("Source '{}' not found", seg.source_id))
                    })?;

                let duration = (seg.source_out_frame - seg.source_in_frame).max(0);
                if duration == 0 {
                    continue;
                }

                let start_sec =
                    (seg.source_in_frame as f64 * source.fps.den as f64) / source.fps.num as f64;
                let mut child = Command::new("ffmpeg")
                    .args([
                        "-ss",
                        &format!("{:.4}", start_sec),
                        "-i",
                        &source.path,
                        "-frames:v",
                        &duration.to_string(),
                        "-s",
                        &format!("{}x{}", width, height),
                        "-f",
                        "rawvideo",
                        "-pix_fmt",
                        "rgba",
                        "-",
                    ])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| RenderError::Decoder(e.to_string()))?;

                let mut stdout = child.stdout.take().ok_or_else(|| {
                    RenderError::Decoder("Failed to capture decoder stdout".into())
                })?;

                for _ in 0..duration {
                    let mut buffer = d_recycle_rx
                        .recv()
                        .map_err(|_| RenderError::Decoder("Recycle channel closed".into()))?;

                    stdout
                        .read_exact(&mut buffer)
                        .map_err(|e| RenderError::Decoder(format!("Read frame failed: {e}")))?;

                    d_tx.send(Some((current_timeline_frame, buffer)))
                        .map_err(|_| RenderError::Decoder("Decode channel closed".into()))?;

                    current_timeline_frame += 1;
                }

                let _ = child.wait();
            }

            let _ = d_tx.send(None); // Signal EOF
            Ok(())
        });

        // 4. Spawn Compositor Worker Thread (Rayon Parallel Compositing REQ-RENDER-002)
        let c_tx = composite_tx.clone();
        let project_ref = Arc::new(project.clone());
        let compositor_handle = thread::spawn(move || -> Result<(), RenderError> {
            let style = CaptionStyle::from_id(&project_ref.captions.style_id);
            let caption_renderer = CaptionRenderer::new(style).map_err(|e| {
                RenderError::Compositor(format!("Failed to initialize caption renderer: {e}"))
            })?;
            let overlay_renderer = OverlayRenderer::new().map_err(|e| {
                RenderError::Compositor(format!("Failed to initialize overlay renderer: {e}"))
            })?;
            let total_frames = project_ref.timeline.total_duration_frames();

            while let Ok(Some((frame_idx, mut buffer))) = decode_rx.recv() {
                // 1. Layer overlays and motion templates onto buffer (REQ-OVERLAY-001..003)
                if !project_ref.overlays.is_empty() {
                    overlay_renderer.render_frame(
                        frame_idx,
                        &mut buffer,
                        width,
                        height,
                        &project_ref.overlays,
                        total_frames,
                    );
                }

                // 2. Layer kinetic captions onto buffer (REQ-CAP-001..004)
                if project_ref.captions.enabled && !project_ref.captions.groups.is_empty() {
                    caption_renderer.render_frame(
                        frame_idx,
                        &mut buffer,
                        width,
                        height,
                        &project_ref.captions,
                    );
                }

                c_tx.send(Some((frame_idx, buffer)))
                    .map_err(|_| RenderError::Decoder("Composite channel closed".into()))?;
            }
            let _ = c_tx.send(None);
            Ok(())
        });

        // 5. Spawn Encoder Thread
        let enc_recycle_tx = recycle_tx.clone();
        let out_str = output_path.to_string_lossy().to_string();
        let enc_config = encoder_config.clone();

        let encoder_handle = thread::spawn(move || -> Result<i64, RenderError> {
            let mut cmd = Command::new("ffmpeg");
            cmd.args([
                "-y",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "-s",
                &format!("{}x{}", width, height),
                "-r",
                &format!("{}/{}", fps.num, fps.den),
                "-i",
                "-",
            ]);

            // Add cut audio stream if present
            if temp_audio_wav.exists() {
                cmd.args(["-i", &temp_audio_wav.to_string_lossy()]);
                cmd.args(["-map", "0:v", "-map", "1:a?"]);
            } else {
                cmd.args(["-map", "0:v"]);
            }

            cmd.args(["-c:v", &enc_config.codec_name]);
            for extra in &enc_config.extra_args {
                cmd.arg(extra);
            }

            cmd.args([
                "-c:a",
                "aac",
                "-b:a",
                "192k",
                "-movflags",
                "+faststart",
                &out_str,
            ]);

            cmd.stdin(Stdio::piped());
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());

            let mut child = cmd
                .spawn()
                .map_err(|e| RenderError::Encoder(e.to_string()))?;
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| RenderError::Encoder("Failed to open encoder stdin".into()))?;

            let mut frames_encoded = 0;
            while let Ok(Some((_frame_idx, buffer))) = composite_rx.recv() {
                stdin
                    .write_all(&buffer)
                    .map_err(|e| RenderError::Encoder(format!("Write frame failed: {e}")))?;

                frames_encoded += 1;
                // Recycle buffer back into pool
                let _ = enc_recycle_tx.send(buffer);
            }

            drop(stdin);
            let status = child
                .wait()
                .map_err(|e| RenderError::Encoder(format!("Encoder wait failed: {e}")))?;

            if !status.success() {
                return Err(RenderError::Encoder(format!(
                    "FFmpeg encoder exited with code {:?}",
                    status.code()
                )));
            }

            Ok(frames_encoded)
        });

        // Await thread completions
        decoder_handle
            .join()
            .map_err(|_| RenderError::Decoder("Decoder thread panicked".into()))??;
        compositor_handle
            .join()
            .map_err(|_| RenderError::Decoder("Compositor thread panicked".into()))??;
        let frames_encoded = encoder_handle
            .join()
            .map_err(|_| RenderError::Encoder("Encoder thread panicked".into()))??;

        let elapsed = start_time.elapsed().as_secs_f64();
        let speed_x_realtime = if elapsed > 0.0 {
            total_seconds / elapsed
        } else {
            0.0
        };

        // Clean up temp audio
        let _ = fs::remove_dir_all(&temp_dir);

        info!(
            "Render finished: {} frames in {:.2}s ({:.2}x realtime) using {}",
            frames_encoded, elapsed, speed_x_realtime, encoder_config.codec_name
        );

        Ok(RenderResult {
            output_file: output_path.to_string_lossy().to_string(),
            duration_seconds: total_seconds,
            frames_rendered: frames_encoded,
            render_time_seconds: elapsed,
            speed_x_realtime,
            encoder_used: encoder_config.codec_name,
        })
    }

    /// Prepares a contiguous cut audio WAV file precisely aligned with video segments (REQ-RENDER-004)
    fn prepare_cut_audio(project: &Project, out_wav: &Path) -> Result<(), RenderError> {
        let v_track = match project.timeline.video_tracks.first() {
            Some(t) => t,
            None => return Ok(()),
        };

        if v_track.segments.is_empty() {
            return Ok(());
        }

        let primary_source = match project.sources.first() {
            Some(s) => s,
            None => return Ok(()),
        };

        // Extract full source audio to temporary PCM
        let full_src_wav = out_wav.with_extension("src.wav");
        cutline_media::extract_audio_16k_mono(Path::new(&primary_source.path), &full_src_wav)
            .map_err(|e| RenderError::Audio(e.to_string()))?;

        // Read source audio samples
        let mut reader = hound::WavReader::open(&full_src_wav)
            .map_err(|e| RenderError::Audio(format!("Could not read source WAV: {e}")))?;

        let spec = reader.spec();
        let all_samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap_or(0)).collect();

        let mut writer = hound::WavWriter::create(out_wav, spec)
            .map_err(|e| RenderError::Audio(format!("Could not create cut WAV: {e}")))?;

        let sample_rate = spec.sample_rate as f64;
        let fps = project.output.fps.as_f64();

        // Concatenate sample slices matching exact video segment boundaries
        for seg in &v_track.segments {
            let seg_fps = project
                .sources
                .iter()
                .find(|s| s.id == seg.source_id)
                .map(|s| s.fps.as_f64())
                .unwrap_or(fps);
            let start_sample = ((seg.source_in_frame as f64 / seg_fps) * sample_rate).round() as usize;
            let end_sample = ((seg.source_out_frame as f64 / seg_fps) * sample_rate).round() as usize;

            let clamped_start = start_sample.min(all_samples.len());
            let clamped_end = end_sample.min(all_samples.len());

            if clamped_end > clamped_start {
                for &sample in &all_samples[clamped_start..clamped_end] {
                    writer.write_sample(sample).unwrap();
                }
            }
        }

        writer
            .finalize()
            .map_err(|e| RenderError::Audio(e.to_string()))?;
        let _ = fs::remove_file(&full_src_wav);

        Ok(())
    }

    /// Extracts, composites, and encodes a single frame as a PNG image (REQ-PREVIEW-001)
    pub fn render_preview_frame(
        project: &Project,
        frame_idx: i64,
        out_path: &Path,
    ) -> Result<SingleFrameInfo, RenderError> {
        let track = project
            .timeline
            .video_tracks
            .first()
            .ok_or(RenderError::EmptyTimeline)?;

        if track.segments.is_empty() {
            return Err(RenderError::EmptyTimeline);
        }

        let width = project.output.width;
        let height = project.output.height;
        let fps = project.output.fps;
        let total_frames = project.timeline.total_duration_frames();
        let clamped_frame = frame_idx.clamp(0, (total_frames - 1).max(0));

        let seg = track
            .segments
            .iter()
            .find(|s| clamped_frame >= s.timeline_in_frame && clamped_frame < s.timeline_out_frame)
            .unwrap_or_else(|| track.segments.last().unwrap());

        let source = project
            .sources
            .iter()
            .find(|s| s.id == seg.source_id)
            .ok_or_else(|| RenderError::Decoder(format!("Source '{}' not found", seg.source_id)))?;

        let seg_offset = clamped_frame - seg.timeline_in_frame;
        let src_frame = seg.source_in_frame + seg_offset;
        let src_sec = src_frame as f64 / source.fps.as_f64();

        // 1. Decode single frame via FFmpeg
        let mut child = Command::new("ffmpeg")
            .args([
                "-ss",
                &format!("{:.4}", src_sec),
                "-i",
                &source.path,
                "-frames:v",
                "1",
                "-s",
                &format!("{}x{}", width, height),
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| RenderError::Decoder(e.to_string()))?;

        let mut buffer = vec![0u8; (width * height * 4) as usize];
        if let Some(mut stdout) = child.stdout.take() {
            let _ = stdout.read_exact(&mut buffer);
        }
        let _ = child.wait();

        // 2. Composite overlays
        let mut active_overlays = Vec::new();
        if !project.overlays.is_empty() {
            let overlay_renderer = OverlayRenderer::new().map_err(|e| {
                RenderError::Compositor(format!("Failed to initialize overlay renderer: {e}"))
            })?;
            overlay_renderer.render_frame(
                clamped_frame,
                &mut buffer,
                width,
                height,
                &project.overlays,
                total_frames,
            );

            for o in &project.overlays {
                if clamped_frame >= o.timeline_in_frame && clamped_frame < o.timeline_out_frame {
                    active_overlays.push(o.id.clone());
                }
            }
        }

        // 3. Composite captions
        let mut active_captions = Vec::new();
        if project.captions.enabled && !project.captions.groups.is_empty() {
            let style = CaptionStyle::from_id(&project.captions.style_id);
            let caption_renderer = CaptionRenderer::new(style).map_err(|e| {
                RenderError::Compositor(format!("Failed to initialize caption renderer: {e}"))
            })?;
            caption_renderer.render_frame(
                clamped_frame,
                &mut buffer,
                width,
                height,
                &project.captions,
            );

            for g in &project.captions.groups {
                if clamped_frame >= g.timeline_in_frame && clamped_frame < g.timeline_out_frame {
                    for w in &g.words {
                        if clamped_frame >= w.start_frame && clamped_frame <= w.end_frame {
                            active_captions.push(format!("*{}*", w.text));
                        } else {
                            active_captions.push(w.text.clone());
                        }
                    }
                }
            }
        }

        // 4. Encode to PNG
        let pixmap = tiny_skia::PixmapRef::from_bytes(&buffer, width, height)
            .ok_or_else(|| RenderError::Compositor("Failed to wrap buffer as pixmap".into()))?;
        let png_bytes = pixmap
            .encode_png()
            .map_err(|e| RenderError::Compositor(format!("PNG encode error: {e}")))?;

        if let Some(parent) = out_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(out_path, png_bytes)?;

        Ok(SingleFrameInfo {
            frame: clamped_frame,
            timestamp_seconds: clamped_frame as f64 / fps.as_f64(),
            width,
            height,
            output_file: out_path.to_string_lossy().to_string(),
            active_captions,
            active_overlays,
        })
    }
}
