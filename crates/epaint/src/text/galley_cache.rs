use std::sync::Arc;

use emath::OrderedFloat;
use nohash_hasher::IntMap;

use crate::text::{
    ByteIndex, Galley, LayoutJob, LayoutSection, fonts::FontsImpl, text_layout::layout,
};

/// A laid-out [`Galley`], and what we need to know to evict it.
struct CachedGalley {
    /// When it was last used
    last_used: u32,

    /// Hashes of all other entries this one depends on for quick re-layout.
    /// Their `last_used`s should be updated alongside this one to make sure they're
    /// not evicted.
    children: Option<Arc<[u64]>>,

    galley: Arc<Galley>,
}

/// Memoizes [`LayoutJob`] → [`Galley`], so that repeated layout of the same text is cheap.
///
/// Entries not used during a frame are evicted in [`Self::flush_cache`].
#[derive(Default)]
pub(super) struct GalleyCache {
    /// Frame counter used to do garbage collection on the cache
    generation: u32,
    cache: IntMap<u64, CachedGalley>,
}

impl GalleyCache {
    fn layout_internal(
        &mut self,
        fonts: &mut FontsImpl,
        mut job: LayoutJob,
        pixels_per_point: f32,
        allow_split_paragraphs: bool,
    ) -> (u64, Arc<Galley>) {
        if job.wrap.max_width.is_finite() {
            // Protect against rounding errors in egui layout code.

            // Say the user asks to wrap at width 200.0.
            // The text layout wraps, and reports that the final width was 196.0 points.
            // This then trickles up the `Ui` chain and gets stored as the width for a tooltip (say).
            // On the next frame, this is then set as the max width for the tooltip,
            // and we end up calling the text layout code again, this time with a wrap width of 196.0.
            // Except, somewhere in the `Ui` chain with added margins etc, a rounding error was introduced,
            // so that we actually set a wrap-width of 195.9997 instead.
            // Now the text that fit perfrectly at 196.0 needs to wrap one word earlier,
            // and so the text re-wraps and reports a new width of 185.0 points.
            // And then the cycle continues.

            // So we limit max_width to integers.

            // Related issues:
            // * https://github.com/emilk/egui/issues/4927
            // * https://github.com/emilk/egui/issues/4928
            // * https://github.com/emilk/egui/issues/5084
            // * https://github.com/emilk/egui/issues/5163

            job.wrap.max_width = job.wrap.max_width.round();
        }

        let hash = crate::util::hash((&job, OrderedFloat(pixels_per_point))); // TODO(emilk): even faster hasher?

        let galley = match self.cache.entry(hash) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                // The job was found in cache - no need to re-layout.
                let cached = entry.into_mut();
                cached.last_used = self.generation;

                let galley = Arc::clone(&cached.galley);
                if let Some(children) = &cached.children {
                    // The point of `allow_split_paragraphs` is to split large jobs into paragraph,
                    // and then cache each paragraph individually.
                    // That way, if we edit a single paragraph, only that paragraph will be re-layouted.
                    // For that to work we need to keep all the child/paragraph
                    // galleys alive while the parent galley is alive:
                    for child_hash in Arc::clone(children).iter() {
                        if let Some(cached_child) = self.cache.get_mut(child_hash) {
                            cached_child.last_used = self.generation;
                        }
                    }
                }

                galley
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let job = Arc::new(job);
                if allow_split_paragraphs && should_cache_each_paragraph_individually(&job) {
                    let (child_galleys, child_hashes) =
                        self.layout_each_paragraph_individually(fonts, &job, pixels_per_point);
                    debug_assert_eq!(
                        child_hashes.len(),
                        child_galleys.len(),
                        "Bug in `layout_each_paragraph_individually`"
                    );
                    let galley = Arc::new(Galley::concat(job, &child_galleys, pixels_per_point));

                    self.cache.insert(
                        hash,
                        CachedGalley {
                            last_used: self.generation,
                            children: Some(child_hashes.into()),
                            galley: Arc::clone(&galley),
                        },
                    );
                    galley
                } else {
                    let galley = layout(fonts, pixels_per_point, job);
                    let galley = Arc::new(galley);
                    entry.insert(CachedGalley {
                        last_used: self.generation,
                        children: None,
                        galley: Arc::clone(&galley),
                    });
                    galley
                }
            }
        };

        (hash, galley)
    }

    pub(super) fn layout(
        &mut self,
        fonts: &mut FontsImpl,
        pixels_per_point: f32,
        job: LayoutJob,
        allow_split_paragraphs: bool,
    ) -> Arc<Galley> {
        self.layout_internal(fonts, job, pixels_per_point, allow_split_paragraphs)
            .1
    }

    /// Split on `\n` and lay out (and cache) each paragraph individually.
    fn layout_each_paragraph_individually(
        &mut self,
        fonts: &mut FontsImpl,
        job: &LayoutJob,
        pixels_per_point: f32,
    ) -> (Vec<Arc<Galley>>, Vec<u64>) {
        profiling::function_scope!();

        let mut current_section = 0;
        let mut start = 0;
        let mut max_rows_remaining = job.wrap.max_rows;
        let mut child_galleys = Vec::new();
        let mut child_hashes = Vec::new();

        while start < job.text.len() {
            let is_first_paragraph = start == 0;
            // `end` will not include the `\n` since we don't want to create an empty row in our
            // split galley
            let mut end = job.text[start..]
                .find('\n')
                .map_or(job.text.len(), |i| start + i);
            if end == job.text.len() - 1 && job.text.ends_with('\n') {
                end += 1; // If the text ends with a newline, we include it in the last paragraph.
            }

            let mut paragraph_job = LayoutJob {
                text: job.text[start..end].to_owned(),
                wrap: crate::text::TextWrapping {
                    max_rows: max_rows_remaining,
                    ..job.wrap
                },
                sections: Vec::new(),
                break_on_newline: job.break_on_newline,
                halign: job.halign,
                justify: job.justify,
                first_row_min_height: if is_first_paragraph {
                    job.first_row_min_height
                } else {
                    0.0
                },
                round_output_to_gui: job.round_output_to_gui,
                keep_trailing_whitespace: job.keep_trailing_whitespace,
            };

            // Add overlapping sections:
            for section in &job.sections[current_section..job.sections.len()] {
                let LayoutSection {
                    leading_space,
                    byte_range: section_range,
                    format,
                } = section;

                // `start` and `end` are the byte range of the current paragraph.
                // How does the current section overlap with the paragraph range?

                if section_range.end <= ByteIndex(start) {
                    // The section is behind us
                    current_section += 1;
                } else if ByteIndex(end) < section_range.start {
                    break; // Haven't reached this one yet.
                } else {
                    // Section range overlaps with paragraph range
                    debug_assert!(
                        section_range.start <= section_range.end,
                        "Bad byte_range: {section_range:?}"
                    );
                    let new_range = section_range.start.saturating_sub(start)
                        ..(section_range.end.min(ByteIndex(end))).saturating_sub(start);
                    debug_assert!(
                        new_range.start <= new_range.end,
                        "Bad new section range: {new_range:?}"
                    );
                    paragraph_job.sections.push(LayoutSection {
                        leading_space: if ByteIndex(start) <= section_range.start {
                            *leading_space
                        } else {
                            0.0
                        },
                        byte_range: new_range,
                        format: format.clone(),
                    });
                }
            }

            // TODO(emilk): we could lay out each paragraph in parallel to get a nice speedup on multicore machines.
            let (hash, galley) =
                self.layout_internal(fonts, paragraph_job, pixels_per_point, false);
            child_hashes.push(hash);

            // This will prevent us from invalidating cache entries unnecessarily:
            if max_rows_remaining != usize::MAX {
                max_rows_remaining -= galley.rows.len();
            }

            let elided = galley.elided;
            child_galleys.push(galley);
            if elided {
                break;
            }

            start = end + 1;
        }

        (child_galleys, child_hashes)
    }

    pub fn num_galleys_in_cache(&self) -> usize {
        self.cache.len()
    }

    /// Must be called once per frame to clear the [`Galley`] cache.
    pub fn flush_cache(&mut self) {
        let current_generation = self.generation;
        self.cache.retain(|_key, cached| {
            cached.last_used == current_generation // only keep those that were used this frame
        });
        self.generation = self.generation.wrapping_add(1);
    }
}

