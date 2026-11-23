use clap::{Parser, Subcommand};
use cutline_cli::error::AppError;
use cutline_cli::mcp::run_mcp_server;
use cutline_cli::ops::*;
use cutline_cli::response::{print_error, print_success};
use cutline_media::verify_environment;
use std::path::PathBuf;
use std::process::ExitCode;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(
    name = "cutline",
    version = "0.1.0",
    about = "AI-drivable video editing engine"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Probe media file for duration, resolution, frame rate, and codecs
    Probe {
        /// Path to video or media file
        video: PathBuf,
    },

    /// Initialize a new non-destructive project.json
    #[command(subcommand)]
    Project(ProjectCommands),

    /// Audio extraction and management commands
    #[command(subcommand)]
    Audio(AudioCommands),

    /// Speech-to-text offline transcription with word-level timestamps
    Transcribe {
        /// Path to project.json or media file
        project: PathBuf,

        /// Whisper model variant (tiny, base, small)
        #[arg(long, default_value = "base")]
        model: String,

        /// Output transcript path (default transcript.json)
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Audio and video analysis commands (silence, scene detection)
    #[command(subcommand)]
    Analyze(AnalyzeCommands),

    /// Non-destructive timeline editing operations (cut, trim, auto-cut-silence)
    #[command(subcommand)]
    Timeline(TimelineCommands),

    /// Kinetic word-by-word captions commands (REQ-CAP-001..004)
    #[command(subcommand)]
    Captions(CaptionsCommands),

    /// Visual overlay and motion template commands (REQ-OVERLAY-001..004)
    #[command(subcommand)]
    Overlay(OverlayCommands),

    /// Asset library commands (list, add built-in or custom assets)
    #[command(subcommand)]
    Assets(AssetsCommands),

    /// Render final video through the multi-threaded streaming pipeline
    Render {
        /// Project file to render
        project: PathBuf,

        /// Output video file path
        #[arg(long, default_value = "final.mp4")]
        out: PathBuf,

        /// Export preset (tiktok, youtube, fast-draft)
        #[arg(long)]
        preset: Option<String>,

        /// Hardware encoder preference (auto, nvenc, amf, qsv, vt, vaapi, none)
        #[arg(long)]
        hw_accel: Option<String>,
    },

    /// Render a single frame with composited video, captions, and overlays to PNG (REQ-PREVIEW-001)
    PreviewFrame {
        /// Project file
        project: PathBuf,

        /// Timestamp or frame (e.g. 00:00:12.400, 12.4s, frame:372)
        #[arg(long)]
        time: String,

        /// Output PNG image path
        #[arg(long, default_value = "frame.png")]
        out: PathBuf,
    },

    /// Render an evenly spaced series of thumbnail frames across the timeline (REQ-PREVIEW-002)
    PreviewStrip {
        /// Project file
        project: PathBuf,

        /// Number of preview frames to extract (default: 8)
        #[arg(long, default_value_t = 8)]
        count: usize,

        /// Output directory for preview images
        #[arg(long, default_value = "previews")]
        out_dir: PathBuf,
    },

    /// Validate edit rules, safe areas, caption collisions, and brand constraints (REQ-LINT-001)
    Lint {
        /// Project file
        project: PathBuf,

        /// Optional path to transcript.json for cut-word split validation
        #[arg(long)]
        transcript: Option<PathBuf>,
    },

    /// Export the JSON Schema for project.json
    Schema,

    /// Start Model Context Protocol (MCP) JSON-RPC 2.0 stdio server (REQ-AGENT-004)
    Mcp,
}

#[derive(Subcommand, Debug)]
enum ProjectCommands {
    /// Initialize a project file from a primary video
    Init {
        /// Primary talking-head video file
        video: PathBuf,

        /// Optional B-roll footage paths
        #[arg(long = "b-roll", num_args = 1..)]
        b_roll: Vec<PathBuf>,

        /// Output preset (tiktok, youtube, fast-draft)
        #[arg(long, default_value = "tiktok")]
        preset: String,

        /// Target project file output path
        #[arg(long, default_value = "project.json")]
        out: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum AudioCommands {
    /// Extract audio to a 16kHz mono 16-bit PCM WAV file
    Extract {
        /// Media file or project.json
        input: PathBuf,

        /// Output WAV file path
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum AnalyzeCommands {
    /// Analyze audio using RMS energy hysteresis to detect pauses and dead air
    Silence {
        /// Project file or audio WAV file
        project: PathBuf,

        /// Activation threshold in dB (below this enters silence)
        #[arg(long, default_value_t = -32.0)]
        threshold_db: f32,

        /// Minimum silence duration in milliseconds
        #[arg(long, default_value_t = 300)]
        min_duration_ms: i64,
    },

    /// Detect visual scene cuts in the primary video
    Scenes {
        /// Project file or video file
        project: PathBuf,

        /// Scene change sensitivity threshold (0.0 - 1.0)
        #[arg(long, default_value_t = 0.4)]
        threshold: f32,
    },
}

#[derive(Subcommand, Debug)]
enum TimelineCommands {
    /// Cut out an interval from the timeline (REQ-EDIT-001)
    Cut {
        /// Project file
        project: PathBuf,

        /// Start timestamp or frame (e.g. 00:00:10.500, 10.5s, frame:315)
        #[arg(long)]
        start: String,

        /// End timestamp or frame
        #[arg(long)]
        end: String,
    },

    /// Automatically cut all detected silences with safety padding (REQ-EDIT-002)
    AutoCutSilence {
        /// Project file
        project: PathBuf,

        /// Safety padding in milliseconds to keep around speech (default: 50ms)
        #[arg(long, default_value_t = 50)]
        padding_ms: i64,
    },

    /// Trim global project start and end points
    Trim {
        /// Project file
        project: PathBuf,

        /// Trim in timestamp or frame
        #[arg(long = "in")]
        trim_in: String,

        /// Trim out timestamp or frame
        #[arg(long = "out")]
        trim_out: String,
    },
}

#[derive(Subcommand, Debug)]
enum CaptionsCommands {
    /// Apply kinetic captions with word grouping and animated styling (REQ-CAP-001..004)
    Apply {
        /// Project file (e.g. project.json)
        project: PathBuf,

        /// Caption style preset or custom JSON (clean-yellow, mono-warm)
        #[arg(long, default_value = "clean-yellow")]
        style: String,

        /// Path to transcript.json (defaults to transcript.json in project dir)
        #[arg(long)]
        transcript: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum OverlayCommands {
    /// Add an overlay or motion template to the project timeline (REQ-OVERLAY-001..004)
    Add {
        /// Project file
        project: PathBuf,

        /// Asset identifier (e.g. svg:bird-flying, template:lower-third, template:keyword-badge, template:animated-progress, template:punch-in-zoom)
        #[arg(long)]
        asset: String,

        /// Start timestamp or frame (e.g. 00:00:01.500, 1.5s, frame:45)
        #[arg(long)]
        at: String,

        /// Duration as timestamp or frame (e.g. 00:00:02.000, 2s, frame:60)
        #[arg(long)]
        duration: String,

        /// Anchor position (top_center, center, lower_third, bottom_center)
        #[arg(long, default_value = "top_center")]
        anchor: String,

        /// Scale multiplier (default: 1.0)
        #[arg(long, default_value_t = 1.0)]
        scale: f32,

        /// Opacity (0.0 - 1.0, default: 1.0)
        #[arg(long, default_value_t = 1.0)]
        opacity: f32,

        /// Title text for template overlays (e.g. speaker name or badge text)
        #[arg(long)]
        title: Option<String>,

        /// Subtitle text for template overlays (e.g. speaker role)
        #[arg(long)]
        subtitle: Option<String>,

        /// Semantic reason why this visual is placed (e.g. "Speaker said: 'imagine a bird is flying'")
        #[arg(long)]
        reason: Option<String>,

        /// Associated transcript word indices (e.g. 12,13,14,15)
        #[arg(long, value_delimiter = ',')]
        word_range: Option<Vec<usize>>,
    },

    /// List active overlays in the project timeline
    List {
        /// Project file
        project: PathBuf,
    },

    /// Remove an overlay from the project timeline by ID
    Remove {
        /// Project file
        project: PathBuf,

        /// Overlay ID to remove
        #[arg(long)]
        id: String,
    },
}

#[derive(Subcommand, Debug)]
enum AssetsCommands {
    /// List all built-in procedural templates and registered vector assets
    List,

    /// Register a custom SVG or image asset into the project library
    Add {
        /// Path to asset file (SVG, PNG, JPG, MP4)
        path: PathBuf,

        /// Optional custom asset identifier
        #[arg(long)]
        id: Option<String>,
    },
}

fn run_op<F>(command: &str, f: F) -> ExitCode
where
    F: FnOnce() -> Result<serde_json::Value, AppError>,
{
    match f() {
        Ok(val) => {
            print_success(command, val);
            ExitCode::SUCCESS
        }
        Err(err) => {
            print_error(command, &err.code, &err.message, &err.hint);
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    // REQ-AGENT-002: Direct all diagnostic logs to stderr only
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();

    // REQ-MEDIA-001: FFmpeg startup verification
    if let Err(e) = verify_environment() {
        print_error("startup", e.error_code(), &e.to_string(), e.hint());
        return ExitCode::FAILURE;
    }

    match cli.command {
        Commands::Probe { video } => run_op("probe", || op_probe(&video)),
        Commands::Project(ProjectCommands::Init {
            video,
            b_roll,
            preset,
            out,
        }) => run_op("project.init", || {
            op_project_init(&video, &b_roll, &preset, &out)
        }),
        Commands::Audio(AudioCommands::Extract { input, out }) => {
            run_op("audio.extract", || op_audio_extract(&input, out.as_deref()))
        }
        Commands::Transcribe {
            project,
            model,
            out,
        } => run_op("transcribe", || {
            op_transcribe(&project, &model, out.as_deref())
        }),
        Commands::Analyze(AnalyzeCommands::Silence {
            project,
            threshold_db,
            min_duration_ms,
        }) => run_op("analyze.silence", || {
            op_analyze_silence(&project, threshold_db, min_duration_ms)
        }),
        Commands::Analyze(AnalyzeCommands::Scenes { project, threshold }) => {
            run_op("analyze.scenes", || op_analyze_scenes(&project, threshold))
        }
        Commands::Timeline(TimelineCommands::Cut {
            project,
            start,
            end,
        }) => run_op("timeline.cut", || op_timeline_cut(&project, &start, &end)),
        Commands::Timeline(TimelineCommands::AutoCutSilence {
            project,
            padding_ms,
        }) => run_op("timeline.auto_cut_silence", || {
            op_timeline_auto_cut_silence(&project, padding_ms)
        }),
        Commands::Timeline(TimelineCommands::Trim {
            project,
            trim_in,
            trim_out,
        }) => run_op("timeline.trim", || {
            op_timeline_trim(&project, &trim_in, &trim_out)
        }),
        Commands::Captions(CaptionsCommands::Apply {
            project,
            style,
            transcript,
        }) => run_op("captions.apply", || {
            op_captions_apply(&project, &style, transcript.as_deref())
        }),
        Commands::Overlay(OverlayCommands::Add {
            project,
            asset,
            at,
            duration,
            anchor,
            scale,
            opacity,
            title,
            subtitle,
            reason,
            word_range,
        }) => run_op("overlay.add", || {
            op_overlay_add(
                &project,
                &asset,
                &at,
                &duration,
                &anchor,
                scale,
                opacity,
                title.as_deref(),
                subtitle.as_deref(),
                reason.as_deref(),
                word_range.as_deref(),
            )
        }),
        Commands::Overlay(OverlayCommands::List { project }) => {
            run_op("overlay.list", || op_overlay_list(&project))
        }
        Commands::Overlay(OverlayCommands::Remove { project, id }) => {
            run_op("overlay.remove", || op_overlay_remove(&project, &id))
        }
        Commands::Assets(AssetsCommands::List) => run_op("assets.list", op_assets_list),
        Commands::Assets(AssetsCommands::Add { path, id }) => {
            run_op("assets.add", || op_assets_add(&path, id.as_deref()))
        }
        Commands::Render {
            project,
            out,
            preset,
            hw_accel,
        } => run_op("render", || {
            op_render(&project, &out, preset.as_deref(), hw_accel.as_deref())
        }),
        Commands::PreviewFrame { project, time, out } => {
            run_op("preview-frame", || op_preview_frame(&project, &time, &out))
        }
        Commands::PreviewStrip {
            project,
            count,
            out_dir,
        } => run_op("preview-strip", || {
            op_preview_strip(&project, count, &out_dir)
        }),
        Commands::Lint {
            project,
            transcript,
        } => match op_lint(&project, transcript.as_deref()) {
            Ok(val) => {
                let passed = val.get("passed").and_then(|v| v.as_bool()).unwrap_or(true);
                print_success("lint", val);
                if passed {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(err) => {
                print_error("lint", &err.code, &err.message, &err.hint);
                ExitCode::FAILURE
            }
        },
        Commands::Schema => match op_schema() {
            Ok(val) => {
                println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
                ExitCode::SUCCESS
            }
            Err(err) => {
                print_error("schema", &err.code, &err.message, &err.hint);
                ExitCode::FAILURE
            }
        },
        Commands::Mcp => {
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            if let Err(e) = run_mcp_server(stdin.lock(), stdout.lock()) {
                eprintln!("Cutline MCP server exited with error: {e}");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
    }
}
