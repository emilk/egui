#[derive(Clone)]
struct Frame {
    /// `_main` is usually as the deepest depth.
    depth: usize,
    name: String,
    file_and_line: String,
}

/// Capture a callstack, skipping the frames that are not interesting.
///
/// In particular: slips everything before `egui::Context::run`,
/// and skipping all frames in the `egui::` namespace.
pub fn capture() -> String {
    let mut frames = vec![];
    let mut depth = 0;

    backtrace::trace(|frame| {
        // Resolve this instruction pointer to a symbol name
        backtrace::resolve_frame(frame, |symbol| {
            let mut file_and_line = symbol.filename().map(shorten_source_file_path);

            if let Some(file_and_line) = &mut file_and_line {
                if let Some(line_nr) = symbol.lineno() {
                    file_and_line.push_str(&format!(":{line_nr}"));
                }
            }
            let file_and_line = file_and_line.unwrap_or_default();

            let name = symbol
                .name()
                .map(|name| name.to_string())
                .unwrap_or_default();

            frames.push(Frame {
                depth,
                name,
                file_and_line,
            });
        });

        depth += 1; // note: we can resolve multiple symbols on the same frame.

        true // keep going to the next frame
    });

    if frames.is_empty() {
        return Default::default();
    }

    // Inclusive:
    let mut min_depth = 0;
    let mut max_depth = frames.len() - 1;

    for frame in &frames {
        if frame.name.starts_with("egui::callstack::capture") {
            min_depth = frame.depth + 1;
        }
        if frame.name.starts_with("egui::context::Context::run") {
            max_depth = frame.depth;
        }
    }

    // Remove frames that are uninteresting:
    frames.retain(|frame| {
        // Keep some special frames to give the user a sense of chronology:
        if frame.name == "main"
            || frame.name == "_main"
            || frame.name.starts_with("egui::context::Context::run")
            || frame.name.starts_with("eframe::run_native")
        {
            return true;
        }

        if frame.depth < min_depth || max_depth < frame.depth {
            return false;
        }

        // Remove stuff that isn't user calls:
        let skip_prefixes = [
            // "backtrace::", // not needed, since we cut at at egui::callstack::capture
            "egui::",
            "<egui::",
            "<F as egui::widgets::Widget>",
            "egui_plot::",
            "egui_extras::",
            "core::ptr::drop_in_place<egui::ui::Ui>::",
            "eframe::",
            "core::ops::function::FnOnce::call_once",
            "<alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once",
        ];
        for prefix in skip_prefixes {
            if frame.name.starts_with(prefix) {
                return false;
            }
        }
        true
    });

    frames.reverse(); // main on top, i.e. chronological order. Same as Python.

    let mut deepest_depth = 0;
    let mut widest_file_line = 0;
    for frame in &frames {
        deepest_depth = frame.depth.max(deepest_depth);
        widest_file_line = frame.file_and_line.len().max(widest_file_line);
    }

    let widest_depth = deepest_depth.to_string().len();

    let mut formatted = String::new();

    if !frames.is_empty() {
        let mut last_depth = frames[0].depth;

        for frame in &frames {
            let Frame {
                depth,
                name,
                file_and_line,
            } = frame;

            if frame.depth + 1 < last_depth || last_depth + 1 < frame.depth {
                // Show that some frames were elided
                formatted.push_str(&format!("{:widest_depth$}  …\n", ""));
            }

            formatted.push_str(&format!(
                "{depth:widest_depth$}: {file_and_line:widest_file_line$}  {name}\n"
            ));

            last_depth = frame.depth;
        }
    }

    formatted
}

/// Shorten a path to a Rust source file from a callstack.
///
/// Example input:
/// * `/Users/emilk/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.24.1/src/runtime/runtime.rs`
/// * `crates/rerun/src/main.rs`
/// * `/rustc/d5a82bbd26e1ad8b7401f6a718a9c57c96905483/library/core/src/ops/function.rs`
fn shorten_source_file_path(path: &std::path::Path) -> String {
    // Look for `src` and strip everything up to it.

    let components: Vec<_> = path.iter().map(|path| path.to_string_lossy()).collect();

    let mut src_idx = None;
    for (i, c) in components.iter().enumerate() {
        if c == "src" {
            src_idx = Some(i);
        }
    }

    // Look for the last `src`:
    if let Some(src_idx) = src_idx {
        // Before `src` comes the name of the crate - let's include that:
        let first_index = src_idx.saturating_sub(1);

        let mut output = components[first_index].to_string();
        for component in &components[first_index + 1..] {
            output.push('/');
            output.push_str(component);
        }
        output
    } else {
        // No `src` directory found - weird!
        path.display().to_string()
    }
}

#[test]
fn test_shorten_path() {
    for (before, after) in [
        ("/Users/emilk/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.24.1/src/runtime/runtime.rs", "tokio-1.24.1/src/runtime/runtime.rs"),
        ("crates/rerun/src/main.rs", "rerun/src/main.rs"),
        ("/rustc/d5a82bbd26e1ad8b7401f6a718a9c57c96905483/library/core/src/ops/function.rs", "core/src/ops/function.rs"),
        ("/weird/path/file.rs", "/weird/path/file.rs"),
        ]
        {
        use std::str::FromStr as _;
        let before = std::path::PathBuf::from_str(before).unwrap();
        assert_eq!(shorten_source_file_path(&before), after);
    }
}