/// If true, lay out and cache each paragraph (sections separated by newlines) individually.
///
/// This makes it much faster to re-layout the full text when only a portion of it has changed since last frame, i.e. when editing somewhere in a file with thousands of lines/paragraphs.
fn should_cache_each_paragraph_individually(job: &LayoutJob) -> bool {
    // We currently don't support this elided text, i.e. when `max_rows` is set.
    // Most often, elided text is elided to one row,
    // and so will always be fast to lay out.
    job.break_on_newline && job.wrap.max_rows == usize::MAX && job.text.contains('\n')
}

#[cfg(feature = "default_fonts")]
#[cfg(test)]
mod tests {
    use core::f32;

    use super::*;
    use crate::text::{FontDefinitions, FontFamily, FontId, TextOptions, TextWrapping};
    use crate::{Stroke, text::TextFormat};
    use ecolor::Color32;
    use emath::Align;

    fn jobs() -> Vec<LayoutJob> {
        vec![
            LayoutJob::simple(
                String::default(),
                FontId::new(14.0, FontFamily::Monospace),
                Color32::WHITE,
                f32::INFINITY,
            ),
            LayoutJob::simple(
                "ends with newlines\n\n".to_owned(),
                FontId::new(14.0, FontFamily::Monospace),
                Color32::WHITE,
                f32::INFINITY,
            ),
            LayoutJob::simple(
                "Simple test.".to_owned(),
                FontId::new(14.0, FontFamily::Monospace),
                Color32::WHITE,
                f32::INFINITY,
            ),
            {
                let mut job = LayoutJob::simple(
                    "hi".to_owned(),
                    FontId::default(),
                    Color32::WHITE,
                    f32::INFINITY,
                );
                job.append("\n", 0.0, TextFormat::default());
                job.append("\n", 0.0, TextFormat::default());
                job.append("world", 0.0, TextFormat::default());
                job.wrap.max_rows = 2;
                job
            },
            {
                let mut job = LayoutJob::simple(
                    "Test text with a lot of words\n and a newline.".to_owned(),
                    FontId::new(14.0, FontFamily::Monospace),
                    Color32::WHITE,
                    40.0,
                );
                job.first_row_min_height = 30.0;
                job
            },
            LayoutJob::simple(
                "This some text that may be long.\nDet kanske också finns lite ÅÄÖ här.".to_owned(),
                FontId::new(14.0, FontFamily::Proportional),
                Color32::WHITE,
                50.0,
            ),
            {
                let mut job = LayoutJob {
                    first_row_min_height: 20.0,
                    ..Default::default()
                };
                job.append(
                    "1st paragraph has underline and strikethrough, and has some non-ASCII characters:\n ÅÄÖ.",
                    0.0,
                    TextFormat {
                        font_id: FontId::new(15.0, FontFamily::Monospace),
                        underline: Stroke::new(1.0, Color32::RED),
                        strikethrough: Stroke::new(1.0, Color32::GREEN),
                        ..Default::default()
                    },
                );
                job.append(
                    "2nd paragraph has some leading space.\n",
                    16.0,
                    TextFormat {
                        font_id: FontId::new(14.0, FontFamily::Proportional),
                        ..Default::default()
                    },
                );
                job.append(
                    "3rd paragraph is kind of boring, but has italics.\nAnd a newline",
                    0.0,
                    TextFormat {
                        font_id: FontId::new(10.0, FontFamily::Proportional),
                        italics: true,
                        ..Default::default()
                    },
                );

                job
            },
            {
                // Regression test for <https://github.com/emilk/egui/issues/7378>
                let mut job = LayoutJob::default();
                job.append("\n", 0.0, TextFormat::default());
                job.append("", 0.0, TextFormat::default());
                job
            },
        ]
    }

    #[expect(clippy::print_stdout)]
    #[test]
    fn test_split_paragraphs() {
        for pixels_per_point in [1.0, 2.0_f32.sqrt(), 2.0] {
            let mut fonts = FontsImpl::new(TextOptions::default(), FontDefinitions::default());

            for halign in [Align::Min, Align::Center, Align::Max] {
                for justify in [false, true] {
                    for mut job in jobs() {
                        job.halign = halign;
                        job.justify = justify;

                        let whole = GalleyCache::default().layout(
                            &mut fonts,
                            pixels_per_point,
                            job.clone(),
                            false,
                        );

                        let split = GalleyCache::default().layout(
                            &mut fonts,
                            pixels_per_point,
                            job.clone(),
                            true,
                        );

                        for (i, row) in whole.rows.iter().enumerate() {
                            println!(
                                "Whole row {i}: section_index_at_start={}, first glyph section_index: {:?}",
                                row.row.section_index_at_start,
                                row.row.glyphs.first().map(|g| g.section_index)
                            );
                        }
                        for (i, row) in split.rows.iter().enumerate() {
                            println!(
                                "Split row {i}: section_index_at_start={}, first glyph section_index: {:?}",
                                row.row.section_index_at_start,
                                row.row.glyphs.first().map(|g| g.section_index)
                            );
                        }

                        // Don't compare for equaliity; but format with a specific precision and make sure we hit that.
                        // NOTE: we use a rather low precision, because as long as we're within a pixel I think it's good enough.
                        similar_asserts::assert_eq!(
                            format!("{:#.1?}", split),
                            format!("{:#.1?}", whole),
                            "pixels_per_point: {pixels_per_point:.2}, input text: '{}'",
                            job.text
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_intrinsic_size() {
        let pixels_per_point = [1.0, 1.3, 2.0, 0.867];
        let max_widths = [40.0, 80.0, 133.0, 200.0];
        let rounded_output_to_gui = [false, true];

        for pixels_per_point in pixels_per_point {
            let mut fonts = FontsImpl::new(TextOptions::default(), FontDefinitions::default());

            for &max_width in &max_widths {
                for round_output_to_gui in rounded_output_to_gui {
                    for mut job in jobs() {
                        job.wrap = TextWrapping::wrap_at_width(max_width);

                        job.round_output_to_gui = round_output_to_gui;

                        let galley_wrapped =
                            layout(&mut fonts, pixels_per_point, job.clone().into());

                        job.wrap = TextWrapping::no_max_width();

                        let text = job.text.clone();
                        let galley_unwrapped = layout(&mut fonts, pixels_per_point, job.into());

                        let intrinsic_size = galley_wrapped.intrinsic_size();
                        let unwrapped_size = galley_unwrapped.size();

                        let difference = (intrinsic_size - unwrapped_size).length().abs();
                        similar_asserts::assert_eq!(
                            format!("{intrinsic_size:.4?}"),
                            format!("{unwrapped_size:.4?}"),
                            "Wrapped intrinsic size should almost match unwrapped size. Intrinsic: {intrinsic_size:.8?} vs unwrapped: {unwrapped_size:.8?}
                                Difference: {difference:.8?}
                                wrapped rows: {}, unwrapped rows: {}
                                pixels_per_point: {pixels_per_point}, text: {text:?}, max_width: {max_width}, round_output_to_gui: {round_output_to_gui}",
                            galley_wrapped.rows.len(),
                            galley_unwrapped.rows.len()
                            );
                        similar_asserts::assert_eq!(
                            format!("{intrinsic_size:.4?}"),
                            format!("{unwrapped_size:.4?}"),
                            "Unwrapped galley intrinsic size should exactly match its size. \
                                {:.8?} vs {:8?}",
                            galley_unwrapped.intrinsic_size(),
                            galley_unwrapped.size(),
                        );
                    }
                }
            }
        }
    }
}
